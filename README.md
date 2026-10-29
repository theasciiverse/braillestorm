# Braillestorm

**YOU ARE THE LAST PROMPT STANDING.**

Something has gone badly wrong in your terminal: globs tumble loose, block
cursors blink awake and hunt you down, forks split into two quick children
when you shoot them, spinners whirl in place and then dash straight at your
line, and the null device itself creeps after you, swallowing sparks, bending
your shots and dragging you in. You're a lone shell prompt with a twin stream of fire and
three bombs, in a twin-stick arena shooter drawn in Braille dots as thin
glowing lines, over a grid that dents, ripples and rings with every shot,
burst and bomb. Waves never stop, each bigger and faster than the last, and
from the tenth every fifth wave is a rush. Every kill without losing your
ship climbs the multiplier, up to x10, and now and then a shot-down enemy
leaves a pick-up behind: wide fire, rapid fire, a shield or a bomb. Every
five waves the grid turns a new colour. How long can you hold back the
storm?

Inspired by Mark Incitti's freeware GridWars (2005); no code, art or sound is
taken from it.

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install braillestorm
av play braillestorm
```

To build it yourself: `cargo run --release`.

## Controls

- **WASD**: move
- **Arrow keys**: fire; or hold the **left mouse button** to fire at the pointer
- **Space** or **right click**: bomb
- **P** or **Enter**: pause; **Space** or **Enter**: play again after game over
- **M**: sound on or off
- **Q** or **Esc**: quit
- **B**: a black hole on or off (it drifts, drinks the sparks and sucks the
  grid down), **H**: hide the top line
- **F2**: glow, **F3**: grid colour, **F4** / **F5**: grid denser or
  sparser, **F6**: random blasts, **F7**: autopilot
- **Gamepad**: left stick to move, right stick to fire, A, a shoulder button
  or a trigger to bomb, Start to pause, Back to quit

Best in a terminal that reports key releases (kitty, foot, WezTerm,
Ghostty, Alacritty, iTerm2 and others), or with a gamepad or the mouse.
Elsewhere the arrows fire sticky: the last arrow keeps firing until you
press the opposite one. `braillestorm --help` lists every key and option.

## Options

- `--demo`: watch the computer play
- `--mute`: start with the sound off
- `--wave N`: start at wave N
- `--fps N`: draw fewer frames a second (the game still runs at 60)

## Files

The high score is kept in `$XDG_DATA_HOME/braillestorm/hiscore`
(`~/.local/share/braillestorm` without it), `%APPDATA%\braillestorm` on
Windows. `RUST_LOG=debug braillestorm 2>braillestorm.log` writes a session
log.

## Requirements

A UTF-8 terminal with truecolour and a font with Braille (U+2800 to
U+28FF); 60x20 or bigger plays best, and the field grows with the window.
On Linux, sound needs libasound2 and gamepads libudev1.

## Building

```bash
cargo build --release
```

## Licence

GNU General Public License, version 3 or (at your option) any later
version; see [LICENSE](LICENSE).
