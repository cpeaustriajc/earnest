#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: formula.sh <version> <release download url> <folder of .sha256 files>" >&2
  exit 2
fi
version=$1
download=$2
sums=$3

sum() {
  local file="$sums/earnest-$1.tar.gz.sha256"
  if [ ! -s "$file" ]; then
    echo "formula.sh: $file is missing or empty" >&2
    exit 1
  fi
  cut -d ' ' -f 1 "$file"
}

darwin_arm64=$(sum darwin-arm64)
darwin_x64=$(sum darwin-x64)
linux_arm64=$(sum linux-arm64-gnu)
linux_x64=$(sum linux-x64-gnu)

cat <<FORMULA
class Earnest < Formula
  desc "Model checker for models written in TypeScript or Rust"
  homepage "https://github.com/cpeaustriajc/earnest"
  version "$version"

  on_macos do
    on_arm do
      url "$download/earnest-darwin-arm64.tar.gz"
      sha256 "$darwin_arm64"
    end
    on_intel do
      url "$download/earnest-darwin-x64.tar.gz"
      sha256 "$darwin_x64"
    end
  end

  on_linux do
    on_arm do
      url "$download/earnest-linux-arm64-gnu.tar.gz"
      sha256 "$linux_arm64"
    end
    on_intel do
      url "$download/earnest-linux-x64-gnu.tar.gz"
      sha256 "$linux_x64"
    end
  end

  def install
    bin.install "earnest"
  end

  test do
    (testpath/"basket.model.ts").write <<~TS
      import { m } from "@earnest/model";

      const Item = m.enum(["Pen", "Ink"]);

      export const Basket = m.model("Basket", {
        items: m.set(Item),
      });

      export const add = m.action(Basket, "an item goes in", {
        input: { item: Item },
        when: (s, { item }) => !s.items.has(item),
        changes: (s, { item }) => ({ items: s.items.add(item) }),
      });

      export const remove = m.action(Basket, "an item comes out", {
        input: { item: Item },
        when: (s, { item }) => s.items.has(item),
        changes: (s, { item }) => ({ items: s.items.delete(item) }),
      });

      export const counted = m.rule(Basket, "The basket holds at most two items", (s) => s.items.size <= 2);
    TS
    assert_match "pass", shell_output("#{bin}/earnest verify --no-cache basket.model.ts")
  end
end
FORMULA
