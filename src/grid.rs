//! The warping grid: a mesh of points joined by springs to their
//! neighbours and, more weakly, to where they started. Blasts push the
//! points out, black holes pull them in, and the springs ring it back to
//! rest.

use crate::canvas::{Canvas, Rgb};

/// Spring between neighbours.
const STIFF: f32 = 0.28;
const DAMP: f32 = 0.06;
/// Spring back to the rest position.
const HOME: f32 = 0.035;
const HOME_DAMP: f32 = 0.02;
/// Velocity kept per step.
const FRICTION: f32 = 0.94;
/// Physics steps per frame.
const STEPS: usize = 2;
/// Every MAJOR-th line is drawn brighter.
const MAJOR: usize = 4;

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    rx: f32,
    ry: f32,
    pinned: bool,
}

pub struct Grid {
    cols: usize,
    rows: usize,
    pub spacing: f32,
    pts: Vec<Point>,
    fx: Vec<f32>,
    fy: Vec<f32>,
}

impl Grid {
    /// A grid filling `w` by `h` dots with roughly `spacing` dots between
    /// lines; the border points are pinned.
    pub fn new(w: usize, h: usize, spacing: f32) -> Grid {
        let cols = ((w as f32 - 1.0) / spacing).round().max(1.0) as usize + 1;
        let rows = ((h as f32 - 1.0) / spacing).round().max(1.0) as usize + 1;
        let (sx, sy) = ((w as f32 - 1.0) / (cols - 1) as f32, (h as f32 - 1.0) / (rows - 1) as f32);
        let mut pts = Vec::with_capacity(cols * rows);
        for j in 0..rows {
            for i in 0..cols {
                let (x, y) = (i as f32 * sx, j as f32 * sy);
                pts.push(Point { x, y, vx: 0.0, vy: 0.0, rx: x, ry: y, pinned: i == 0 || j == 0 || i == cols - 1 || j == rows - 1 });
            }
        }
        let n = pts.len();
        Grid { cols, rows, spacing, pts, fx: vec![0.0; n], fy: vec![0.0; n] }
    }

    /// Push points away from (x, y), strongest at the centre, nothing past
    /// `radius`.
    pub fn blast(&mut self, x: f32, y: f32, force: f32, radius: f32) {
        let r2 = radius * radius;
        for p in self.pts.iter_mut().filter(|p| !p.pinned) {
            let (dx, dy) = (p.x - x, p.y - y);
            let d2 = dx * dx + dy * dy;
            if d2 < r2 {
                let d = d2.sqrt().max(1.0);
                let k = force * (1.0 - d / radius) / d;
                p.vx += dx * k;
                p.vy += dy * k;
            }
        }
    }

    /// Pull points towards (x, y) for one frame; negative `force` pushes.
    pub fn pull(&mut self, x: f32, y: f32, force: f32, radius: f32) {
        let r2 = radius * radius;
        for p in self.pts.iter_mut().filter(|p| !p.pinned) {
            let (dx, dy) = (x - p.x, y - p.y);
            let d2 = dx * dx + dy * dy;
            if d2 < r2 && d2 > 4.0 {
                let d = d2.sqrt();
                let k = force * (1.0 - d / radius) / d;
                p.vx += dx * k;
                p.vy += dy * k;
            }
        }
    }

    pub fn update(&mut self) {
        let (cols, rows) = (self.cols, self.rows);
        for _ in 0..STEPS {
            self.fx.fill(0.0);
            self.fy.fill(0.0);
            for j in 0..rows {
                for i in 0..cols {
                    let a = j * cols + i;
                    if i + 1 < cols {
                        self.spring(a, a + 1);
                    }
                    if j + 1 < rows {
                        self.spring(a, a + cols);
                    }
                }
            }
            for (k, p) in self.pts.iter_mut().enumerate() {
                if p.pinned {
                    continue;
                }
                let ax = self.fx[k] + (p.rx - p.x) * HOME - p.vx * HOME_DAMP;
                let ay = self.fy[k] + (p.ry - p.y) * HOME - p.vy * HOME_DAMP;
                p.vx = (p.vx + ax) * FRICTION;
                p.vy = (p.vy + ay) * FRICTION;
                // a hard limit keeps a huge blast from tearing the mesh apart
                let v2 = p.vx * p.vx + p.vy * p.vy;
                if v2 > 36.0 {
                    let s = 6.0 / v2.sqrt();
                    p.vx *= s;
                    p.vy *= s;
                }
                p.x += p.vx;
                p.y += p.vy;
            }
        }
    }

    /// Springs pull neighbours towards each other in proportion to how far
    /// each has moved from rest relative to the other, which keeps the
    /// mesh's shape and lets a dent travel outwards as a wave.
    fn spring(&mut self, a: usize, b: usize) {
        let (pa, pb) = (self.pts[a], self.pts[b]);
        let ex = (pb.x - pb.rx) - (pa.x - pa.rx);
        let ey = (pb.y - pb.ry) - (pa.y - pa.ry);
        let fx = ex * STIFF + (pb.vx - pa.vx) * DAMP;
        let fy = ey * STIFF + (pb.vy - pa.vy) * DAMP;
        self.fx[a] += fx;
        self.fy[a] += fy;
        self.fx[b] -= fx;
        self.fy[b] -= fy;
    }

    /// Draw the lines; they light up where the mesh is stretched.
    pub fn draw(&self, c: &mut Canvas, base: Rgb) {
        let (cols, rows) = (self.cols, self.rows);
        let colour = |p: &Point, q: &Point, major: bool| {
            let off = ((p.x - p.rx).abs() + (p.y - p.ry).abs() + (q.x - q.rx).abs() + (q.y - q.ry).abs()) * 0.5;
            let lift = (off / self.spacing).min(1.0);
            let k = if major { 1.0 } else { 0.5 };
            [base[0] * (k + lift * 0.9), base[1] * (k + lift * 0.9), base[2] * (k + lift * 0.9)]
        };
        for j in 0..rows {
            for i in 0..cols {
                let p = &self.pts[j * cols + i];
                if i + 1 < cols {
                    let q = &self.pts[j * cols + i + 1];
                    c.line(p.x, p.y, q.x, q.y, colour(p, q, j % MAJOR == 0));
                }
                if j + 1 < rows {
                    let q = &self.pts[(j + 1) * cols + i];
                    c.line(p.x, p.y, q.x, q.y, colour(p, q, i % MAJOR == 0));
                }
            }
        }
    }

    /// Total displacement, for the session log.
    pub fn energy(&self) -> f32 {
        self.pts.iter().map(|p| (p.x - p.rx).abs() + (p.y - p.ry).abs()).sum()
    }

    pub fn size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settles_back_to_rest() {
        let mut g = Grid::new(200, 120, 8.0);
        g.blast(100.0, 60.0, 40.0, 60.0);
        g.update();
        assert!(g.energy() > 10.0);
        for _ in 0..600 {
            g.update();
        }
        assert!(g.energy() < 1.0, "energy left {}", g.energy());
    }

    #[test]
    fn border_stays_put() {
        let mut g = Grid::new(100, 60, 8.0);
        for _ in 0..30 {
            g.pull(50.0, 30.0, 3.0, 80.0);
            g.update();
        }
        let p = g.pts[0];
        assert_eq!((p.x, p.y), (p.rx, p.ry));
    }
}
