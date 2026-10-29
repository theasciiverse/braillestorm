//! Sparks, black holes and bomb rings.

use crate::canvas::{Canvas, Rgb};
use crate::grid::Grid;
use crate::rng::Rng;

pub struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    c: Rgb,
    /// 1 when born, gone at 0.
    life: f32,
    fade: f32,
}

pub struct Hole {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    /// Grows as it swallows sparks.
    pub mass: f32,
    phase: f32,
}

/// A bomb's shock front, growing until it fades.
pub struct Ring {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

const RING_SPEED: f32 = 5.0;
const RING_MAX: f32 = 260.0;

#[derive(Default)]
pub struct Fx {
    pub sparks: Vec<Spark>,
    pub holes: Vec<Hole>,
    pub rings: Vec<Ring>,
}

const SPARK_MAX: usize = 4000;

impl Fx {
    /// A burst of `n` sparks in hue `c`, plus a shove to the grid.
    pub fn blast(&mut self, grid: &mut Grid, rng: &mut Rng, x: f32, y: f32, c: Rgb, n: usize) {
        grid.blast(x, y, 6.0, 70.0);
        for _ in 0..n {
            let a = rng.f32() * std::f32::consts::TAU;
            let s = 0.6 + rng.f32() * rng.f32() * 5.0;
            // a little white mixed in so the core of a burst is hot
            let w = rng.f32() * 0.5;
            let fade = 0.008 + rng.f32() * 0.02;
            self.spark(x, y, a.cos() * s, a.sin() * s, [c[0] + w, c[1] + w, c[2] + w], fade);
        }
    }

    /// One spark; `fade` is the life it loses a frame.
    pub fn spark(&mut self, x: f32, y: f32, vx: f32, vy: f32, c: Rgb, fade: f32) {
        if self.sparks.len() >= SPARK_MAX {
            self.sparks.remove(0);
        }
        self.sparks.push(Spark { x, y, vx, vy, c, life: 1.0, fade });
    }

    pub fn ring(&mut self, x: f32, y: f32) {
        self.rings.push(Ring { x, y, r: 4.0 });
    }

    pub fn toggle_hole(&mut self, x: f32, y: f32) {
        if let Some(i) = self.holes.iter().position(|h| (h.x - x).hypot(h.y - y) < 12.0 + h.mass) {
            self.holes.remove(i);
        } else {
            self.holes.push(Hole { x, y, vx: 0.0, vy: 0.0, mass: 4.0, phase: 0.0 });
        }
    }

    /// One frame; `wells` are the nulls on the field (x, y, strength),
    /// which drink sparks as the black holes do.
    pub fn update(&mut self, grid: &mut Grid, rng: &mut Rng, w: f32, h: f32, wells: &[(f32, f32, f32)]) {
        self.rings.retain_mut(|r| {
            r.r += RING_SPEED;
            r.r < RING_MAX
        });
        for hole in &mut self.holes {
            // drift slowly, kept off the edges
            hole.vx = (hole.vx + (rng.f32() - 0.5) * 0.04) * 0.99;
            hole.vy = (hole.vy + (rng.f32() - 0.5) * 0.04) * 0.99;
            hole.x = (hole.x + hole.vx).clamp(20.0, w - 20.0);
            hole.y = (hole.y + hole.vy).clamp(20.0, h - 20.0);
            hole.phase += 0.15;
            grid.pull(hole.x, hole.y, 0.12 + hole.mass * 0.01, 50.0 + hole.mass * 3.0);
        }
        let holes = &mut self.holes;
        self.sparks.retain_mut(|s| {
            for hole in holes.iter_mut() {
                let (dx, dy) = (hole.x - s.x, hole.y - s.y);
                let d2 = dx * dx + dy * dy;
                if d2 < 9.0 {
                    hole.mass = (hole.mass + 0.02).min(30.0);
                    return false;
                }
                let k = (0.9 + hole.mass * 0.05) / d2.max(25.0);
                s.vx += dx * k;
                s.vy += dy * k;
            }
            for &(wx, wy, ws) in wells {
                let (dx, dy) = (wx - s.x, wy - s.y);
                let d2 = dx * dx + dy * dy;
                if d2 < 16.0 {
                    return false;
                }
                let k = 1.4 * ws / d2.max(36.0);
                s.vx += dx * k;
                s.vy += dy * k;
            }
            s.vx *= 0.97;
            s.vy *= 0.97;
            s.x += s.vx;
            s.y += s.vy;
            if s.x < 0.0 || s.x >= w {
                s.vx = -s.vx;
                s.x = s.x.clamp(0.0, w - 1.0);
            }
            if s.y < 0.0 || s.y >= h {
                s.vy = -s.vy;
                s.y = s.y.clamp(0.0, h - 1.0);
            }
            s.life -= s.fade;
            s.life > 0.0
        });
    }

    pub fn draw(&self, c: &mut Canvas) {
        for s in &self.sparks {
            let l = s.life * s.life * 1.6;
            // a streak behind the spark, longer the faster it goes
            c.line(s.x - s.vx * 2.5, s.y - s.vy * 2.5, s.x, s.y, [s.c[0] * l, s.c[1] * l, s.c[2] * l]);
        }
        for r in &self.rings {
            let l = 1.0 - r.r / RING_MAX;
            c.circle(r.x, r.y, r.r, [0.6 * l, 1.0 * l, 1.2 * l]);
            c.circle(r.x, r.y, r.r * 0.94, [0.2 * l, 0.4 * l, 0.6 * l]);
        }
        for h in &self.holes {
            let r = 4.0 + h.mass * 0.4;
            let pulse = 1.0 + (h.phase).sin() * 0.15;
            c.circle(h.x, h.y, r * pulse, [1.2, 0.25, 0.2]);
            c.circle(h.x, h.y, r * 0.55, [0.6, 0.1, 0.1]);
            c.circle(h.x, h.y, r * 1.7 * pulse, [0.25, 0.05, 0.12]);
        }
    }
}
