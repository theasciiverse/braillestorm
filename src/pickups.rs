//! Pick-ups: a letter in a pulsing box, dropped now and then by an enemy
//! shot down, waiting where it fell until the ship flies through it or it
//! fades. Drops are paced by time, not kills, about one every half minute:
//! more firepower must not mean more pick-ups, or they snowball.
//!
//! - W, wide: a third stream either side of the twin one, for a while.
//! - R, rapid: volleys twice as often, for a while.
//! - S, shield: untouchable for a few seconds.
//! - B, bomb: one more bomb.

use crate::canvas::{Canvas, Rgb};
use crate::enemies::Kind;
use crate::rng::Rng;
use crate::text;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Power {
    Wide,
    Rapid,
    Shield,
    Bomb,
}

impl Power {
    fn letter(self) -> &'static str {
        match self {
            Power::Wide => "W",
            Power::Rapid => "R",
            Power::Shield => "S",
            Power::Bomb => "B",
        }
    }

    fn colour(self) -> Rgb {
        match self {
            Power::Wide => [1.3, 0.75, 0.2],
            Power::Rapid => [1.3, 1.2, 0.3],
            Power::Shield => [0.4, 1.3, 1.1],
            Power::Bomb => [1.3, 0.4, 0.35],
        }
    }
}

/// Frames a pick-up waits before it is gone; it blinks for the last
/// `BLINK` of them.
const LIFE: u32 = 8 * 60;
const BLINK: u32 = 2 * 60;
/// At most this many on the field at once.
const MAX: usize = 2;
/// How close the ship has to come to take one.
const REACH: f32 = 7.0;
/// Nothing drops sooner than `QUIET` frames after the last drop; then the
/// chance for each kill climbs to `CHANCE` over `RAMP` frames. A null
/// drops half the time once `NULL_QUIET` has passed.
const QUIET: u32 = 20 * 60;
const RAMP: u32 = 30 * 60;
const CHANCE: f32 = 0.15;
const NULL_QUIET: u32 = 8 * 60;
const NULL_CHANCE: f32 = 0.5;

pub struct Pickup {
    pub power: Power,
    pub x: f32,
    pub y: f32,
    age: u32,
}

#[derive(Default)]
pub struct Pickups {
    pub list: Vec<Pickup>,
    /// Frames since the last drop (or the start of the game).
    since: u32,
}

impl Pickups {
    /// Maybe drop something where an enemy of kind `k` was shot down.
    pub fn maybe_drop(&mut self, k: Kind, x: f32, y: f32, rng: &mut Rng) {
        let chance = if k == Kind::Null && self.since >= NULL_QUIET {
            NULL_CHANCE
        } else {
            CHANCE * (self.since.saturating_sub(QUIET) as f32 / RAMP as f32).min(1.0)
        };
        if self.list.len() >= MAX || rng.f32() >= chance {
            return;
        }
        self.since = 0;
        let power = match rng.next() % 10 {
            0..=2 => Power::Wide,
            3..=5 => Power::Rapid,
            6..=7 => Power::Shield,
            _ => Power::Bomb,
        };
        log::info!("pick-up {power:?} at ({x:.0}, {y:.0})");
        self.list.push(Pickup { power, x, y, age: 0 });
    }

    /// Age them, keep them on the field, and hand over one the ship at
    /// (sx, sy) flies through.
    pub fn update(&mut self, w: f32, h: f32, ship: Option<(f32, f32)>) -> Option<Power> {
        self.since += 1;
        let mut taken = None;
        self.list.retain_mut(|p| {
            p.age += 1;
            p.x = p.x.clamp(6.0, w - 6.0);
            p.y = p.y.clamp(6.0, h - 6.0);
            if taken.is_none() && ship.is_some_and(|(sx, sy)| (sx - p.x).hypot(sy - p.y) < REACH) {
                taken = Some(p.power);
                return false;
            }
            p.age < LIFE
        });
        taken
    }

    pub fn nearest(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        self.list.iter().map(|p| (p.x, p.y)).min_by(|a, b| (a.0 - x).hypot(a.1 - y).total_cmp(&(b.0 - x).hypot(b.1 - y)))
    }

    pub fn draw(&self, c: &mut Canvas) {
        for p in &self.list {
            if p.age + BLINK > LIFE && (p.age / 6) % 2 == 1 {
                continue;
            }
            let k = p.power.colour();
            let pulse = 1.0 + ((p.age as f32) * 0.12).sin() * 0.12;
            // the letter, and a box round it a cell's shape wider
            let (lw, lh) = (text::width(p.power.letter(), 1), text::height(1));
            text::draw(c, p.x - lw / 2.0, p.y - lh / 2.0, 1, p.power.letter(), k);
            let (bw, bh) = ((lw / 2.0 + 3.0) * pulse, (lh / 2.0 + 2.5) * pulse);
            let dim = [k[0] * 0.55, k[1] * 0.55, k[2] * 0.55];
            c.poly(&[(p.x - bw, p.y - bh), (p.x + bw, p.y - bh), (p.x + bw, p.y + bh), (p.x - bw, p.y + bh)], dim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_wait_their_time_and_the_field_holds_two() {
        let mut rng = Rng::new(1);
        let mut p = Pickups::default();
        for _ in 0..200 {
            p.maybe_drop(Kind::Fork, 50.0, 50.0, &mut rng);
        }
        assert!(p.list.is_empty(), "nothing in the quiet time");
        for _ in 0..QUIET + RAMP {
            p.update(200.0, 116.0, None);
        }
        for _ in 0..200 {
            p.maybe_drop(Kind::Fork, 50.0, 50.0, &mut rng);
        }
        assert_eq!(p.list.len(), 1, "one, then quiet again");
        p.since = QUIET + RAMP;
        for _ in 0..200 {
            p.maybe_drop(Kind::Fork, 50.0, 50.0, &mut rng);
            p.since = QUIET + RAMP;
        }
        assert_eq!(p.list.len(), MAX);
    }

    #[test]
    fn the_ship_takes_one_and_the_rest_fade() {
        let mut p = Pickups::default();
        p.list.push(Pickup { power: Power::Wide, x: 50.0, y: 50.0, age: 0 });
        p.list.push(Pickup { power: Power::Bomb, x: 150.0, y: 50.0, age: 0 });
        assert_eq!(p.update(200.0, 116.0, Some((52.0, 51.0))), Some(Power::Wide));
        assert_eq!(p.list.len(), 1);
        for _ in 0..LIFE {
            p.update(200.0, 116.0, None);
        }
        assert!(p.list.is_empty());
    }
}
