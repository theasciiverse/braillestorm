//! Braillestorm: twin-stick arena shooter in Braille.
//!
//! The ship flies over the warping grid and fights off waves of enemies;
//! three lives, an extra one (and a bomb) every 25,000 points, high score
//! kept in the data folder; synthesised sound (`m` or `--mute` silences it).
//! `--demo` lets the autopilot fly. `--shot` plays
//! some seconds headless on the autopilot and prints the screen (`--cells`
//! for `tools/shot.py`); `--bench` times frames without a terminal.

mod audio;
mod canvas;
mod enemies;
mod fx;
mod grid;
mod input;
mod rng;
mod save;
mod ship;
mod text;
mod world;

use input::{Cmd, Controls, Input};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::io;
use std::time::{Duration, Instant};
use world::{World, GRID_COLOURS};

const FRAME: Duration = Duration::from_micros(16_667);
const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);

const HELP: &str = "braillestorm [--demo] [--mute] [--seed N] [--spacing N] [--fps N] [--wave N]

Twin-stick shooting over a warping grid, in Braille, in waves that grow:
clear the field to bring on the next, with a bonus if you kept your ship.
Three lives; an extra life and bomb every 25,000 points.

  WASD            move             (or the left stick)
  arrow keys      fire             (or hold the left mouse button to fire
                                    at the pointer, or the right stick)
  Space           bomb             (or right click, or a pad's A/shoulder)
  p / Enter       pause            (Enter or Space plays again after game over;
                                    a pad's Start pauses, Back quits)
  m               sound on/off
  q / Esc         quit

  b  black hole on/off   h  hide the top line
  F2 glow   F3 grid colour   F4/F5 grid denser/sparser
  F6 random blasts   F7 demo autopilot

Without the kitty keyboard protocol (kitty, foot, WezTerm, Ghostty,
Alacritty, iTerm2 and others report key releases) WASD stutters for a
moment after the first press, and arrows fire sticky: the last arrow keeps
firing until its opposite is pressed. The mouse and a gamepad work anywhere.

  --mute                               start with the sound off
  --wave N                             start at wave N
  --fps N                              draw N frames a second (60; 30 or 20 for smaller preview casts)
  --shot SECS [--size WxH] [--wave N] [--cells]   play headless on the autopilot, print the screen
  --bench FRAMES [--size WxH]          time frames without a terminal

Session log: RUST_LOG=debug braillestorm 2>braillestorm.log";

fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let arg = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(String::as_str);
    let seed = arg("--seed").and_then(|v| v.parse::<u64>().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
    });
    let spacing = arg("--spacing").and_then(|v| v.parse::<f32>().ok()).unwrap_or(8.0).clamp(4.0, 32.0);
    let size = arg("--size").and_then(|v| {
        let (w, h) = v.split_once('x')?;
        Some((w.parse::<u16>().ok()?.clamp(10, 1000), h.parse::<u16>().ok()?.clamp(5, 500)))
    });
    let wave = arg("--wave").and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).clamp(1, 999);
    if let Some(secs) = arg("--shot") {
        shot(secs.parse().unwrap_or(3.0), size.unwrap_or((100, 30)), seed, spacing, wave, args.iter().any(|a| a == "--cells"));
        return Ok(());
    }
    if let Some(frames) = arg("--bench") {
        bench(frames.parse().unwrap_or(600), size.unwrap_or((240, 70)), seed, spacing);
        return Ok(());
    }

    let mut terminal = ratatui::init();
    // the demo takes no keys but q, so it skips the query (a recorder never
    // answers it, and the query then waits two seconds)
    let demo = args.iter().any(|a| a == "--demo");
    let kitty = !demo && supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    execute!(io::stdout(), EnableMouseCapture)?;
    let _ = execute!(io::stdout(), EnableFocusChange);
    let mut input = Input::new(kitty);
    input.open_pad();
    let screen = terminal.get_frame().area();
    log::info!("start {}x{} kitty_keyboard={kitty} pad={:?} seed {seed} spacing {spacing}", screen.width, screen.height, input.pad_name);
    let mut world = World::new(screen, seed, spacing);
    world.demo = demo;
    world.keep_hi = true;
    world.hi = save::load_hi();
    world.start_wave = wave;
    world.restart();
    let fps = arg("--fps").and_then(|v| v.parse::<u32>().ok()).unwrap_or(60);
    let mut audio = audio::Audio::new(args.iter().any(|a| a == "--mute"));
    let result = run(&mut terminal, &mut world, &mut input, &mut audio, fps);
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableFocusChange);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    log::info!("quit after {} frames, score {}", world.frames, world.score);
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, world: &mut World, input: &mut Input, audio: &mut audio::Audio, draw_fps: u32) -> io::Result<()> {
    let mut screen = terminal.get_frame().area();
    let mut next = Instant::now();
    let mut snapshot = Instant::now();
    let (mut fps, mut counted, mut since) = (0.0f32, 0u32, Instant::now());
    let mut draw_ms = 0.0f32;
    // the game always runs at 60 updates a second; --fps only draws less often
    let every = (60 / draw_fps.clamp(1, 60)).max(1) as u64;
    let mut drawn = 0u64;
    loop {
        loop {
            let wait = next.saturating_duration_since(Instant::now());
            if !event::poll(wait)? {
                break;
            }
            match event::read()? {
                Event::Key(k) => {
                    log::debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
                    input.on_key(k, Instant::now());
                }
                Event::Mouse(m) => {
                    if !matches!(m.kind, event::MouseEventKind::Moved | event::MouseEventKind::Drag(_)) {
                        log::debug!("mouse {:?} at {},{}", m.kind, m.column, m.row);
                    }
                    input.on_mouse(m);
                }
                Event::FocusLost => {
                    log::debug!("focus lost: releasing all keys, pausing");
                    input.release_all();
                    if world.phase == world::Phase::Play && !world.demo {
                        world.paused = true;
                    }
                }
                Event::Resize(w, h) => {
                    screen = Rect::new(0, 0, w, h);
                    log::info!("resize {w}x{h}");
                    world.rebuild(screen);
                }
                _ => {}
            }
        }
        next += FRAME;
        let now = Instant::now();
        if next < now {
            next = now + FRAME;
        }

        input.poll_pad();
        for cmd in std::mem::take(&mut input.cmds) {
            log::debug!("cmd {cmd:?}");
            match cmd {
                Cmd::Quit | Cmd::Exit => return Ok(()),
                Cmd::Glow => world.glow = !world.glow,
                Cmd::Colour => world.colour = (world.colour + 1) % GRID_COLOURS.len(),
                Cmd::AutoBlasts => world.auto_blasts = !world.auto_blasts,
                Cmd::Demo => world.demo = !world.demo,
                Cmd::Start => world.start_or_pause(),
                Cmd::Pause => {
                    if matches!(world.phase, world::Phase::Over(_)) {
                        world.start_or_pause();
                    } else {
                        world.paused = !world.paused;
                    }
                }
                Cmd::Hole => world.toggle_hole(),
                Cmd::Mute => audio.toggle_mute(),
                Cmd::Hud => {
                    world.hud = !world.hud;
                    world.rebuild(screen);
                }
                Cmd::Denser | Cmd::Sparser => {
                    let step = if cmd == Cmd::Denser { -2.0 } else { 2.0 };
                    world.spacing = (world.spacing + step).clamp(4.0, 32.0);
                    world.rebuild(screen);
                }
            }
        }

        let pointer = input.mouse.and_then(|(c, r)| world.to_dots(c, r));
        let mut controls = input.controls(Instant::now(), (world.ship.x, world.ship.y), pointer);
        if world.demo {
            controls = world.demo_controls();
        }
        world.update(controls);
        for &s in &world.sounds {
            audio.play(s);
        }

        let keys = if input.exact { "keys exact" } else { "keys repeat" };
        let pad = input.pad_name.as_deref().map_or(String::new(), |n| format!("  pad {n}"));
        // the demo shows no aim, pad or timings, so a preview cast stays clean
        let sound = if !audio.available() { "  no sound" } else if audio.muted { "  sound off" } else { "" };
        let status = if world.demo { "DEMO".to_string() } else { format!("fire by {}  {keys}{pad}{sound}  {fps:>3.0} fps {draw_ms:>4.1} ms  {}x{}", input.aim_name(), screen.width, screen.height) };
        drawn += 1;
        if drawn % every == 0 {
            let t = Instant::now();
            terminal.draw(|f| {
                let area = f.area();
                if area != screen {
                    screen = area;
                    world.rebuild(screen);
                }
                world.draw(f.buffer_mut(), area, &status);
            })?;
            draw_ms = draw_ms * 0.9 + t.elapsed().as_secs_f32() * 1000.0 * 0.1;
        }

        counted += 1;
        if since.elapsed() >= Duration::from_secs(1) {
            fps = counted as f32 / since.elapsed().as_secs_f32();
            counted = 0;
            since = Instant::now();
        }
        if snapshot.elapsed() >= SNAPSHOT_EVERY {
            snapshot = Instant::now();
            world.snapshot(fps, &controls);
        }
    }
}

/// Play `secs` seconds on the autopilot without a terminal and print the
/// screen. With `cells`, one line per cell instead: `x y fg bg symbol`,
/// colours as hex, for `tools/shot.py` to draw as an image.
fn shot(secs: f32, (w, h): (u16, u16), seed: u64, spacing: f32, wave: u32, cells: bool) {
    let screen = Rect::new(0, 0, w, h);
    let mut world = World::new(screen, seed, spacing);
    world.start_wave = wave;
    world.restart();
    for _ in 0..(secs * 60.0) as u32 {
        let c = world.demo_controls();
        world.update(c);
    }
    let mut buf = Buffer::empty(screen);
    world.draw(&mut buf, screen, "headless");
    if cells {
        let hex = |c: Color| match c {
            Color::Rgb(r, g, b) => format!("{r:02x}{g:02x}{b:02x}"),
            _ => "000000".into(),
        };
        println!("{w} {h}");
        for y in 0..h {
            for x in 0..w {
                let c = &buf[(x, y)];
                println!("{x} {y} {} {} {}", hex(c.fg), hex(c.bg), c.symbol().escape_unicode());
            }
        }
        return;
    }
    for y in 0..h {
        let row: String = (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect();
        println!("{}", row.trim_end());
    }
}

/// Time update plus drawing into a buffer on the autopilot, without a
/// terminal.
fn bench(frames: u32, (w, h): (u16, u16), seed: u64, spacing: f32) {
    let screen = Rect::new(0, 0, w, h);
    let mut world = World::new(screen, seed, spacing);
    world.auto_blasts = true;
    let mut buf = Buffer::empty(screen);
    let (mut upd, mut drw) = (Duration::ZERO, Duration::ZERO);
    for _ in 0..frames {
        let t = Instant::now();
        let c: Controls = world.demo_controls();
        world.update(c);
        upd += t.elapsed();
        let t = Instant::now();
        world.draw(&mut buf, screen, "bench");
        drw += t.elapsed();
    }
    let ms = |d: Duration| d.as_secs_f64() * 1000.0 / frames as f64;
    println!("{w}x{h} cells, {frames} frames: update {:.3} ms, draw {:.3} ms a frame; score {}, {} enemies, phase {:?}", ms(upd), ms(drw), world.score, world.enemies.list.len(), world.phase);
}
