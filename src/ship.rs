//! The ship and its shots.
//!
//! The ship is a shell prompt, `❯`, drawn as a chevron with an inner
//! stroke; it turns to face where it flies, leaves exhaust behind, and
//! fires a twin stream wherever it is aimed. Shots shove the grid as they
//! pass and burst against the walls.

use crate::canvas::{Canvas, Rgb};
use crate::fx::Fx;
use crate::grid::Grid;
use crate::input::Controls;
use crate::rng::Rng;

/// Acceleration per frame at full stick, and velocity kept per frame: top
/// speed is ACCEL / (1 - DRAG) dots a frame.
const ACCEL: f32 = 0.38;
const DRAG: f32 = 0.85;
const SHOT_SPEED: f32 = 6.0;
/// Frames between volleys.
const COOLDOWN: u32 = 5;
const SHOT_LIFE: u32 = 240;
pub const BOMBS: u32 = 3;
const SHIP: Rgb = [0.75, 1.3, 0.85];
const SHOT: Rgb = [1.4, 1.2, 0.45];

pub struct Shot {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    age: u32,
}

impl Shot {
    #[cfg(test)]
    pub fn test(x: f32, y: f32) -> Shot {
        Shot { x, y, vx: 0.0, vy: 0.0, age: 0 }
    }
}

/// Frames the ship blinks, untouchable, after coming back.
pub const SHIELD: u32 = 120;
/// How close an enemy has to come to hit the ship.
pub const RADIUS: f32 = 3.5;

pub struct Ship {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    /// Facing, radians.
    angle: f32,
    cooldown: u32,
    /// Alternates the two barrels' tiny spread.
    volley: u32,
    pub bombs: u32,
    pub shots: Vec<Shot>,
    /// Frames left untouchable.
    pub shield: u32,
}

impl Ship {
    pub fn new(x: f32, y: f32) -> Ship {
        Ship { x, y, vx: 0.0, vy: 0.0, angle: -std::f32::consts::FRAC_PI_2, cooldown: 0, volley: 0, bombs: BOMBS, shots: Vec::new(), shield: 0 }
    }

    /// Back on the field after being lost: still, shielded, no shots out.
    pub fn respawn(&mut self) {
        (self.vx, self.vy) = (0.0, 0.0);
        self.shots.clear();
        self.shield = SHIELD;
    }

    /// One frame. Returns where a bomb went off, if one did.
    pub fn update(&mut self, c: Controls, grid: &mut Grid, fx: &mut Fx, rng: &mut Rng, w: f32, h: f32) -> Option<(f32, f32)> {
        self.shield = self.shield.saturating_sub(1);
        self.vx = (self.vx + c.mv.0 * ACCEL) * DRAG;
        self.vy = (self.vy + c.mv.1 * ACCEL) * DRAG;
        let margin = 4.0;
        self.x = (self.x + self.vx).clamp(margin, w - margin);
        self.y = (self.y + self.vy).clamp(margin, h - margin);
        let speed = self.vx.hypot(self.vy);
        if speed > 0.3 {
            // turn towards the way it flies, not snapping
            let want = self.vy.atan2(self.vx);
            let mut d = want - self.angle;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            self.angle += d * 0.35;
            grid.pull(self.x, self.y, -0.25 - speed * 0.05, 16.0);
            if rng.f32() < speed * 0.4 {
                let a = self.angle + std::f32::consts::PI + (rng.f32() - 0.5) * 0.8;
                let s = 0.8 + rng.f32() * 1.2;
                let (bx, by) = (self.x - self.angle.cos() * 4.0, self.y - self.angle.sin() * 4.0);
                fx.spark(bx, by, a.cos() * s + self.vx * 0.3, a.sin() * s + self.vy * 0.3, [1.0, 0.45, 0.1], 0.06);
            }
        }

        self.cooldown = self.cooldown.saturating_sub(1);
        if let Some(a) = c.fire {
            if self.cooldown == 0 {
                self.cooldown = COOLDOWN;
                self.volley += 1;
                let (ca, sa) = (a.cos(), a.sin());
                let wobble = if self.volley % 2 == 0 { 0.02 } else { -0.02 };
                for side in [-1.0f32, 1.0] {
                    let (ox, oy) = (-sa * side * 1.5, ca * side * 1.5);
                    let b = a + wobble * side;
                    self.shots.push(Shot { x: self.x + ox + ca * 3.0, y: self.y + oy + sa * 3.0, vx: b.cos() * SHOT_SPEED, vy: b.sin() * SHOT_SPEED, age: 0 });
                }
            }
        }

        self.shots.retain_mut(|s| {
            s.x += s.vx;
            s.y += s.vy;
            s.age += 1;
            grid.pull(s.x, s.y, -0.35, 8.0);
            let out = s.x < 0.0 || s.y < 0.0 || s.x >= w || s.y >= h;
            if out {
                let (x, y) = (s.x.clamp(0.0, w - 1.0), s.y.clamp(0.0, h - 1.0));
                grid.blast(x, y, 1.5, 16.0);
                for _ in 0..5 {
                    let a = rng.f32() * std::f32::consts::TAU;
                    let v = 0.5 + rng.f32() * 1.5;
                    fx.spark(x, y, a.cos() * v, a.sin() * v, SHOT, 0.05);
                }
            }
            !out && s.age < SHOT_LIFE
        });

        if c.bomb && self.bombs > 0 {
            self.bombs -= 1;
            log::info!("bomb at ({:.0}, {:.0}), {} left", self.x, self.y, self.bombs);
            fx.ring(self.x, self.y);
            grid.blast(self.x, self.y, 14.0, 260.0);
            return Some((self.x, self.y));
        }
        None
    }

    /// Draw the shots, and the ship unless `hidden` (lost and not back yet).
    pub fn draw(&self, c: &mut Canvas, hidden: bool) {
        for s in &self.shots {
            c.line(s.x - s.vx * 0.7, s.y - s.vy * 0.7, s.x, s.y, SHOT);
        }
        if hidden || (self.shield > 0 && (self.shield / 6) % 2 == 1) {
            return;
        }
        let (ca, sa) = (self.angle.cos(), self.angle.sin());
        let at = |(px, py): (f32, f32)| (self.x + px * ca - py * sa, self.y + px * sa + py * ca);
        // the prompt chevron and its inner stroke
        let outer = [at((-4.0, -5.5)), at((5.0, 0.0)), at((-4.0, 5.5))];
        let inner = [at((-3.0, -2.0)), at((1.0, 0.0)), at((-3.0, 2.0))];
        c.line(outer[0].0, outer[0].1, outer[1].0, outer[1].1, SHIP);
        c.line(outer[1].0, outer[1].1, outer[2].0, outer[2].1, SHIP);
        c.line(inner[0].0, inner[0].1, inner[1].0, inner[1].1, SHIP);
        c.line(inner[1].0, inner[1].1, inner[2].0, inner[2].1, SHIP);
    }
}
