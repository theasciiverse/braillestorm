#!/bin/sh
# Record the 100x30 catalog preview, asciiverse/braillestorm.cast, with
# hub's castrec.py: the demo autopilot, 18 s into the game so all three
# enemy kinds show in 25 s, drawn at 20 fps to keep the file near 24 MB
# (the grid changes half the screen every frame). Seed 7 was the best of
# eight takes (2026-10-20); the still is at 23 s.
set -e
cd "$(dirname "$0")/.."
here=$(pwd)
castrec=$(cd ../../hub/tools && pwd)/castrec.py
cargo build --release
tmp=$(mktemp -d)
cp target/release/braillestorm "$tmp/"
(cd "$tmp" && PATH="$tmp:$PATH" HOME="$tmp" XDG_DATA_HOME="$tmp/data" \
  python3 "$castrec" "$here/asciiverse/braillestorm.cast" . "w25;kq;w0.5" \
  -- braillestorm --demo --seed "${1:-7}" --fps 20 --time 18)
rm -rf "$tmp"
