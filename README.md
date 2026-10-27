# Braillestorm

A twin-stick arena shooter for the terminal, drawn in Braille dots as thin
glowing lines over a grid that dents, ripples and rings with every shot,
burst and bomb. An original Asciiverse game, inspired by Mark Incitti's
freeware GridWars (2005); no code, art or sound is taken from it.

Fly a shell prompt through a storm of terminal things: globs tumble about
at random, block cursors blink and hunt you down, and forks split into two
quick children when you shoot them. They come in waves, each bigger and
faster than the last, announced in big glowing letters; clear the field to
bring on the next, with a bonus if you kept your ship. Every fifth wave
from the tenth is a rush. Three lives, an extra life and bomb every 25,000
points, and the best score kept between games.

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install braillestorm
av play braillestorm
```

Or download the archive for your system from this repository's Releases,
unpack it and run `./braillestorm` (`braillestorm.exe` on Windows).

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

Licence not chosen yet.
