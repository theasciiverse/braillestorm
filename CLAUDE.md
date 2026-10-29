# Braillestorm

Asciiverse twin-stick arena shooter. Rust + Ratatui, drawn in Braille (Drawille-style
sub-cells) so the ships, enemies and the warping grid read as glowing vector lines.
Brought to the terminal from Mark Incitti's GridWars (2005 to 2006, BlitzMax),
itself a free tribute to Bizarre Creations' Geometry Wars (2003 to 2005).

**Status** (2026-10-29, version 0.4.0): playable in waves, with sound. The
ship (a shell prompt `❯` as a chevron) flies over the warping grid, fires a
twin stream and bombs, against five enemy kinds of our own (below) in
numbered waves, each announced by a banner of big Braille letters; a score
multiplier up to x10; pick-ups; stages that change the grid's colour; three lives, an extra life and bomb every 25,000
points before the multiplier, game over and play again, pause, high score
in `$XDG_DATA_HOME/braillestorm/hiscore`. Synthesised sound, `m` or
`--mute` to silence it. A demo autopilot flies, dodges and shoots. Route
decided by David on 2026-10-20: a spiritual successor of our own (below);
everything else here is a first proposal. What comes next, release by
release, is in `ROADMAP.md`.

In `hub/catalog` since 2026-10-29 (`braillestorm.toml` and `.md`), on the
website mockup since 2026-10-20 with the NEW! banner; licence
GPL-3.0-or-later (David, 2026-10-27); preview
`asciiverse/braillestorm.cast` copied to the site's `casts/`, local Linux
build packaged by the site's `dev/build_catalog.py` into `dev-releases/`.
Own git repo, branch `main`, private at github.com/theasciiverse/braillestorm.
Release workflow `.github/workflows/release.yml` (2026-10-20, on ASCII
Wars' pattern): glibc Linux on Ubuntu 22.04 (x86_64, aarch64; libasound
and libudev linked dynamically, the manifest's system_deps), macOS and Windows (MSVC),
x86_64 and ARM64 each; archives `braillestorm-<target>` with the stripped
binary, README.md and LICENSE, plus SHA256SUMS.
Tags `v<version>`, now `v0.4.0` (2026-10-29). Only the Linux steps were
run here (no rustup for other targets), so macOS and Windows build first
on GitHub.

The preview (`asciiverse/record-preview.sh`, hub's `castrec.py`): `--demo
--seed 6 --fps 20 --wave 3 --mute`, 27 s with the banners for waves 3, 4 and 5,
still at 12.2 s (the wave 4 banner), best of eight takes (the demo is
deterministic per seed, so `--shot 27 --wave 3 --seed N` with
`RUST_LOG=info` shows where each take's waves begin). It is 20 MB even so: the warping grid changes about half the screen every
frame and 70% of the bytes are truecolour escapes, so a lower frame rate
barely helps (36 s at 20 fps was 33 MB, at 30 fps 45 s was 52 MB). The demo
skips the kitty keyboard query, which otherwise left 2 s of black at the
start of a recording (a recorder never answers it).

```bash
cargo run --release                          # play (see --help for every key)
cargo run --release -- --demo                # the autopilot flies
cargo run --release -- --mute                # no sound (m toggles it in play)
cargo run --release -- --wave 10            # start at wave 10 (the first rush)
cargo run --release -- --fps 20              # draw less often (the game still runs at 60)
asciiverse/record-preview.sh                 # re-record the catalog preview
RUST_LOG=debug cargo run --release 2>braillestorm.log
cargo test
cargo run --release -- --bench 600 --size 400x110      # frame cost without a terminal
RUST_LOG=info cargo run --release -- --bench 36000 --size 100x30 --seed 1 2>&1 | grep -E 'lost|game over'   # autopilot survival
cargo run --release -- --shot 6 --size 100x30 --seed 4 --cells | python3 tools/shot.py out.png
```

Controls: WASD or the left stick move; arrows, the left mouse button held
(fires at the pointer) or the right stick fire; Space, right click or a
pad's A or shoulder buttons bomb (3); p or Enter pause (a pad's Start;
Back quits), Space or Enter play again after game over; m sound on or
off; losing focus pauses. Sandbox keys from the grid test: b
black hole, h top line, F2 glow, F3 grid colour, F4/F5 spacing, F6 random
blasts, F7 autopilot. With the kitty keyboard protocol keys are exact;
without it WASD is held while repeats arrive (stutters briefly after the
first press) and firing is sticky: an arrow sets its axis, two arrows in a
row make a diagonal, the opposite arrow stops.

Code: `src/canvas.rs` (light per Braille dot, added up where things overlap,
resolved per cell: dots above a threshold, the brightest dot's colour, and a
blurred dim copy of the light as the cell background for glow),
`src/grid.rs` (points with springs to their neighbours and back to rest,
border pinned; blast and pull), `src/fx.rs` (sparks, black holes, bomb
rings), `src/ship.rs` (ship, shots, bombs, shield after coming back),
`src/enemies.rs` (the roster, the waves, and what happened each frame as
`Events`), `src/audio.rs` (every sound synthesised at start-up, after
Brickspire's; rodio, ALSA on Linux), `src/text.rs` (our 5x7 pixel
font drawn as dots, for the banners), `src/save.rs`
(high score), `src/input.rs` (keys, mouse, gamepad to move/fire/bomb, after
Panzer's), `src/world.rs` (the field, score and multiplier, lives, wave banners and
clear bonus, stages, game over, the nulls' pull, the sounds asked for,
sandbox switches, demo autopilot), `src/pickups.rs` (drops, paced by time),
`src/rng.rs`, `src/main.rs` (the loop at 60 fps, headless `--shot` and
`--bench`), `tools/shot.py` (cells to PNG, Braille dots drawn as dots).

Measured 2026-10-20: 60 fps steady in tmux at 100x30, 240x70 and 400x110;
building and writing a frame takes about 0.9, 3.5 and 7.8 ms. David ran the
grid test the same day in his own terminal (173x58, 60 fps, 2 ms a frame)
and approved the look as it is: thin dotted Braille lines, a dim grid,
faint glow. Keep it; don't thicken lines or saturate sparks without asking.
Checked in tmux (no key releases): WASD, sticky and diagonal fire, bomb,
fire off; gamepad detected (an Xbox 360 pad on David's machine). Not yet
tried by hand: mouse firing, gamepad sticks, kitty-protocol keys.

## The source

- **GridWars** by Mark Incitti, Canadian; first released as freeware on 21 December
  2005 on his site (incitti.com/Blitz). Bizarre Creations asked him to take it
  down; the download was pulled on 8 August 2006. Incitti said he had been in
  talks with Bizarre for months and his offers to change the look and the name
  were ignored. It was later rehosted elsewhere for Windows, macOS and Linux.
  A GridWars 2 followed. (Wikipedia; Engadget, 2006-08-17.)
- **Source**: Incitti shipped the BlitzMax source with the game (his site, 2006:
  "Some of them come with source code too", "BlitzMax source code included").
  About 13,800 lines: `gridwars.bmx` (6,045: enemies, player, spawning, menus),
  `gridparttrail.bmx` (the spring grid, particles, trails), `control.bmx`
  (input), `sound.bmx`, `vectorfont.bmx`. Music is three `.it` modules by Swith;
  the graphics sets are by taumel and svrman, credited in the Readme.
- **No licence.** Checked 2026-10-20: no licence or permission text in the
  source, the Readme, his archived site (Wayback, 2006) or the maintained repo
  (github.com/mmatyas/GridWars: Raspberry Pi and BMX-NG builds by Mátyás
  Mustoha 2016 and 2025, Linux fixes by Aleksandr Beliaev 2021; 18 forks, some
  active in October 2026, including a PortMaster build). Source shipped for
  reading is not a licence to translate and redistribute it.
- **Local copies**: `../../references/Gridwars/` (same files as
  terra:/pool/games/source/Gridwars, sha256-identical): `GridWars.zip` (the
  mmatyas git clone), `GridWars54.zip` (Incitti's own source release),
  the Linux, macOS and Windows binaries, Incitti's `T3MP3ST` (Tempest) and
  Vector Zone (Battlezone) sources, and unrelated clones: ShapeWars
  (`Geometry-Wars-master.zip`, George Kudrayvtsev 2012, zlib licence), a Java
  remake (`source-archive.zip`), a C++ GL clone, a German console game
  (`source_code.zip`, Boulder Dash-like, not a shooter) and `vga_gamespack`
  (svgalib board games, unrelated).

## Route: a successor of our own (decided 2026-10-20)

Mechanics are free: a twin-stick arena shooter with a warping grid is a
genre, not anyone's property, and in Braille the graphics are entirely
different anyway. So Braillestorm is our own code from scratch, with our own
enemy roster, names, art and synthesised sound, tuned by playing GridWars and
reading how it behaves, not by translating its source. No GridWars code,
numbers tables, music (Swith) or graphics (taumel, svrman) go in. No letter
to Incitti needed; a porting request was the alternative, not taken.

Keep clear of Geometry Wars' trade dress (its name, logo and exact enemy
designs); the store page can say "twin-stick arena shooter", without naming
it.

**Name: Braillestorm** (id `braillestorm`; chosen by David 2026-10-20). The
working name Vectorwars was taken: *VectorWars VR* on Steam (app 600350, Red
Iron Labs, 2017), a top-down arcade space shooter, still sold; *Vector Wars*
on itch.io several times over; *Vektor Wars* on Steam (2015). Asciiwars, the
other favourite, is taken too (*ASCII Wars* on Steam and itch.io, a
bullet-hell boss rush). Braillestorm checked 2026-10-20: nothing on Steam or
itch.io (as one word or two), no GitHub repo, crate name free on crates.io,
no game or product in a web search (only a 2017 forum username);
braillestorm.com, .net, .org and .io have no DNS (probably free, no
registrar checked). USPTO and EUIPO not searched.

## What GridWars has (the reference, from its source)

- **Enemies** (its "Know Your Enemies" screen, points in brackets): Sammy
  the Seeker (10), Paul the Pinwheel (25), Dimmy the Diamond (50), Cubie the
  Cube (50/100), Shy the Square (100), Selena the Snake (100), Ivan the
  Interceptor (100), Trish the Triangle (150), Dwight the Black Hole (150),
  Indy the Butterfly (10); plus generators. Each one's behaviour is still to
  be read from its `Update` method in `gridwars.bmx`.
- **Pick-ups**: temporary back shooter, temporary side shooters, extra front
  shooter, extra shot speed, extra player; bombs (what they do exactly not read yet).
- **The grid**: a mesh of points under the play field (`gridpoint` in
  `gridparttrail.bmx`): each is nudged toward a target, stepped back toward
  its rest position and damped (0.899 a frame), so shots, explosions and
  black holes leave ripples; 20 grid types (F3 cycles them). The grid is
  the game's look.
- **Controls**: dual analogue, mouse, joypad, keyboard (WASD move, arrows fire,
  Space bomb) and a mouse-and-keys hybrid; inertia setting; cheat keys.
- **Scoring**: points per enemy and a hi score table; anything more (such
  as a multiplier) not checked yet.

## The terminal version (proposal)

- **Look**: Braille sub-cells (2 by 4 dots per cell) for everything: the ship,
  the enemies as line figures, shots, sparks and the grid. Truecolour on black,
  with "glow" from brightness falloff on lines and particles. Braille holds one
  colour per cell, so where colours meet the brighter one wins; keep the grid
  dim so ships and enemies stay readable over it.
- **Play field**: a fixed world larger than the screen, scrolling with the
  ship, or fit to the window; to try both. Must play at 100x30 (the preview
  size) and use a big terminal fully.
- **Input**: twin-stick needs two directions held at once. Gamepad first
  (as Ceres and Brickspire do). Keyboard needs key-release events, which only
  the kitty keyboard protocol reports (Ceres has this); without it, fall back
  to a fire-direction lock (the last arrow keeps firing until another or a
  stop key). Mouse aim through terminal motion reporting.
- **Sound**: synthesised, our own (no GridWars samples or Swith's modules).
- **Modes**: the classic endless survival first; more later if it earns them.
- **Asciiverse conventions**: session logging (`log` + `env_logger`), hi
  score in the XDG data folder, headless screenshot mode for previews, a
  `--demo` autopilot for the 100x30 `.cast`.

## First steps

1. Braille renderer: lines, points, colour per cell; frame-rate check in a big
   terminal. Done (grid test).
2. The grid on its own: dents, waves, black-hole pull, at 60 fps. Done (grid
   test); look and feel to be tuned by playing.
3. Ship, shots and input (gamepad, kitty keyboard, fallback, mouse). Done,
   with practice targets; feel to be tuned by playing.
4. Our first three enemy kinds and spawning, then a playable loop with score
   and lives. Done; balance to be tuned by playing.
5. Named waves with a banner and a breather between them. Done (0.2.0).
6. Sound, a score multiplier and two more enemy kinds (spinner, null).
   Done (0.3.0).
7. Pick-ups and stages. Done (0.4.0). What follows is in `ROADMAP.md`.

## Enemies (our own, 2026-10-20)

Terminal things, to go with the prompt ship. Each fades in (blinking,
shrinking to size) for 45 frames where it spawns and can neither hurt nor be
hurt meanwhile; new ones keep clear of the ship.

- **Glob** `*` (violet, 25): six-armed asterisk tumbling on a random walk.
  From the start.
- **Cursor** (cyan, 50): hollow block cursor, twice as tall as wide, its
  fill blinking; homes in, top speed rising the longer it chases. From
  wave 2; from wave 5 also out of all four corners (more per corner every
  four waves).
- **Fork** `Y` (pink, 100): turns slowly toward the ship; shot, it throws
  two **children** (orange diamonds, 25) out either side of the shot's line,
  fast homers that overshoot. A bomb kills a fork whole. From wave 4.
- **Spinner** `|/-\` (lime, 75; 2026-10-29): the busy-terminal spinner, a
  bar turning in eighth turns. It spins in place for 50 frames, then dashes
  (2.6 dots a frame, at most 40 frames) along whichever axis lines it up
  with the ship, stopping when level with it or at a wall. From wave 6,
  about one spawn in six.
- **Null** `∅` (warm white, 250; 2026-10-29): the null device, a ring with
  the glyph's leaning stroke and a faint halo. Creeps after the ship, pulls
  the grid down within 70 dots, drinks sparks, bends the ship's shots and
  tugs the ship itself; takes 25 hits, flinching at each (40 made the
  autopilot's games end ten waves early); a bomb kills it whole. Dies with a
  ring and a big burst. From wave 8, one at a time (two from wave 16), a
  spawn roll above 0.96. Our own take on the gravity-well idea the genre
  shares: it never spawns anything and does not grow.

## Waves (David's pick, 2026-10-20)

Wave N sends `(8 + 5N) * size` enemies (half as many again in a rush), one
spawn every `(60 - 6N).max(8) / size` frames, at most `(12 + 4N) * size` on
the field at once, where `size` is the field's area against 100x30 (square
root, 1 to 4). Glob packs from wave 3, growing every four waves. Everything
moves 5% faster each wave, up to 1.6 times. Every fifth wave from the 10th
is a **rush**: half the spawns are cursors out of every corner. Once a wave
has sent everything, the next starts when the field is clear, or after
15 s if some are left wandering; then a 2.5 s **breather** with no spawns.
Clearing a wave without losing the ship pays `100 * N`.

**Multiplier** (2026-10-29): kills, by shot or bomb, without losing the
ship raise it one step after `25 * current` kills (x2 after 25, x3 after
50 more, ...), up to x10; losing the ship drops it to x1. It shows after the
score in the top line. Kill points are multiplied, the clear bonus is not,
and extra lives count points before the multiplier: counting the
multiplied score gave the autopilot about 150 extra lives and bombs a game. Losing the ship
bursts every enemy (no points); it returns after 100 frames with a 2 s
blinking shield.

**Banner**, at each wave's start, during the breather: `WAVE N` in big
Braille letters and, under it at half the size, only `rush!` on a rush
wave and the clear bonus (`+300`) (David: little text, so it does not take
the eye off the action; no "new: fork" notes, the enemies explain
themselves, while a rush warning changes what to do next). No brackets
round the headline either: blown up in pixel letters a lone `]` read as a
stray shape (David, 2026-10-20). Our own
5x7 font, drawn as light so it glows; no box and no dark backdrop (David:
the graphics are too good for a line box, use oversized Braille text in a
colour contrasting with the game, and keep the play area visible under
it). Only the cells the letters' dots fall in are cleared, since a cell has
one colour and grid dots there would blur the letters; the grid shows
everywhere else, between the strokes too. It types out over
40 frames with a block cursor, holds, and fades over the last 25 of 150.
Colour by grid colour (F3): amber on blue, magenta on green, cyan on
amber, lime on violet. Headline up to six times the font, about a fifth of
the field's height; the rest at half the headline's scale; lines too wide
for the window are left out.

## Pick-ups and stages (2026-10-29, 0.4.0)

A pick-up is a letter of the banner font in a pulsing box, left where an
enemy was shot down (not bombed), waiting 8 s (blinking the last 2) for the
ship to fly through it; at most two on the field. **W** wide: a third
stream either side, 0.2 rad off the aim; **R** rapid: a volley every 3
frames instead of 5; both last 6 s, shown with seconds left in the top line,
and end when the ship is lost. **S** shield: 3 s untouchable (the respawn
blink). **B** bomb: one more. Odds W 3, R 3, S 2, B 2 in ten.

Drops are paced by time, not kills: nothing for 20 s after the last drop,
then each kill's chance climbs to 0.15 over 30 s; a null drops half the
time once 8 s have passed. Per-kill odds snowballed (more firepower, more
kills, more drops): the autopilot outlived ten minutes on two of eight
seeds. With 10 s powers the time pacing still let three of eight survive;
6 s powers and a 3 s shield end all eight at waves 31 to 49 (0.3.0: 22 to
37), with 12 to 25 drops a game. The autopilot detours for a pick-up when
nothing is within 30 dots of it.

**Stages**: every five waves the grid (and the banner with it) moves on to
the next of the four colours; F3 picks where the cycle starts.

## Sound (2026-10-29)

All synthesised in `src/audio.rs` at start-up, no files, on Brickspire's
pattern; nothing of GridWars'. A soft zip for each volley (kept very low,
it plays a dozen times a second); a burst of its own for every kind (glob
pop, cursor blip stepped like bits going blank, fork crack, child chirp,
spinner winding down, null implosion); a ping up a pentatonic scale under
each burst once the multiplier is past x1; a knock when a shot dents a
null; a faint shimmer when something fades in; bomb crack and boom; the
ship lost; a rising shimmer when it is back; a bright run for a pick-up
and a softer fall when wide or rapid runs out; two notes for a wave, three
warning pulses for a rush; the multiplier going up, an extra life, game
over. The same effect again within 40 ms is skipped (60 for volleys, 140
for arrivals), so a bomb clearing forty enemies is one bang. Peaks checked
by a test (all under 1.0; the bomb and the ship lost are the loudest at
about 0.8). Without an audio device the game is silent and says "no sound"
in the top line; `m` or `--mute` silences it. Not yet heard by David.

Measured 2026-10-29 with the new kinds and the multiplier (`--bench 36000`
at 100x30, seeds 1 to 5): the autopilot's games end at waves 22 to 37,
with two to seven bonus lives; every kind has taken a ship.

Measured 2026-10-20 with waves (`--bench 36000`, seeds 1 to 3): the
autopilot, which aims perfectly, reaches game over at waves 30 to 38 at
100x30 (5 to 7 minutes, score 122,000 to 197,000, helped by bonus lives);
at 240x70 it is still alive after ten minutes at wave 31. Before the waves
the same bot lasted about four and a half minutes at 100x30. Frame cost
with enemies: update 0.01 ms at 100x30, 0.06 ms at 240x70.
