//! Enemies, our own roster, drawn as things from a terminal:
//!
//! - Glob, an asterisk `*` that tumbles about at random; dangerous only if
//!   you fly into it. 25 points.
//! - Cursor, a hollow block cursor that blinks and homes in, a little faster
//!   the longer it chases. 50 points.
//! - Fork, a branching `Y` that homes in slowly; shot, it forks into two
//!   children that rush the ship and overshoot. 100 points, children 25.
//! - Spinner, the `|/-\` of a busy terminal: it spins in place, then dashes
//!   straight across or up and down to line up with the ship. 75 points.
//! - Null, `∅`, the null device: it creeps after the ship pulling the grid,
//!   sparks, shots and the ship itself in, and takes twenty-five hits. 250 points.
//!
//! Each one fades in where it spawns and can neither hurt nor be hurt until
//! it has. The spawner speeds up and lets in new kinds as the game goes on,
//! and spawns more on a bigger field.

use crate::canvas::{Canvas, Rgb};
use crate::fx::Fx;
use crate::grid::Grid;
use crate::rng::Rng;
use crate::ship::Shot;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Glob,
    Cursor,
    Fork,
    Child,
    Spinner,
    Null,
}

impl Kind {
    fn radius(self) -> f32 {
        match self {
            Kind::Glob => 4.5,
            Kind::Cursor => 4.5,
            Kind::Fork => 5.0,
            Kind::Child => 2.5,
            Kind::Spinner => 4.5,
            Kind::Null => 7.0,
        }
    }

    pub fn points(self) -> u64 {
        match self {
            Kind::Glob => 25,
            Kind::Cursor => 50,
            Kind::Fork => 100,
            Kind::Child => 25,
            Kind::Spinner => 75,
            Kind::Null => 250,
        }
    }

    fn colour(self) -> Rgb {
        match self {
            Kind::Glob => [0.75, 0.4, 1.3],
            Kind::Cursor => [0.2, 0.9, 1.3],
            Kind::Fork => [1.3, 0.35, 0.8],
            Kind::Child => [1.3, 0.6, 0.15],
            Kind::Spinner => [0.6, 1.3, 0.25],
            Kind::Null => [1.35, 1.15, 0.95],
        }
    }

    /// Hits it takes.
    fn hits(self) -> u32 {
        match self {
            Kind::Null => NULL_HITS,
            _ => 1,
        }
    }
}

/// Frames an enemy takes to fade in.
const ARRIVE: u32 = 45;
const NULL_HITS: u32 = 25;
/// A null's reach, in dots, and how hard it pulls at its rim.
pub const NULL_REACH: f32 = 70.0;
const NULL_PULL: f32 = 0.05;
/// A spinner spins this many frames between dashes, and dashes at most this
/// many, at `SPIN_DASH` dots a frame.
const SPIN_WAIT: u32 = 50;
const SPIN_DASH_FRAMES: u32 = 40;
const SPIN_DASH: f32 = 2.6;

pub struct Enemy {
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    /// Heading for globs, facing for forks, spin for the rest.
    angle: f32,
    /// Frames since it spawned.
    age: u32,
    /// Hits left.
    hp: u32,
    /// A spinner's frames into spinning or dashing; a null's flinch after a hit.
    t: u32,
}

impl Enemy {
    fn solid(&self) -> bool {
        self.age >= ARRIVE
    }
}

/// Frames between waves, with nothing spawning.
pub const BREATHER: u32 = 150;
/// Frames a wave may linger once everything is out before the next comes
/// anyway (a glob can wander a long way from the guns).
const LINGER: u32 = 15 * 60;

/// What happened to the enemies this frame, for the score and the sound.
#[derive(Default)]
pub struct Events {
    /// Shot down, by kind.
    pub shot: Vec<Kind>,
    /// Gone in a bomb blast.
    pub bombed: Vec<Kind>,
    pub splits: u32,
    pub dents: u32,
    pub arrivals: u32,
}

/// A wave has begun: its number, and whether the one before it ended with
/// the field cleared (rather than by running out of time).
pub struct NewWave {
    pub wave: u32,
    pub cleared: bool,
}

#[derive(Default)]
pub struct Enemies {
    pub list: Vec<Enemy>,
    /// The current wave, from 1; 0 before the first.
    pub wave: u32,
    /// Enemies this wave still has to send.
    budget: u32,
    /// Frames since the wave sent its last enemy.
    since: u32,
    /// Frames of quiet left before the wave starts sending.
    pub breather: u32,
    wait: u32,
    pub events: Events,
}

/// A warning for a wave's banner, if it needs one: only rushes do (the
/// enemies explain themselves, a rush changes what to do next).
pub fn wave_note(wave: u32) -> Option<&'static str> {
    rush(wave).then_some("rush!")
}

/// Every fifth wave from the tenth is a rush of corner cursors.
fn rush(wave: u32) -> bool {
    wave >= 10 && wave % 5 == 0
}

impl Enemies {
    /// Start the next game at `wave` instead of the first.
    pub fn start_at(&mut self, wave: u32) {
        self.wave = wave.saturating_sub(1);
    }

    /// Run the waves: send enemies while the wave has some to send, then,
    /// once the field is clear (or it has lingered too long), take a
    /// breather and start the next. `ship` is kept clear of new arrivals.
    pub fn spawn(&mut self, rng: &mut Rng, w: f32, h: f32, ship: (f32, f32)) -> Option<NewWave> {
        if self.breather > 0 {
            self.breather -= 1;
            return None;
        }
        // a 100x30 terminal is 1; a big field gets more enemies, sooner
        let size = (w * h / (200.0 * 116.0)).sqrt().clamp(1.0, 4.0);
        if self.budget == 0 {
            self.since += 1;
            let cleared = self.list.is_empty();
            if !cleared && self.since < LINGER {
                return None;
            }
            self.wave += 1;
            let n = (8.0 + self.wave as f32 * 5.0) * size * if rush(self.wave) { 1.5 } else { 1.0 };
            self.budget = n as u32;
            self.since = 0;
            self.breather = BREATHER;
            self.wait = 0;
            log::info!("wave {} ({} enemies), last one {}", self.wave, self.budget, if cleared { "cleared" } else { "timed out" });
            return Some(NewWave { wave: self.wave, cleared });
        }
        if self.wait > 0 {
            self.wait -= 1;
            return None;
        }
        let wave = self.wave;
        let cap = ((12.0 + wave as f32 * 4.0) * size).min(250.0) as usize;
        self.wait = ((60.0 - wave as f32 * 6.0).max(8.0) / size) as u32;
        // packs and corner waves grow every few waves
        let more = (wave / 4) as u64;
        if self.list.len() >= cap {
            return None;
        }
        let before = self.list.len();
        let roll = rng.f32();
        let corners = if rush(wave) { 0.5 } else if wave >= 5 { 0.12 } else { 0.0 };
        match () {
            _ if roll < corners => {
                // a cursor out of each corner
                for (cx, cy) in [(0.08, 0.1), (0.92, 0.1), (0.08, 0.9), (0.92, 0.9)] {
                    for i in 0..1 + more.min(3) {
                        self.add(Kind::Cursor, w * cx + i as f32 * 3.0, h * cy, rng);
                    }
                }
            }
            _ if wave >= 3 && roll < corners + 0.18 => {
                // a pack of globs together, away from the ship
                if let Some((x, y)) = place(rng, w, h, ship) {
                    for _ in 0..4 + more.min(8) + rng.next() % 5 {
                        let (ox, oy) = ((rng.f32() - 0.5) * 24.0, (rng.f32() - 0.5) * 24.0);
                        self.add(Kind::Glob, (x + ox).clamp(6.0, w - 6.0), (y + oy).clamp(6.0, h - 6.0), rng);
                    }
                }
            }
            _ if wave >= 8 && roll > 0.96 && self.count(Kind::Null) < 1 + (wave >= 16) as usize => {
                if let Some((x, y)) = place(rng, w, h, ship) {
                    self.add(Kind::Null, x, y, rng);
                }
            }
            _ => {
                let kind = if wave >= 6 && roll > 0.82 {
                    Kind::Spinner
                } else if wave >= 4 && roll < 0.5 {
                    Kind::Fork
                } else if wave >= 2 && roll < 0.75 {
                    Kind::Cursor
                } else {
                    Kind::Glob
                };
                if let Some((x, y)) = place(rng, w, h, ship) {
                    self.add(kind, x, y, rng);
                }
            }
        }
        let arrived = (self.list.len() - before) as u32;
        self.budget = self.budget.saturating_sub(arrived);
        self.events.arrivals += arrived;
        None
    }

    fn add(&mut self, kind: Kind, x: f32, y: f32, rng: &mut Rng) {
        self.list.push(Enemy { kind, x, y, vx: 0.0, vy: 0.0, angle: rng.f32() * TAU, age: 0, hp: kind.hits(), t: 0 });
    }

    fn count(&self, kind: Kind) -> usize {
        self.list.iter().filter(|e| e.kind == kind).count()
    }

    /// Where the nulls are, and how hard each pulls: (x, y, strength).
    pub fn wells(&self) -> Vec<(f32, f32, f32)> {
        self.list.iter().filter(|e| e.kind == Kind::Null && e.solid()).map(|e| (e.x, e.y, 1.0)).collect()
    }

    /// The pull of every null on something at (x, y): a velocity change,
    /// strongest at the rim, nothing past its reach.
    pub fn pull_at(&self, x: f32, y: f32, k: f32) -> (f32, f32) {
        let (mut ax, mut ay) = (0.0, 0.0);
        for (nx, ny, s) in self.wells() {
            let (dx, dy) = (nx - x, ny - y);
            let d = dx.hypot(dy).max(1.0);
            if d < NULL_REACH {
                let f = NULL_PULL * s * k * (1.0 - d / NULL_REACH).powf(0.7);
                ax += dx / d * f;
                ay += dy / d * f;
            }
        }
        (ax, ay)
    }

    /// Move everything one frame; `ship` is where they chase, if the ship
    /// is there to chase.
    pub fn update(&mut self, rng: &mut Rng, w: f32, h: f32, ship: Option<(f32, f32)>) {
        // everything gets quicker wave by wave, up to 1.6 times as fast
        let heat = 1.0 + (self.wave.saturating_sub(1) as f32 * 0.05).min(0.6);
        for e in &mut self.list {
            e.age += 1;
            if !e.solid() {
                e.angle += 0.05;
                continue;
            }
            let toward = ship.map(|(sx, sy)| {
                let (dx, dy) = (sx - e.x, sy - e.y);
                let d = dx.hypot(dy).max(1.0);
                (dx / d, dy / d)
            });
            match e.kind {
                Kind::Glob => {
                    e.angle += (rng.f32() - 0.5) * 0.35;
                    e.vx = e.angle.cos() * 0.7;
                    e.vy = e.angle.sin() * 0.7;
                }
                Kind::Cursor => {
                    let (tx, ty) = toward.unwrap_or((0.0, 0.0));
                    let chase = ((e.age - ARRIVE) as f32 / 600.0).min(1.0);
                    let top = 0.9 + chase * 0.7;
                    e.vx = (e.vx + tx * 0.06) * 0.97;
                    e.vy = (e.vy + ty * 0.06) * 0.97;
                    let s = e.vx.hypot(e.vy);
                    if s > top {
                        e.vx *= top / s;
                        e.vy *= top / s;
                    }
                }
                Kind::Fork => {
                    if let Some((tx, ty)) = toward {
                        let mut d = ty.atan2(tx) - e.angle;
                        while d > PI {
                            d -= TAU;
                        }
                        while d < -PI {
                            d += TAU;
                        }
                        e.angle += d.clamp(-0.03, 0.03);
                    }
                    e.vx = e.angle.cos() * 0.6;
                    e.vy = e.angle.sin() * 0.6;
                }
                Kind::Child => {
                    let (tx, ty) = toward.unwrap_or((0.0, 0.0));
                    e.vx = (e.vx + tx * 0.12) * 0.95;
                    e.vy = (e.vy + ty * 0.12) * 0.95;
                    e.angle += 0.25;
                }
                Kind::Spinner => {
                    e.t += 1;
                    if e.vx == 0.0 && e.vy == 0.0 {
                        // spinning, a quarter turn every few frames, then off
                        // along whichever axis lines it up with the ship
                        if e.t % 4 == 0 {
                            e.angle += PI / 4.0;
                        }
                        if e.t >= SPIN_WAIT {
                            e.t = 0;
                            if let Some((sx, sy)) = ship {
                                let (dx, dy) = (sx - e.x, sy - e.y);
                                (e.vx, e.vy) = if dx.abs() > dy.abs() { (SPIN_DASH * dx.signum(), 0.0) } else { (0.0, SPIN_DASH * dy.signum()) };
                                e.angle = if e.vx != 0.0 { 0.0 } else { PI / 2.0 };
                            }
                        }
                    } else {
                        // dashing until level with the ship, or out of breath
                        let level = ship.is_some_and(|(sx, sy)| if e.vx != 0.0 { (sx - e.x).abs() < 2.0 } else { (sy - e.y).abs() < 2.0 });
                        if level || e.t >= SPIN_DASH_FRAMES {
                            (e.vx, e.vy) = (0.0, 0.0);
                            e.t = 0;
                        }
                    }
                }
                Kind::Null => {
                    let (tx, ty) = toward.unwrap_or((0.0, 0.0));
                    e.vx = (e.vx + tx * 0.01) * 0.98;
                    e.vy = (e.vy + ty * 0.01) * 0.98;
                    e.angle += 0.02;
                    e.t = e.t.saturating_sub(1);
                }
            }
            e.x += e.vx * heat;
            e.y += e.vy * heat;
            let r = e.kind.radius();
            if e.kind == Kind::Spinner && (e.x < r || e.x > w - r || e.y < r || e.y > h - r) {
                // a wall ends a dash
                (e.vx, e.vy) = (0.0, 0.0);
                e.t = 0;
            }
            if e.x < r || e.x > w - r {
                e.vx = -e.vx;
                e.angle = PI - e.angle;
                e.x = e.x.clamp(r, w - r);
            }
            if e.y < r || e.y > h - r {
                e.vy = -e.vy;
                e.angle = -e.angle;
                e.y = e.y.clamp(r, h - r);
            }
        }
        // keep them from stacking into one shape
        let n = self.list.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (&self.list[i], &self.list[j]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let near = a.kind.radius() + b.kind.radius();
                let d2 = dx * dx + dy * dy;
                if d2 < near * near && d2 > 0.01 {
                    let d = d2.sqrt();
                    let push = (near - d) * 0.15 / d;
                    self.list[i].x -= dx * push;
                    self.list[i].y -= dy * push;
                    self.list[j].x += dx * push;
                    self.list[j].y += dy * push;
                }
            }
        }
    }

    /// Burst whatever a shot touches (and that shot); what went is in
    /// `events.shot`.
    pub fn shoot(&mut self, shots: &mut Vec<Shot>, grid: &mut Grid, fx: &mut Fx, rng: &mut Rng) {
        let mut i = 0;
        while i < self.list.len() {
            let e = &self.list[i];
            let r = e.kind.radius() + 1.5;
            let hit = e.solid().then(|| shots.iter().position(|s: &Shot| (s.x - e.x).hypot(s.y - e.y) < r)).flatten();
            if let Some(k) = hit {
                let (sx, sy) = (shots[k].x, shots[k].y);
                shots.swap_remove(k);
                if self.list[i].hp > 1 {
                    // it takes more: a dent and a few sparks
                    let e = &mut self.list[i];
                    e.hp -= 1;
                    e.t = 8;
                    self.events.dents += 1;
                    grid.blast(sx, sy, 2.0, 30.0);
                    for _ in 0..6 {
                        let a = rng.f32() * TAU;
                        let v = 0.6 + rng.f32() * 1.6;
                        fx.spark(sx, sy, a.cos() * v, a.sin() * v, e.kind.colour(), 0.05);
                    }
                    i += 1;
                    continue;
                }
                let e = self.list.swap_remove(i);
                self.events.shot.push(e.kind);
                let sparks = if e.kind == Kind::Null { 400 } else { 60 };
                self.burst(&e, grid, fx, rng, sparks);
                if e.kind == Kind::Null {
                    fx.ring(e.x, e.y);
                    grid.blast(e.x, e.y, 10.0, 160.0);
                }
                if e.kind == Kind::Fork {
                    self.events.splits += 1;
                    // two children thrown out either side of the shot's line
                    let a = (e.y - sy).atan2(e.x - sx);
                    for side in [-1.0f32, 1.0] {
                        let b = a + side * PI / 2.0;
                        let (vx, vy) = (b.cos() * 2.0, b.sin() * 2.0);
                        self.list.push(Enemy { kind: Kind::Child, x: e.x + vx * 2.0, y: e.y + vy * 2.0, vx, vy, angle: 0.0, age: ARRIVE, hp: 1, t: 0 });
                    }
                }
                continue;
            }
            i += 1;
        }
    }

    /// Burst every enemy within `radius` of (x, y), for bombs; what went is
    /// in `events.bombed`. Forks and nulls die whole.
    pub fn clear_around(&mut self, x: f32, y: f32, radius: f32, grid: &mut Grid, fx: &mut Fx, rng: &mut Rng) {
        let (gone, kept): (Vec<Enemy>, Vec<Enemy>) = self.list.drain(..).partition(|e| (e.x - x).hypot(e.y - y) < radius);
        self.list = kept;
        for e in gone {
            self.events.bombed.push(e.kind);
            self.burst(&e, grid, fx, rng, 30);
        }
    }

    /// Everything bursts, for when the ship is lost; no points.
    pub fn clear_all(&mut self, grid: &mut Grid, fx: &mut Fx, rng: &mut Rng) {
        for e in std::mem::take(&mut self.list) {
            self.burst(&e, grid, fx, rng, 15);
        }
    }

    fn burst(&self, e: &Enemy, grid: &mut Grid, fx: &mut Fx, rng: &mut Rng, sparks: usize) {
        fx.blast(grid, rng, e.x, e.y, e.kind.colour(), sparks);
    }

    /// Whether a solid enemy touches a circle at (x, y).
    pub fn touching(&self, x: f32, y: f32, r: f32) -> Option<Kind> {
        self.list.iter().find(|e| e.solid() && (e.x - x).hypot(e.y - y) < e.kind.radius() + r).map(|e| e.kind)
    }

    pub fn nearest(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        self.list.iter().filter(|e| e.solid()).map(|e| (e.x, e.y)).min_by(|a, b| (a.0 - x).hypot(a.1 - y).total_cmp(&(b.0 - x).hypot(b.1 - y)))
    }

    pub fn count_within(&self, x: f32, y: f32, r: f32) -> usize {
        self.list.iter().filter(|e| (e.x - x).hypot(e.y - y) < r).count()
    }

    pub fn draw(&self, c: &mut Canvas) {
        for e in &self.list {
            // fading in: drawn bigger and dimmer, settling to size
            let (scale, l) = if e.solid() {
                (1.0, 1.0)
            } else {
                let f = e.age as f32 / ARRIVE as f32;
                let blink = if (e.age / 4) % 2 == 0 { 1.0 } else { 0.5 };
                (1.0 + (1.0 - f) * 1.5, (0.2 + f * 0.5) * blink)
            };
            let k = e.kind.colour();
            let col = [k[0] * l, k[1] * l, k[2] * l];
            let r = e.kind.radius() * scale;
            let (x, y) = (e.x, e.y);
            match e.kind {
                Kind::Glob => {
                    for i in 0..3 {
                        let a = e.angle * 0.6 + i as f32 * PI / 3.0;
                        let (dx, dy) = (a.cos() * r, a.sin() * r);
                        c.line(x - dx, y - dy, x + dx, y + dy, col);
                    }
                }
                Kind::Cursor => {
                    // a cell is twice as tall as wide; so is the cursor
                    let (hw, hh) = (r * 0.7, r * 1.25);
                    c.poly(&[(x - hw, y - hh), (x + hw, y - hh), (x + hw, y + hh), (x - hw, y + hh)], col);
                    if (e.age / 20) % 2 == 0 {
                        let mut yy = y - hh + 2.0;
                        while yy < y + hh - 1.0 {
                            c.line(x - hw + 2.0, yy, x + hw - 2.0, yy, [col[0] * 0.6, col[1] * 0.6, col[2] * 0.6]);
                            yy += 2.0;
                        }
                    }
                }
                Kind::Fork => {
                    // stem behind, two branches reaching forward
                    let (ca, sa) = (e.angle.cos(), e.angle.sin());
                    let at = |px: f32, py: f32| (x + (px * ca - py * sa) * r, y + (px * sa + py * ca) * r);
                    let (tail, node, a, b) = (at(-1.0, 0.0), at(0.1, 0.0), at(1.0, -0.8), at(1.0, 0.8));
                    c.line(tail.0, tail.1, node.0, node.1, col);
                    c.line(node.0, node.1, a.0, a.1, col);
                    c.line(node.0, node.1, b.0, b.1, col);
                    c.circle(a.0, a.1, 1.2 * scale, col);
                    c.circle(b.0, b.1, 1.2 * scale, col);
                }
                Kind::Child => {
                    let (ca, sa) = (e.angle.cos() * r, e.angle.sin() * r);
                    c.poly(&[(x + ca, y + sa), (x - sa, y + ca), (x - ca, y - sa), (x + sa, y - ca)], col);
                }
                Kind::Spinner => {
                    // one bar of | / - \, a cell's height long, with a
                    // faint trail of the step before while it spins
                    let bar = |a: f32, k: f32, c: &mut Canvas| {
                        let (dx, dy) = (a.cos() * r * 0.75, a.sin() * r * 1.3);
                        c.line(x - dx, y - dy, x + dx, y + dy, [col[0] * k, col[1] * k, col[2] * k]);
                    };
                    if e.vx == 0.0 && e.vy == 0.0 {
                        bar(e.angle - PI / 4.0, 0.3, c);
                    }
                    bar(e.angle, 1.0, c);
                    c.circle(x, y, 1.0, col);
                }
                Kind::Null => {
                    // the null sign: a ring with a stroke through it, and
                    // a faint halo where its pull reaches; it flinches when hit
                    let flinch = 1.0 + e.t as f32 * 0.03;
                    let rr = r * flinch;
                    c.circle(x, y, rr, col);
                    // the stroke leans like the glyph's, rocking a little
                    let a = PI / 4.0 + e.angle.sin() * 0.12;
                    let (dx, dy) = (a.cos() * rr * 1.35, a.sin() * rr * 1.35);
                    c.line(x - dx, y + dy, x + dx, y - dy, col);
                    let hurt = 1.0 - e.hp as f32 / NULL_HITS as f32;
                    let halo = 0.12 + hurt * 0.15;
                    c.circle(x, y, rr * 2.2, [col[0] * halo, col[1] * halo * 0.6, col[2] * halo * 0.4]);
                }
            }
        }
    }
}

/// Somewhere for a new enemy, well away from the ship.
fn place(rng: &mut Rng, w: f32, h: f32, ship: (f32, f32)) -> Option<(f32, f32)> {
    let clear = (w.min(h) * 0.45).min(80.0);
    for _ in 0..30 {
        let (x, y) = (w * (0.05 + rng.f32() * 0.9), h * (0.05 + rng.f32() * 0.9));
        if (x - ship.0).hypot(y - ship.1) >= clear {
            return Some((x, y));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_clear_of_the_ship() {
        let mut rng = Rng::new(7);
        let mut en = Enemies::default();
        en.start_at(6);
        for _ in 0..60 * 20 {
            en.spawn(&mut rng, 200.0, 116.0, (100.0, 58.0));
        }
        assert!(!en.list.is_empty());
        // corner cursors are placed by pattern, not by `place`
        for e in en.list.iter().filter(|e| e.kind != Kind::Cursor) {
            assert!((e.x - 100.0).hypot(e.y - 58.0) >= 52.0 - 18.0, "{:?} at {}, {}", e.kind, e.x, e.y);
        }
    }

    #[test]
    fn waves_follow_a_cleared_field() {
        let mut rng = Rng::new(5);
        let mut en = Enemies::default();
        let first = en.spawn(&mut rng, 200.0, 116.0, (100.0, 58.0)).expect("wave 1 starts at once");
        assert_eq!(first.wave, 1);
        let mut started = Vec::new();
        for _ in 0..60 * 120 {
            if let Some(w) = en.spawn(&mut rng, 200.0, 116.0, (100.0, 58.0)) {
                started.push((w.wave, w.cleared));
            }
            // everything shot the moment it is out
            en.list.clear();
        }
        assert!(started.len() >= 3, "{started:?}");
        assert!(started.iter().all(|&(_, cleared)| cleared));
        assert_eq!(started[0].0, 2);
    }

    #[test]
    fn a_lingering_wave_moves_on() {
        let mut rng = Rng::new(5);
        let mut en = Enemies::default();
        let mut started = Vec::new();
        for _ in 0..60 * 60 {
            if let Some(w) = en.spawn(&mut rng, 200.0, 116.0, (100.0, 58.0)) {
                started.push((w.wave, w.cleared));
            }
        }
        assert!(started.iter().any(|&(w, cleared)| w == 2 && !cleared), "{started:?}");
    }

    #[test]
    fn a_shot_fork_leaves_two_children() {
        let mut rng = Rng::new(3);
        let mut grid = Grid::new(200, 116, 8.0);
        let mut fx = Fx::default();
        let mut en = Enemies::default();
        en.add(Kind::Fork, 100.0, 50.0, &mut rng);
        en.list[0].age = ARRIVE;
        let mut shots = vec![Shot::test(99.0, 50.0)];
        en.shoot(&mut shots, &mut grid, &mut fx, &mut rng);
        assert_eq!(en.events.shot, vec![Kind::Fork]);
        assert_eq!(en.events.splits, 1);
        assert!(shots.is_empty());
        assert_eq!(en.list.iter().filter(|e| e.kind == Kind::Child).count(), 2);
    }

    #[test]
    fn a_null_takes_all_its_hits() {
        let mut rng = Rng::new(3);
        let mut grid = Grid::new(200, 116, 8.0);
        let mut fx = Fx::default();
        let mut en = Enemies::default();
        en.add(Kind::Null, 100.0, 50.0, &mut rng);
        en.list[0].age = ARRIVE;
        for i in 1..=NULL_HITS {
            let mut shots = vec![Shot::test(97.0, 50.0)];
            en.shoot(&mut shots, &mut grid, &mut fx, &mut rng);
            assert!(shots.is_empty(), "hit {i} not taken");
            assert_eq!(en.list.is_empty(), i == NULL_HITS, "after hit {i}");
        }
        assert_eq!(en.events.dents, NULL_HITS - 1);
        assert_eq!(en.events.shot, vec![Kind::Null]);
    }

    #[test]
    fn a_null_pulls_toward_itself_within_reach_only() {
        let mut rng = Rng::new(3);
        let mut en = Enemies::default();
        en.add(Kind::Null, 100.0, 50.0, &mut rng);
        en.list[0].age = ARRIVE;
        let (ax, ay) = en.pull_at(130.0, 50.0, 1.0);
        assert!(ax < 0.0 && ay.abs() < 1e-6, "{ax} {ay}");
        assert_eq!(en.pull_at(100.0 + NULL_REACH + 1.0, 50.0, 1.0), (0.0, 0.0));
    }

    #[test]
    fn a_spinner_dashes_to_line_up_with_the_ship() {
        let mut rng = Rng::new(3);
        let mut en = Enemies::default();
        en.add(Kind::Spinner, 40.0, 50.0, &mut rng);
        en.list[0].age = ARRIVE;
        let ship = (160.0, 60.0);
        for _ in 0..SPIN_WAIT + SPIN_DASH_FRAMES + 5 {
            en.update(&mut rng, 200.0, 116.0, Some(ship));
        }
        let e = &en.list[0];
        // it went sideways, toward the ship's column, not up or down
        assert!(e.x > 100.0, "x {}", e.x);
        assert!((e.y - 50.0).abs() < 1.0, "y {}", e.y);
    }

    #[test]
    fn arriving_enemies_are_harmless() {
        let mut rng = Rng::new(3);
        let mut en = Enemies::default();
        en.add(Kind::Cursor, 50.0, 50.0, &mut rng);
        assert_eq!(en.touching(50.0, 50.0, 4.0), None);
        en.list[0].age = ARRIVE;
        assert_eq!(en.touching(50.0, 50.0, 4.0), Some(Kind::Cursor));
    }
}
