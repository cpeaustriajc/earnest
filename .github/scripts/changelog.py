import argparse
import datetime
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

EARNEST_EPOCH_PARENT = "08b6db3954b7b6ca33ca69029cc6525410e13175"
DEFAULT_CONFIG = "cliff.toml"
DEFAULT_OUTPUT = "CHANGELOG.md"
DATE_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def run_command(command):
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        message = f"command failed ({' '.join(command)}): {detail}" if detail else f"command failed ({' '.join(command)})"
        sys.exit(message)
    return result.stdout


def git_repo_root():
    return pathlib.Path(run_command(["git", "rev-parse", "--show-toplevel"]).strip())


def git_commit_sha(rev):
    if not rev or not rev.strip():
        return None
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    sha = result.stdout.strip()
    return sha or None


def require_commit(rev, flag_name):
    sha = git_commit_sha(rev)
    if sha is None:
        sys.exit(f"invalid git revision for {flag_name}: {rev}")
    return sha


def is_ancestor(ancestor, descendant):
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def commit_distance(from_sha, to_sha):
    return int(run_command(["git", "rev-list", "--first-parent", "--count", f"{from_sha}..{to_sha}"]).strip())


def pick_closer_sha(preferred_sha, other_sha, to_sha):
    if preferred_sha == other_sha:
        return preferred_sha
    if is_ancestor(other_sha, preferred_sha):
        return preferred_sha
    if is_ancestor(preferred_sha, other_sha):
        return other_sha
    if commit_distance(other_sha, to_sha) < commit_distance(preferred_sha, to_sha):
        return other_sha
    return preferred_sha


def latest_semver_tag_sha(to_sha):
    output = run_command(
        [
            "git",
            "-c",
            "versionsort.suffix=-",
            "tag",
            "--merged",
            to_sha,
            "-l",
            "v[0-9]*.[0-9]*.[0-9]*",
            "--sort=-v:refname",
        ]
    )
    best_sha = None
    seen = set()
    for tag in output.splitlines():
        if "-nightly" in tag:
            continue
        sha = git_commit_sha(f"refs/tags/{tag}")
        if sha is None or sha in seen:
            continue
        seen.add(sha)
        if best_sha is None:
            best_sha = sha
        else:
            best_sha = pick_closer_sha(best_sha, sha, to_sha)
    return best_sha


def latest_nightly_tag_sha(to_sha):
    output = run_command(
        ["git", "tag", "--merged", to_sha, "-l", "v*-nightly*", "--sort=-v:refname", "--sort=-creatordate"]
    )
    best_sha = None
    seen = set()
    for tag in output.splitlines():
        sha = git_commit_sha(f"refs/tags/{tag}")
        if sha is None or sha in seen:
            continue
        seen.add(sha)
        if best_sha is None:
            best_sha = sha
        else:
            best_sha = pick_closer_sha(best_sha, sha, to_sha)
    return best_sha


def resolve_default_from(tag, to_sha):
    if tag is not None:
        exact_tag_sha = git_commit_sha(f"refs/tags/{tag}")
        if exact_tag_sha is not None and is_ancestor(exact_tag_sha, to_sha):
            return exact_tag_sha
    semver_sha = latest_semver_tag_sha(to_sha)
    nightly_sha = latest_nightly_tag_sha(to_sha)
    if semver_sha is not None and nightly_sha is not None:
        return pick_closer_sha(semver_sha, nightly_sha, to_sha)
    if semver_sha is not None:
        return semver_sha
    if nightly_sha is not None:
        return nightly_sha
    return git_commit_sha(EARNEST_EPOCH_PARENT)


def parse_date_timestamp(date_value):
    if not DATE_PATTERN.match(date_value):
        sys.exit(f"invalid --date (expected YYYY-MM-DD): {date_value}")
    try:
        parsed = datetime.datetime.strptime(date_value, "%Y-%m-%d").replace(tzinfo=datetime.timezone.utc)
    except ValueError:
        sys.exit(f"invalid --date (expected YYYY-MM-DD): {date_value}")
    return int(parsed.timestamp())


def write_empty_notes(notes_path):
    if notes_path is not None:
        pathlib.Path(notes_path).write_text("", encoding="utf-8")


def parse_args(argv):
    parser = argparse.ArgumentParser(prog="changelog.py")
    parser.add_argument("--from", dest="from_rev", default=None)
    parser.add_argument("--to", dest="to_rev", default="HEAD")
    parser.add_argument("--tag", default=None)
    parser.add_argument("--date", default=None)
    parser.add_argument("--rebuild", action="store_true")
    parser.add_argument("--prepend", action="store_true")
    parser.add_argument("--output", default=None)
    parser.add_argument("--notes", default=None)
    parser.add_argument("--config", default=None)
    return parser.parse_args(argv)


def main():
    args = parse_args(sys.argv[1:])
    if args.rebuild and args.prepend:
        sys.exit("--rebuild and --prepend cannot be used together")
    if args.tag is not None and (not args.tag.strip() or args.tag != args.tag.strip()):
        sys.exit("--tag must not be empty or have leading/trailing whitespace")
    if args.config is not None and not args.config.strip():
        sys.exit("invalid --config: path cannot be empty")
    if args.output is not None and not args.output.strip():
        sys.exit("invalid --output: path cannot be empty")
    if args.notes is not None and not args.notes.strip():
        sys.exit("invalid --notes: path cannot be empty")
    if shutil.which("git-cliff") is None:
        sys.exit("git-cliff is not installed or not on PATH")

    repo_root = git_repo_root()
    config_path = pathlib.Path(args.config) if args.config is not None else repo_root / DEFAULT_CONFIG
    if not config_path.is_file():
        sys.exit(f"cliff config not found: {config_path}")

    date_timestamp = parse_date_timestamp(args.date) if args.date is not None else None
    to_sha = require_commit(args.to_rev, "--to")

    if args.from_rev is not None:
        from_sha = require_commit(args.from_rev, "--from")
    elif args.rebuild:
        from_sha = git_commit_sha(EARNEST_EPOCH_PARENT)
    else:
        from_sha = resolve_default_from(args.tag, to_sha)

    if from_sha is None and to_sha != git_commit_sha("HEAD"):
        sys.exit("--from is required when --to differs from HEAD and no base tag or epoch exists")

    if from_sha is not None and from_sha == to_sha:
        write_empty_notes(args.notes)
        return

    rev_range = f"{from_sha}..{to_sha}" if from_sha else to_sha
    first_parent_shas = set(run_command(["git", "rev-list", "--first-parent", rev_range]).splitlines())
    if not first_parent_shas:
        write_empty_notes(args.notes)
        return

    resolved_config = str(config_path.resolve())
    context_cmd = [
        "git-cliff",
        "--config",
        resolved_config,
        "--repository",
        str(repo_root),
        "--offline",
        "--no-exec",
        "--context",
    ]
    if args.tag is not None:
        context_cmd.extend(["--tag", args.tag])
    if from_sha is not None:
        context_cmd.append(f"{from_sha}..{to_sha}")

    raw_context = run_command(context_cmd)
    releases = json.loads(raw_context)
    filtered_releases = []
    for release in releases:
        commits = []
        for commit in release.get("commits", []):
            if commit.get("id") not in first_parent_shas:
                continue
            item = dict(commit)
            extra = dict(item.get("extra") or {})
            extra["preprocessed"] = item.get("message", "")
            item["extra"] = extra
            commits.append(item)
        if commits:
            updated = dict(release)
            updated["commits"] = commits
            filtered_releases.append(updated)

    if not filtered_releases:
        write_empty_notes(args.notes)
        return

    if date_timestamp is not None:
        if args.tag is not None:
            for release in filtered_releases:
                if release.get("version") == args.tag:
                    release["timestamp"] = date_timestamp
        else:
            filtered_releases[0]["timestamp"] = date_timestamp

    with tempfile.TemporaryDirectory() as temp_dir:
        context_file = pathlib.Path(temp_dir) / "context.json"
        context_file.write_text(json.dumps(filtered_releases), encoding="utf-8")
        base_cmd = [
            "git-cliff",
            "--config",
            resolved_config,
            "--offline",
            "--no-exec",
            "--from-context",
            str(context_file),
        ]

        stripped_body = run_command([*base_cmd, "--strip", "all"])
        notes_text = stripped_body.lstrip("\n")
        if args.notes is not None:
            pathlib.Path(args.notes).write_text(notes_text, encoding="utf-8")

        if not notes_text.strip():
            return

        output_path = pathlib.Path(args.output) if args.output is not None else repo_root / DEFAULT_OUTPUT
        resolved_output = str(output_path.resolve())
        if args.rebuild:
            run_command([*base_cmd, "--output", resolved_output])
        elif args.prepend:
            if output_path.exists():
                run_command([*base_cmd, "-u", "--prepend", resolved_output])
            else:
                run_command([*base_cmd, "--output", resolved_output])
        elif args.notes is None:
            rendered = run_command(base_cmd)
            sys.stdout.write(rendered)


main()
