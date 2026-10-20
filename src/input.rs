//! Keyboard, mouse and gamepad, boiled down to two sticks and a bomb.
//!
//! - Move: WASD, or the left stick.
//! - Fire: the arrow keys (eight ways), or hold the left mouse button to fire
//!   at the pointer, or the right stick.
//! - Bomb: Space, right click, or a shoulder button or A on a pad.
//!
//! With the kitty keyboard protocol presses and releases are exact. Without
//! it a key counts as held while presses and repeats keep arriving (as in
//! Panzer), which stutters for a moment after the first press; firing is
//! then sticky instead: the last arrow keeps firing until the opposite one
//! is pressed.
//!
//! Gamepads: only pads in SDL's controller database count (gilrs also
//! reports 3D mice and tablet key pads as gamepads); `BRAILLESTORM_ANY_PAD=1`
//! accepts everything.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

const PRE_REPEAT: Duration = Duration::from_millis(220);
const REPEAT_GAP: Duration = Duration::from_millis(110);
#[cfg(feature = "gamepad")]
const STICK_DEADZONE: f32 = 0.2;
#[cfg(feature = "gamepad")]
const AIM_THRESHOLD: f32 = 0.45;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    Up,
    Down,
    Left,
    Right,
    FireUp,
    FireDown,
    FireLeft,
    FireRight,
}
const KEYS: usize = 8;

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings || std::env::var("BRAILLESTORM_ANY_PAD").is_ok_and(|v| v == "1")
}

#[derive(Clone, Copy, Default)]
struct Hold {
    down: bool,
    last: Option<Instant>,
    repeats: u32,
}

impl Hold {
    fn held(&self, exact: bool, now: Instant) -> bool {
        if exact {
            return self.down;
        }
        let Some(last) = self.last else { return false };
        let window = if self.repeats == 0 { PRE_REPEAT } else { REPEAT_GAP };
        now.duration_since(last) <= window
    }
}

/// Commands outside the ship's controls, as edges.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Quit,
    Exit,
    Glow,
    Colour,
    Denser,
    Sparser,
    AutoBlasts,
    Hole,
    Hud,
    Demo,
    /// Play again after game over, else pause.
    Start,
    Pause,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AimBy {
    Keys,
    Mouse,
    Pad,
}

/// What the ship is told to do this frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    /// Length at most 1.
    pub mv: (f32, f32),
    /// Direction to fire in, radians, when firing.
    pub fire: Option<f32>,
    pub bomb: bool,
}

pub struct Input {
    pub exact: bool,
    holds: [Hold; KEYS],
    /// Sticky fire direction when key releases are not reported.
    sticky: Option<(f32, f32)>,
    aim_by: AimBy,
    /// Pointer cell, and whether the left button is down.
    pub mouse: Option<(u16, u16)>,
    mouse_fire: bool,
    bomb_edge: bool,
    pub cmds: Vec<Cmd>,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    pub pad_name: Option<String>,
    pad_move: (f32, f32),
    pad_aim: (f32, f32),
}

impl Input {
    pub fn new(exact: bool) -> Input {
        Input {
            exact,
            holds: [Hold::default(); KEYS],
            sticky: None,
            aim_by: AimBy::Keys,
            mouse: None,
            mouse_fire: false,
            bomb_edge: false,
            cmds: Vec::new(),
            #[cfg(feature = "gamepad")]
            gilrs: None,
            pad_name: None,
            pad_move: (0.0, 0.0),
            pad_aim: (0.0, 0.0),
        }
    }

    /// How firing is steered right now, for the top line.
    pub fn aim_name(&self) -> &'static str {
        match self.aim_by {
            AimBy::Keys if self.exact => "arrows",
            AimBy::Keys => "arrows (sticky)",
            AimBy::Mouse => "mouse",
            AimBy::Pad => "pad",
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let press = k.kind != KeyEventKind::Release;
        let fresh = k.kind == KeyEventKind::Press;
        if fresh && k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c' | 'C')) {
            self.cmds.push(Cmd::Exit);
            return;
        }
        let key = match k.code {
            KeyCode::Char('w' | 'W') => Some(Key::Up),
            KeyCode::Char('s' | 'S') => Some(Key::Down),
            KeyCode::Char('a' | 'A') => Some(Key::Left),
            KeyCode::Char('d' | 'D') => Some(Key::Right),
            KeyCode::Up => Some(Key::FireUp),
            KeyCode::Down => Some(Key::FireDown),
            KeyCode::Left => Some(Key::FireLeft),
            KeyCode::Right => Some(Key::FireRight),
            _ => None,
        };
        if let Some(key) = key {
            let h = &mut self.holds[key as usize];
            if self.exact {
                h.down = press;
            } else if press {
                if h.held(false, now) {
                    h.repeats += 1;
                } else {
                    h.repeats = 0;
                }
                h.last = Some(now);
            }
            if matches!(key, Key::FireUp | Key::FireDown | Key::FireLeft | Key::FireRight) && press {
                self.aim_by = AimBy::Keys;
                if !self.exact && fresh {
                    self.stick(key, now);
                }
            }
        }
        if !fresh {
            return;
        }
        match k.code {
            KeyCode::Char(' ') => self.bomb_edge = true,
            KeyCode::Char('q' | 'Q') | KeyCode::Esc => self.cmds.push(Cmd::Quit),
            KeyCode::Enter => self.cmds.push(Cmd::Start),
            KeyCode::Char('p' | 'P') => self.cmds.push(Cmd::Pause),
            KeyCode::Char('b' | 'B') => self.cmds.push(Cmd::Hole),
            KeyCode::Char('h' | 'H') => self.cmds.push(Cmd::Hud),
            KeyCode::F(2) => self.cmds.push(Cmd::Glow),
            KeyCode::F(3) => self.cmds.push(Cmd::Colour),
            KeyCode::F(4) => self.cmds.push(Cmd::Denser),
            KeyCode::F(5) => self.cmds.push(Cmd::Sparser),
            KeyCode::F(6) => self.cmds.push(Cmd::AutoBlasts),
            KeyCode::F(7) => self.cmds.push(Cmd::Demo),
            _ => {}
        }
    }

    /// Sticky firing: an arrow sets its axis (keeping the other one, so two
    /// arrows in a row make a diagonal), and the opposite arrow of a held
    /// direction stops firing.
    fn stick(&mut self, key: Key, now: Instant) {
        let (dx, dy) = match key {
            Key::FireUp => (0.0, -1.0),
            Key::FireDown => (0.0, 1.0),
            Key::FireLeft => (-1.0, 0.0),
            _ => (1.0, 0.0),
        };
        let (mut x, mut y) = self.sticky.unwrap_or((0.0, 0.0));
        let opposite = (dx != 0.0 && x == -dx) || (dy != 0.0 && y == -dy);
        if opposite {
            self.sticky = None;
            log::debug!("sticky fire off");
            return;
        }
        // an arrow long after the last one starts a fresh direction
        let recent = self.holds.iter().filter_map(|h| h.last).any(|t| t != now && now.duration_since(t) < Duration::from_millis(250));
        if !recent {
            (x, y) = (0.0, 0.0);
        }
        if dx != 0.0 {
            x = dx;
        }
        if dy != 0.0 {
            y = dy;
        }
        self.sticky = Some((x, y));
        log::debug!("sticky fire {:?}", self.sticky);
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::Moved => {
                self.mouse = Some((m.column, m.row));
                self.aim_by = AimBy::Mouse;
            }
            MouseEventKind::Drag(_) => self.mouse = Some((m.column, m.row)),
            MouseEventKind::Down(MouseButton::Left) => {
                log::debug!("mouse fire down at {},{}", m.column, m.row);
                self.mouse = Some((m.column, m.row));
                self.mouse_fire = true;
                self.aim_by = AimBy::Mouse;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                log::debug!("mouse fire up");
                self.mouse_fire = false;
            }
            MouseEventKind::Down(MouseButton::Right) => {
                log::debug!("mouse bomb");
                self.bomb_edge = true;
            }
            _ => {}
        }
    }

    /// The window lost focus: releases will not arrive, so let go of
    /// everything.
    pub fn release_all(&mut self) {
        self.holds = [Hold::default(); KEYS];
        self.mouse_fire = false;
    }

    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable_pad(&p))).collect();
                log::info!("gamepads (name, used): {all:?}");
                self.pad_name = all.into_iter().find(|(_, used)| *used).map(|(n, _)| n);
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    /// Drain gamepad events; call once a frame before `controls`.
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            let mut events = Vec::new();
            while let Some(ev) = g.next_event() {
                events.push(ev);
            }
            for ev in events {
                let g = self.gilrs.as_ref().unwrap();
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                match ev.event {
                    EventType::Connected => {
                        self.pad_name = Some(pad.name().to_string());
                        log::info!("gamepad connected: {:?}", self.pad_name);
                    }
                    EventType::Disconnected => {
                        log::info!("gamepad disconnected");
                        self.pad_name = None;
                    }
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad press {b:?}");
                        match b {
                            Button::South | Button::LeftTrigger | Button::RightTrigger | Button::LeftTrigger2 | Button::RightTrigger2 => self.bomb_edge = true,
                            Button::Start => self.cmds.push(Cmd::Start),
                            Button::Select => self.cmds.push(Cmd::Quit),
                            _ => {}
                        }
                    }
                    EventType::ButtonReleased(b, _) => log::debug!("pad release {b:?}"),
                    _ => {}
                }
                let axis = |a: Axis| pad.axis_data(a).map_or(0.0, |d| d.value());
                let dead = |(x, y): (f32, f32)| if x.hypot(y) < STICK_DEADZONE { (0.0, 0.0) } else { (x, y) };
                self.pad_move = dead((axis(Axis::LeftStickX), axis(Axis::LeftStickY)));
                self.pad_aim = dead((axis(Axis::RightStickX), axis(Axis::RightStickY)));
                if self.pad_aim.0.hypot(self.pad_aim.1) >= AIM_THRESHOLD {
                    self.aim_by = AimBy::Pad;
                }
            }
        }
    }

    /// Controls for this frame; `ship` and `pointer` are in dots, the
    /// pointer only when it is over the play field. Clears the edges.
    pub fn controls(&mut self, now: Instant, ship: (f32, f32), pointer: Option<(f32, f32)>) -> Controls {
        let held = |k: Key| self.holds[k as usize].held(self.exact, now);
        let axis = |neg: Key, pos: Key| held(pos) as i8 as f32 - held(neg) as i8 as f32;
        let keys_mv = (axis(Key::Left, Key::Right), axis(Key::Up, Key::Down));
        let mv = if keys_mv != (0.0, 0.0) {
            let l = keys_mv.0.hypot(keys_mv.1);
            (keys_mv.0 / l, keys_mv.1 / l)
        } else {
            // stick up is positive, screen up is negative y
            let (x, y) = (self.pad_move.0, -self.pad_move.1);
            let l = x.hypot(y).max(1.0);
            (x / l, y / l)
        };
        let fire = match self.aim_by {
            AimBy::Keys if self.exact => {
                let (x, y) = (axis(Key::FireLeft, Key::FireRight), axis(Key::FireUp, Key::FireDown));
                ((x, y) != (0.0, 0.0)).then(|| y.atan2(x))
            }
            AimBy::Keys => self.sticky.map(|(x, y)| y.atan2(x)),
            AimBy::Mouse => match pointer {
                Some((px, py)) if self.mouse_fire => Some((py - ship.1).atan2(px - ship.0)),
                _ => None,
            },
            AimBy::Pad => {
                let (x, y) = self.pad_aim;
                (x.hypot(y) >= 0.3).then(|| (-y).atan2(x))
            }
        };
        let bomb = std::mem::take(&mut self.bomb_edge);
        Controls { mv, fire, bomb }
    }
}
