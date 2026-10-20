//! Light on a Braille canvas.
//!
//! Everything is drawn as light into a buffer with one entry per Braille
//! dot (2 by 4 dots per terminal cell), and light adds up where things
//! overlap. `resolve` then turns each cell into one Braille character: a dot
//! is set where its light is bright enough, the cell's colour is its
//! brightest dot (a cell can only have one), and, with glow on, the cell
//! background takes a dim, blurred copy of the light around it, so bright
//! things bloom into their surroundings.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

pub type Rgb = [f32; 3];

/// Light below this does not set a dot.
const DOT_ON: f32 = 0.10;
/// How much of the blurred light shows as background glow.
const GLOW: f32 = 0.16;

/// Dot (dx, dy) within a cell to its bit in U+2800..U+28FF.
const BIT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

pub struct Canvas {
    /// Size in dots.
    pub w: usize,
    pub h: usize,
    light: Vec<Rgb>,
    /// Per-cell light sums for the glow, reused between frames.
    cells: Vec<Rgb>,
    blur: Vec<Rgb>,
}

impl Canvas {
    pub fn new(cols: u16, rows: u16) -> Canvas {
        let (w, h) = (cols as usize * 2, rows as usize * 4);
        let n = cols as usize * rows as usize;
        Canvas { w, h, light: vec![[0.0; 3]; w * h], cells: vec![[0.0; 3]; n], blur: vec![[0.0; 3]; n] }
    }

    pub fn clear(&mut self) {
        self.light.fill([0.0; 3]);
    }

    pub fn plot(&mut self, x: f32, y: f32, c: Rgb) {
        if x < 0.0 || y < 0.0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        let p = &mut self.light[y * self.w + x];
        p[0] += c[0];
        p[1] += c[1];
        p[2] += c[2];
    }

    /// A line one dot wide; each dot gets `c` once, ends included.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgb) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0);
        if steps > 4096.0 {
            return;
        }
        let (sx, sy) = (dx / steps, dy / steps);
        let n = steps as usize;
        for i in 0..=n {
            self.plot(x0 + sx * i as f32 + 0.5, y0 + sy * i as f32 + 0.5, c);
        }
    }

    /// A closed polygon through `pts`.
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Rgb) {
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            self.line(a.0, a.1, b.0, b.1, c);
        }
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32, c: Rgb) {
        let n = ((r * 1.6) as usize).clamp(8, 64);
        let pts: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                (x + a.cos() * r, y + a.sin() * r)
            })
            .collect();
        self.poly(&pts, c);
    }

    /// Turn the light into Braille cells in `area` (which must match the
    /// canvas size).
    pub fn resolve(&mut self, buf: &mut Buffer, area: Rect, glow: bool) {
        let (cols, rows) = (area.width as usize, area.height as usize);
        if cols * 2 != self.w || rows * 4 != self.h {
            return;
        }
        if glow {
            for cy in 0..rows {
                for cx in 0..cols {
                    let mut s = [0.0f32; 3];
                    for dy in 0..4 {
                        let row = (cy * 4 + dy) * self.w + cx * 2;
                        for dx in 0..2 {
                            let p = self.light[row + dx];
                            s[0] += p[0];
                            s[1] += p[1];
                            s[2] += p[2];
                        }
                    }
                    self.cells[cy * cols + cx] = s;
                }
            }
            box_blur(&self.cells, &mut self.blur, cols, rows);
        }
        for cy in 0..rows {
            for cx in 0..cols {
                let mut bits = 0u8;
                let mut best = [0.0f32; 3];
                let mut best_l = 0.0f32;
                for dy in 0..4 {
                    let row = (cy * 4 + dy) * self.w + cx * 2;
                    for dx in 0..2 {
                        let p = self.light[row + dx];
                        let l = p[0].max(p[1]).max(p[2]);
                        if l > DOT_ON {
                            bits |= BIT[dx][dy];
                            if l > best_l {
                                best_l = l;
                                best = p;
                            }
                        }
                    }
                }
                let cell = &mut buf[(area.x + cx as u16, area.y + cy as u16)];
                let bg = if glow {
                    let g = self.blur[cy * cols + cx];
                    tone([g[0] * GLOW, g[1] * GLOW, g[2] * GLOW], 0.5)
                } else {
                    Color::Rgb(0, 0, 0)
                };
                if bits == 0 {
                    cell.set_char(' ').set_bg(bg);
                } else {
                    let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                    cell.set_char(ch).set_fg(tone(best, 1.0)).set_bg(bg);
                }
            }
        }
    }
}

/// 3 by 3 box blur over the cell grid, weighted to 1/8 of a cell's dots so
/// one fully lit cell glows at about its own brightness.
fn box_blur(src: &[Rgb], dst: &mut [Rgb], cols: usize, rows: usize) {
    for cy in 0..rows {
        for cx in 0..cols {
            let mut s = [0.0f32; 3];
            for oy in cy.saturating_sub(1)..(cy + 2).min(rows) {
                for ox in cx.saturating_sub(1)..(cx + 2).min(cols) {
                    let p = src[oy * cols + ox];
                    s[0] += p[0];
                    s[1] += p[1];
                    s[2] += p[2];
                }
            }
            dst[cy * cols + cx] = [s[0] / 72.0, s[1] / 72.0, s[2] / 72.0];
        }
    }
}

/// Light to a terminal colour: soft saturation, so stacked light goes
/// towards white instead of clipping, quantised to cut down on redraws.
fn tone(c: Rgb, max: f32) -> Color {
    let f = |v: f32| {
        let v = (1.0 - (-v * 1.6).exp()) * max;
        ((v * 31.0).round() / 31.0 * 255.0) as u8
    };
    Color::Rgb(f(c[0]), f(c[1]), f(c[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_map_to_braille_bits() {
        let mut c = Canvas::new(1, 1);
        c.plot(0.0, 0.0, [1.0, 1.0, 1.0]);
        c.plot(1.0, 3.0, [1.0, 1.0, 1.0]);
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        c.resolve(&mut buf, area, false);
        assert_eq!(buf[(0, 0)].symbol(), "\u{2881}");
    }

    #[test]
    fn dim_light_sets_no_dot() {
        let mut c = Canvas::new(1, 1);
        c.plot(0.0, 0.0, [0.05, 0.05, 0.05]);
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        c.resolve(&mut buf, area, false);
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }

    #[test]
    fn lines_stay_inside() {
        let mut c = Canvas::new(4, 2);
        c.line(-50.0, -50.0, 100.0, 100.0, [1.0, 0.0, 0.0]);
        c.circle(4.0, 4.0, 30.0, [0.0, 1.0, 0.0]);
    }
}
