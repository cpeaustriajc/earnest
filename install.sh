#!/bin/sh
set -eu

DEFAULT_DOWNLOAD_URL="https://github.com/cpeaustriajc/earnest/releases/download/nightly"

fail() {
  echo "earnest install: $*" >&2
  exit 1
}

platform() {
  os=$(uname -s)
  machine=$(uname -m)
  case "$machine" in
    x86_64 | amd64) arch=x64 ;;
    arm64 | aarch64) arch=arm64 ;;
    *) fail "no nightly build for the cpu $machine" ;;
  esac
  case "$os" in
    Darwin) echo "darwin-$arch" ;;
    Linux)
      if ldd --version 2>&1 | grep -qi musl; then
        fail "no nightly build for Linux with musl; only glibc is built"
      fi
      echo "linux-$arch-gnu"
      ;;
    *) fail "no nightly build for $os; on Windows use install.ps1" ;;
  esac
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d ' ' -f 1
  else
    fail "found neither sha256sum nor shasum to check the download"
  fi
}

main() {
  command -v curl >/dev/null 2>&1 || fail "curl is needed to download earnest"
  download_url=${EARNEST_DOWNLOAD_URL:-$DEFAULT_DOWNLOAD_URL}
  install_dir=${EARNEST_INSTALL_DIR:-$HOME/.local/bin}
  archive="earnest-$(platform).tar.gz"

  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT

  echo "Downloading $download_url/$archive"
  curl --fail --silent --show-error --location --output "$work/$archive" "$download_url/$archive"
  curl --fail --silent --show-error --location --output "$work/$archive.sha256" "$download_url/$archive.sha256"

  expected=$(cut -d ' ' -f 1 "$work/$archive.sha256")
  actual=$(sha256 "$work/$archive")
  [ -n "$expected" ] || fail "$archive.sha256 is empty"
  [ "$expected" = "$actual" ] || fail "$archive has SHA-256 $actual, but $archive.sha256 says $expected"

  tar -xzf "$work/$archive" -C "$work" earnest
  mkdir -p "$install_dir"
  cp "$work/earnest" "$install_dir/.earnest.new"
  chmod 755 "$install_dir/.earnest.new"
  mv -f "$install_dir/.earnest.new" "$install_dir/earnest"

  echo "Installed $("$install_dir/earnest" --version) in $install_dir"
  case ":$PATH:" in
    *":$install_dir:"*) ;;
    *) echo "Add $install_dir to PATH to run earnest by name." ;;
  esac
}

main "$@"
