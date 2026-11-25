import json
import subprocess
import sys
import time

RATE_LIMIT_WAIT_SECONDS = 600
RATE_LIMIT_TRIES = 12
ALREADY_PUBLISHED = ("already exists", "already uploaded")


def publishable_crates():
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    crates = {
        package["name"]: package
        for package in metadata["packages"]
        if package.get("publish") != []
    }
    order = []
    placed = set()

    def place(name):
        if name in placed:
            return
        placed.add(name)
        for dependency in crates[name]["dependencies"]:
            if dependency["name"] in crates:
                place(dependency["name"])
        order.append(crates[name])

    for name in sorted(crates):
        place(name)
    return order


def publish(name):
    for _ in range(RATE_LIMIT_TRIES):
        result = subprocess.run(
            ["cargo", "publish", "--locked", "-p", name],
            capture_output=True,
            text=True,
        )
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode == 0:
            return
        if any(sign in result.stderr for sign in ALREADY_PUBLISHED):
            print(f"{name} is on crates.io already", flush=True)
            return
        if "429" not in result.stderr and "too many" not in result.stderr:
            sys.exit(f"cargo publish -p {name} failed")
        print(f"crates.io limits new crates; waiting {RATE_LIMIT_WAIT_SECONDS} s", flush=True)
        time.sleep(RATE_LIMIT_WAIT_SECONDS)
    sys.exit(f"crates.io still refused {name} after {RATE_LIMIT_TRIES} tries")


def main():
    dry_run = sys.argv[1:] == ["--dry-run"]
    for crate in publishable_crates():
        name, version = crate["name"], crate["version"]
        if dry_run:
            print(f"{name} {version}")
        else:
            publish(name)


main()
