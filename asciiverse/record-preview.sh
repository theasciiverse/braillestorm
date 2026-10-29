#!/bin/sh
# Record the 100x30 catalog preview, asciiverse/braillestorm.cast, with
# hub's castrec.py: the demo autopilot from wave 3, so the banners for
# waves 3, 4 (forks arrive) and 5 (corner cursors) show in 27 s, drawn at
# 20 fps to keep the file near 20 MB (the grid changes half the screen
# every frame), muted so recording plays nothing aloud. Seed 6 was the best of eight takes (2026-10-20); the still
# is at 12.2 s, the wave 4 banner with wave 3's clear bonus.
set -e
cd "$(dirname "$0")/.."
here=$(pwd)
castrec=$(cd ../../hub/tools && pwd)/castrec.py
cargo build --release
tmp=$(mktemp -d)
cp target/release/braillestorm "$tmp/"
(cd "$tmp" && PATH="$tmp:$PATH" HOME="$tmp" XDG_DATA_HOME="$tmp/data" \
  python3 "$castrec" "$here/asciiverse/braillestorm.cast" . "w27;kq;w0.5" \
  -- braillestorm --demo --mute --seed "${1:-6}" --fps 20 --wave 3)
rm -rf "$tmp"
