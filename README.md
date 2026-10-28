# Braillestorm

**YOU ARE THE LAST PROMPT STANDING.**

Something has gone badly wrong in your terminal: globs tumble loose, block
cursors blink awake and hunt you down, and forks split into two quick children
when you shoot them. You're a lone shell prompt with a twin stream of fire and
three bombs, in a twin-stick arena shooter drawn in Braille dots as thin
glowing lines, over a grid that dents, ripples and rings with every shot,
burst and bomb. Waves never stop, each bigger and faster than the last, and
from the tenth every fifth wave is a rush. How long can you hold back the
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
- **Q** or **Esc**: quit
- **Gamepad**: left stick to move, right stick to fire, A or a shoulder
  button to bomb, Start to pause, Back to quit

Best in a terminal that reports key releases (kitty, foot, WezTerm,
Ghostty, Alacritty, iTerm2 and others), or with a gamepad or the mouse.
Elsewhere the arrows fire sticky: the last arrow keeps firing until you
press the opposite one. `braillestorm --help` lists every key and option.

## Options

- `--demo`: watch the computer play
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
On Linux, gamepads need libudev1.

## Building

```bash
cargo build --release
```

## Licence

GNU General Public License, version 3 or (at your option) any later
version; see [LICENSE](LICENSE).
