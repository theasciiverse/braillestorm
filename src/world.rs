//! The play field and the game on it: canvas, grid, effects, the ship and
//! the enemies, score, lives and game over; plus the sandbox switches from
//! the grid test (glow, grid colour and spacing, random blasts) and a demo
//! autopilot that flies the ship.

use crate::canvas::{Canvas, Rgb};
use crate::enemies::Enemies;
use crate::fx::Fx;
use crate::grid::Grid;
use crate::input::Controls;
use crate::rng::Rng;
use crate::save;
use crate::ship::{self, Ship};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// Grid colours: blue, Asciiverse green, amber, violet.
pub const GRID_COLOURS: [(&str, Rgb); 4] = [("blue", [0.07, 0.10, 0.32]), ("green", [0.04, 0.28, 0.10]), ("amber", [0.30, 0.17, 0.03]), ("violet", [0.21, 0.06, 0.30])];
/// Random blast hues.
const HUES: [Rgb; 6] = [[1.0, 0.45, 0.1], [0.2, 0.9, 1.0], [1.0, 0.25, 0.7], [0.4, 1.0, 0.3], [1.0, 0.9, 0.2], [0.6, 0.4, 1.0]];
const BOMB_RADIUS: f32 = 200.0;
const LIVES: u32 = 3;
/// An extra life and an extra bomb every this many points.
const BONUS_EVERY: u64 = 25_000;
/// Frames between losing the ship and it coming back (or game over).
const LOST: u32 = 100;
/// Frames on the game over screen before a key starts again; the demo
/// starts again by itself after `DEMO_AGAIN`.
const OVER_WAIT: u32 = 60;
const DEMO_AGAIN: u32 = 240;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Play,
    /// The ship is lost; frames until it comes back.
    Lost(u32),
    /// Frames since the game ended.
    Over(u32),
}

pub struct World {
    /// Where the canvas sits on screen.
    pub area: Rect,
    canvas: Canvas,
    grid: Grid,
    fx: Fx,
    pub rng: Rng,
    pub ship: Ship,
    pub enemies: Enemies,
    pub phase: Phase,
    pub paused: bool,
    pub score: u64,
    pub lives: u32,
    pub hi: u64,
    /// Whether this game beat the high score it started with.
    new_hi: bool,
    next_bonus: u64,
    /// Save the high score (not for headless runs).
    pub keep_hi: bool,
    /// The autopilot flew some of this game, so it does not count.
    tainted: bool,
    pub spacing: f32,
    pub glow: bool,
    pub colour: usize,
    /// Random blasts from the grid test.
    pub auto_blasts: bool,
    /// The demo autopilot flies the ship.
    pub demo: bool,
    pub hud: bool,
    blast_wait: u32,
    pub frames: u64,
}

impl World {
    pub fn new(screen: Rect, seed: u64, spacing: f32) -> World {
        let area = play_area(screen, true);
        let canvas = Canvas::new(area.width, area.height);
        let grid = Grid::new(canvas.w, canvas.h, spacing);
        let ship = Ship::new(canvas.w as f32 / 2.0, canvas.h as f32 / 2.0);
        World {
            area,
            canvas,
            grid,
            fx: Fx::default(),
            rng: Rng::new(seed),
            ship,
            enemies: Enemies::default(),
            phase: Phase::Play,
            paused: false,
            score: 0,
            lives: LIVES,
            hi: 0,
            new_hi: false,
            next_bonus: BONUS_EVERY,
            keep_hi: false,
            tainted: false,
            spacing,
            glow: true,
            colour: 0,
            auto_blasts: false,
            demo: false,
            hud: true,
            blast_wait: 30,
            frames: 0,
        }
    }

    /// A fresh game on the same field.
    pub fn restart(&mut self) {
        let (w, h) = self.dims();
        self.ship = Ship::new(w / 2.0, h / 2.0);
        self.ship.shield = ship::SHIELD;
        self.enemies = Enemies::default();
        self.phase = Phase::Play;
        self.paused = false;
        self.score = 0;
        self.lives = LIVES;
        self.new_hi = false;
        self.next_bonus = BONUS_EVERY;
        self.tainted = self.demo;
        log::info!("new game");
    }

    /// Rebuild for a new screen size or spacing; the ship stays (moved
    /// inside if it has to), the enemies are cleared without points.
    pub fn rebuild(&mut self, screen: Rect) {
        self.area = play_area(screen, self.hud);
        self.canvas = Canvas::new(self.area.width, self.area.height);
        self.grid = Grid::new(self.canvas.w, self.canvas.h, self.spacing);
        self.fx = Fx::default();
        self.enemies.list.clear();
        let (w, h) = self.dims();
        self.ship.x = self.ship.x.clamp(4.0, w - 4.0);
        self.ship.y = self.ship.y.clamp(4.0, h - 4.0);
        log::info!("rebuild canvas {}x{} cells, grid {:?} spacing {}", self.area.width, self.area.height, self.grid.size(), self.spacing);
    }

    pub fn dims(&self) -> (f32, f32) {
        (self.canvas.w as f32, self.canvas.h as f32)
    }

    /// A cell on screen to the dot at its middle, if it is on the field.
    pub fn to_dots(&self, col: u16, row: u16) -> Option<(f32, f32)> {
        let a = self.area;
        (col >= a.x && row >= a.y && col < a.right() && row < a.bottom()).then(|| ((col - a.x) as f32 * 2.0 + 1.0, (row - a.y) as f32 * 4.0 + 2.0))
    }

    pub fn toggle_hole(&mut self) {
        let (w, h) = self.dims();
        let at = match self.fx.holes.first() {
            Some(hole) => (hole.x, hole.y),
            None => (w * (0.2 + self.rng.f32() * 0.6), h * (0.2 + self.rng.f32() * 0.6)),
        };
        self.fx.toggle_hole(at.0, at.1);
        log::debug!("black holes {}", self.fx.holes.len());
    }

    /// Enter, a pad's Start: start again after game over, else pause.
    pub fn start_or_pause(&mut self) {
        match self.phase {
            Phase::Over(t) if t >= OVER_WAIT => self.restart(),
            Phase::Over(_) => {}
            _ => self.paused = !self.paused,
        }
    }

    /// What the demo autopilot does this frame: drift along a slow figure of
    /// eight, steer away from anything close (and the walls), fire at the
    /// nearest enemy, and bomb when they close in.
    pub fn demo_controls(&mut self) -> Controls {
        let (w, h) = self.dims();
        let (sx, sy) = (self.ship.x, self.ship.y);
        let t = self.frames as f32 / 60.0;
        let goal = (w * (0.5 + 0.35 * (t * 0.31).sin()), h * (0.5 + 0.3 * (t * 0.62).sin()));
        let (gx, gy) = (goal.0 - sx, goal.1 - sy);
        let gd = gx.hypot(gy).max(1.0);
        let (mut mx, mut my) = (gx / gd * 0.4, gy / gd * 0.4);
        const FEAR: f32 = 45.0;
        for e in &self.enemies.list {
            let (dx, dy) = (sx - e.x, sy - e.y);
            let d = dx.hypot(dy).max(1.0);
            if d < FEAR {
                let k = (1.0 - d / FEAR).powi(2) * 3.0;
                mx += dx / d * k;
                my += dy / d * k;
            }
        }
        let wall = 24.0;
        mx += ((wall - sx) / wall).max(0.0) - ((sx - (w - wall)) / wall).max(0.0);
        my += ((wall - sy) / wall).max(0.0) - ((sy - (h - wall)) / wall).max(0.0);
        let l = mx.hypot(my);
        let mv = if l > 0.05 { (mx / l, my / l) } else { (0.0, 0.0) };
        let fire = self.enemies.nearest(sx, sy).map(|(x, y)| (y - sy).atan2(x - sx));
        let crowd = self.enemies.count_within(sx, sy, 22.0);
        let bomb = crowd >= 3 && self.ship.bombs > 0 && self.ship.shield == 0;
        Controls { mv, fire, bomb }
    }

    pub fn update(&mut self, c: Controls) {
        if self.paused {
            return;
        }
        self.frames += 1;
        self.tainted |= self.demo;
        let (w, h) = self.dims();
        if self.auto_blasts {
            if self.blast_wait == 0 {
                let (x, y) = (w * (0.1 + self.rng.f32() * 0.8), h * (0.1 + self.rng.f32() * 0.8));
                let hue = HUES[(self.rng.next() % HUES.len() as u64) as usize];
                let n = 80 + (self.rng.next() % 120) as usize;
                self.fx.blast(&mut self.grid, &mut self.rng, x, y, hue, n);
                self.blast_wait = 20 + (self.rng.next() % 50) as u32;
            } else {
                self.blast_wait -= 1;
            }
        }
        match self.phase {
            Phase::Play => {
                self.enemies.spawn(&mut self.rng, w, h, (self.ship.x, self.ship.y));
                if let Some((x, y)) = self.ship.update(c, &mut self.grid, &mut self.fx, &mut self.rng, w, h) {
                    let p = self.enemies.clear_around(x, y, BOMB_RADIUS, &mut self.grid, &mut self.fx, &mut self.rng);
                    self.add_score(p);
                }
                self.enemies.update(&mut self.rng, w, h, Some((self.ship.x, self.ship.y)));
                let p = self.enemies.shoot(&mut self.ship.shots, &mut self.grid, &mut self.fx, &mut self.rng);
                self.add_score(p);
                if self.ship.shield == 0 {
                    if let Some(kind) = self.enemies.touching(self.ship.x, self.ship.y, ship::RADIUS) {
                        self.lose_ship(kind);
                    }
                }
            }
            Phase::Lost(t) => {
                self.enemies.update(&mut self.rng, w, h, None);
                if t > 1 {
                    self.phase = Phase::Lost(t - 1);
                } else if self.lives > 0 {
                    self.ship.respawn();
                    self.phase = Phase::Play;
                    log::info!("ship back, {} lives left", self.lives);
                } else {
                    self.game_over();
                }
            }
            Phase::Over(t) => {
                self.phase = Phase::Over(t + 1);
                if self.demo && t >= DEMO_AGAIN {
                    self.restart();
                } else if t >= OVER_WAIT && c.bomb {
                    self.restart();
                }
            }
        }
        self.fx.update(&mut self.grid, &mut self.rng, w, h);
        self.grid.update();
    }

    fn add_score(&mut self, p: u64) {
        self.score += p;
        while self.score >= self.next_bonus {
            self.next_bonus += BONUS_EVERY;
            self.lives += 1;
            self.ship.bombs += 1;
            log::info!("bonus at {}: lives {} bombs {}", self.score, self.lives, self.ship.bombs);
        }
    }

    fn lose_ship(&mut self, by: crate::enemies::Kind) {
        let (x, y) = (self.ship.x, self.ship.y);
        log::info!("ship lost to {by:?} at ({x:.0}, {y:.0}), score {}, {} lives left", self.score, self.lives - 1);
        self.lives -= 1;
        self.fx.blast(&mut self.grid, &mut self.rng, x, y, [0.75, 1.3, 0.85], 300);
        self.fx.ring(x, y);
        self.grid.blast(x, y, 14.0, 260.0);
        self.enemies.clear_all(&mut self.grid, &mut self.fx, &mut self.rng);
        self.ship.shots.clear();
        self.phase = Phase::Lost(LOST);
    }

    fn game_over(&mut self) {
        self.phase = Phase::Over(0);
        let counts = !self.tainted;
        if counts && self.score > self.hi {
            self.hi = self.score;
            self.new_hi = true;
            if self.keep_hi {
                save::save_hi(self.hi);
            }
        }
        log::info!("game over, score {} (counts {counts}), hi {}", self.score, self.hi);
    }

    pub fn draw(&mut self, buf: &mut Buffer, screen: Rect, status: &str) {
        self.canvas.clear();
        self.grid.draw(&mut self.canvas, GRID_COLOURS[self.colour].1);
        self.enemies.draw(&mut self.canvas);
        self.fx.draw(&mut self.canvas);
        self.ship.draw(&mut self.canvas, self.phase != Phase::Play);
        self.canvas.resolve(buf, self.area, self.glow);
        if self.hud && screen.height > 0 {
            let hi = self.hi.max(if self.tainted { 0 } else { self.score });
            let line = format!(" braillestorm  score {}  hi {}  lives {}  bombs {}  {status}", thousands(self.score), thousands(hi), self.lives, self.ship.bombs);
            let style = Style::new().fg(Color::Rgb(0x6a, 0xd0, 0x6a)).bg(Color::Rgb(0, 0, 0));
            buf.set_stringn(screen.x, screen.y, format!("{line:<w$}", w = screen.width as usize), screen.width as usize, style);
        }
        let mut lines: Vec<String> = Vec::new();
        if let Phase::Over(t) = self.phase {
            lines.push("GAME OVER".into());
            lines.push(format!("score {}", thousands(self.score)));
            if self.new_hi {
                lines.push("new high score".into());
            }
            if t >= OVER_WAIT && !self.demo {
                lines.push(String::new());
                lines.push("Space or Enter to play again, q to quit".into());
            }
        } else if self.paused {
            lines.push("PAUSED".into());
            lines.push("p or Enter to go on".into());
        }
        let top = self.area.y + self.area.height.saturating_sub(lines.len() as u16) / 2;
        let style = Style::new().fg(Color::Rgb(0xff, 0xc0, 0x40)).bg(Color::Rgb(0, 0, 0));
        for (i, l) in lines.iter().enumerate() {
            let w = l.chars().count() as u16;
            if l.is_empty() || w > self.area.width {
                continue;
            }
            buf.set_string(self.area.x + (self.area.width - w) / 2, top + i as u16, l, style);
        }
    }

    pub fn snapshot(&self, fps: f32, c: &Controls) {
        log::debug!(
            "snapshot frame {} fps {fps:.1} phase {:?} paused {} score {} lives {} ship ({:.0}, {:.0}) shield {} controls {:?} shots {} bombs {} enemies {} clock {} sparks {} holes {} energy {:.0} demo {}",
            self.frames,
            self.phase,
            self.paused,
            self.score,
            self.lives,
            self.ship.x,
            self.ship.y,
            self.ship.shield,
            c,
            self.ship.shots.len(),
            self.ship.bombs,
            self.enemies.list.len(),
            self.enemies.clock,
            self.fx.sparks.len(),
            self.fx.holes.len(),
            self.grid.energy(),
            self.demo
        );
    }
}

/// 12345 as "12,345".
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The screen minus the top line when the HUD shows.
fn play_area(screen: Rect, hud: bool) -> Rect {
    let top = u16::from(hud && screen.height > 1);
    Rect::new(screen.x, screen.y + top, screen.width, screen.height - top)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(12345), "12,345");
        assert_eq!(thousands(1234567), "1,234,567");
    }
}
