//! All sound is synthesised here; Braillestorm ships no audio files (the
//! approach Brickspire, Panzer and SIGPIPE use).
//!
//! - Effects are rendered once at start-up into sample buffers.
//! - The game asks for sounds as `Sound`s each frame; `play` turns them into
//!   effects, skipping one that played a moment ago, so a bomb clearing forty
//!   enemies is one bang, not forty.
//! - Bursts climb a pentatonic scale with the score multiplier, so a long
//!   streak sounds higher and higher.
//!
//! Without an audio device, or with `--mute`, every call does nothing.

use crate::enemies::Kind;
use rodio::buffer::SamplesBuffer;
use rodio::MixerDeviceSink;
use std::f32::consts::TAU;
use std::num::NonZero;
use std::time::{Duration, Instant};

const SR: u32 = 44_100;
const SRF: f32 = SR as f32;

/// What the game asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sound {
    /// A volley from the twin barrels.
    Fire,
    /// An enemy shot down, at this multiplier.
    Burst(Kind, u32),
    /// A fork split into its two children.
    Split,
    /// A shot struck something that takes more than one.
    Dent,
    /// An enemy has begun fading in.
    Arrive,
    Bomb,
    Lost,
    Back,
    Wave,
    Rush,
    /// The multiplier went up.
    Up,
    /// A pick-up taken, and one running out.
    PowerUp,
    PowerDown,
    ExtraLife,
    GameOver,
}

/// Effects, in the bank's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sfx {
    Fire,
    Glob,
    Cursor,
    Fork,
    Child,
    Spinner,
    Null,
    Split,
    Dent,
    Arrive,
    Bomb,
    Lost,
    Back,
    Wave,
    Rush,
    Up,
    PowerUp,
    PowerDown,
    ExtraLife,
    GameOver,
}

const ALL_SFX: [Sfx; 20] = [
    Sfx::Fire,
    Sfx::Glob,
    Sfx::Cursor,
    Sfx::Fork,
    Sfx::Child,
    Sfx::Spinner,
    Sfx::Null,
    Sfx::Split,
    Sfx::Dent,
    Sfx::Arrive,
    Sfx::Bomb,
    Sfx::Lost,
    Sfx::Back,
    Sfx::Wave,
    Sfx::Rush,
    Sfx::Up,
    Sfx::PowerUp,
    Sfx::PowerDown,
    Sfx::ExtraLife,
    Sfx::GameOver,
];

/// The burst's ping, one note per multiplier step: a pentatonic scale.
const SCALE: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
const NOTES: usize = 10;

pub struct Audio {
    // the sink must outlive everything playing on its mixer
    sink: Option<MixerDeviceSink>,
    bank: Vec<SamplesBuffer>,
    notes: Vec<SamplesBuffer>,
    pub muted: bool,
    last: [Option<Instant>; ALL_SFX.len()],
    last_note: Option<Instant>,
}

impl Audio {
    pub fn new(muted: bool) -> Audio {
        let sink = if muted {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        let (bank, notes) = if sink.is_some() {
            (ALL_SFX.iter().map(|&s| buffer(render_sfx(s))).collect(), (0..NOTES).map(|i| buffer(render_note(i))).collect())
        } else {
            (Vec::new(), Vec::new())
        };
        log::info!("audio {}", if sink.is_some() { "on" } else { "off" });
        Audio { sink, bank, notes, muted, last: [None; ALL_SFX.len()], last_note: None }
    }

    /// Play what the game asked for.
    pub fn play(&mut self, sound: Sound) {
        if self.muted || self.sink.is_none() {
            return;
        }
        let now = Instant::now();
        let sfx = match sound {
            Sound::Burst(kind, mult) => {
                // a ping up the scale with the multiplier, under the burst
                if mult > 1 && self.last_note.is_none_or(|t| now.duration_since(t) >= Duration::from_millis(60)) {
                    self.last_note = Some(now);
                    let i = (mult as usize - 2).min(NOTES - 1);
                    self.add(self.notes[i].clone());
                }
                match kind {
                    Kind::Glob => Sfx::Glob,
                    Kind::Cursor => Sfx::Cursor,
                    Kind::Fork => Sfx::Fork,
                    Kind::Child => Sfx::Child,
                    Kind::Spinner => Sfx::Spinner,
                    Kind::Null => Sfx::Null,
                }
            }
            Sound::Fire => Sfx::Fire,
            Sound::Split => Sfx::Split,
            Sound::Dent => Sfx::Dent,
            Sound::Arrive => Sfx::Arrive,
            Sound::Bomb => Sfx::Bomb,
            Sound::Lost => Sfx::Lost,
            Sound::Back => Sfx::Back,
            Sound::Wave => Sfx::Wave,
            Sound::Rush => Sfx::Rush,
            Sound::Up => Sfx::Up,
            Sound::PowerUp => Sfx::PowerUp,
            Sound::PowerDown => Sfx::PowerDown,
            Sound::ExtraLife => Sfx::ExtraLife,
            Sound::GameOver => Sfx::GameOver,
        };
        let i = ALL_SFX.iter().position(|&s| s == sfx).unwrap_or(0);
        // the same effect again this soon only muddies it
        let gap = match sfx {
            Sfx::Arrive => 140,
            Sfx::Fire => 60,
            _ => 40,
        };
        if self.last[i].is_some_and(|t| now.duration_since(t) < Duration::from_millis(gap)) {
            return;
        }
        self.last[i] = Some(now);
        self.add(self.bank[i].clone());
    }

    fn add(&self, b: SamplesBuffer) {
        if let Some(sink) = &self.sink {
            sink.mixer().add(b);
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        log::info!("muted {}", self.muted);
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }
}

fn buffer(samples: Vec<f32>) -> SamplesBuffer {
    SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(SR).unwrap(), samples)
}

// ------------------------------------------------------------------ helpers

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn sine(ph: f32) -> f32 {
    (ph * TAU).sin()
}

fn square(ph: f32) -> f32 {
    if ph - ph.floor() < 0.5 { 1.0 } else { -1.0 }
}

fn tri(ph: f32) -> f32 {
    let p = ph - ph.floor();
    4.0 * (p - 0.5).abs() - 1.0
}

fn saw(ph: f32) -> f32 {
    2.0 * (ph - ph.floor()) - 1.0
}

/// `dur` seconds of samples from `f(t, noise)`.
fn synth(dur: f32, seed: u32, mut f: impl FnMut(f32, &mut Noise) -> f32) -> Vec<f32> {
    let mut n = Noise(seed);
    (0..(dur * SRF) as usize).map(|i| f(i as f32 / SRF, &mut n)).collect()
}

/// A little tune: (midi note, seconds), played on `wave` with a soft attack.
fn tune(notes: &[(f32, f32)], gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for &(n, d) in notes {
        let len = (d * SRF) as usize;
        for i in 0..len {
            let t = i as f32 / SRF;
            if n > 0.0 {
                ph += midi(n) / SRF;
            }
            let env = (t / 0.008).min(1.0) * ((d - t) / 0.03).clamp(0.0, 1.0);
            let x = if n > 0.0 { wave(ph) * env } else { 0.0 };
            lp += (x - lp) * 0.3;
            out.push(lp * gain);
        }
    }
    out
}

/// Several sounds played together.
fn chord(parts: &[Vec<f32>]) -> Vec<f32> {
    let len = parts.iter().map(Vec::len).max().unwrap_or(0);
    (0..len).map(|i| parts.iter().map(|p| p.get(i).copied().unwrap_or(0.0)).sum()).collect()
}

/// Noise through a low-pass whose cutoff sweeps from `from` to `to` Hz.
fn rumble(dur: f32, from: f32, to: f32, gain: f32, seed: u32) -> Vec<f32> {
    let mut n = Noise(seed);
    let mut lp = 0.0f32;
    let mut lp2 = 0.0f32;
    (0..(dur * SRF) as usize)
        .map(|i| {
            let t = i as f32 / SRF;
            let k = t / dur;
            let c = (TAU * (from + (to - from) * k) / SRF).min(0.9);
            lp += (n.next() - lp) * c;
            lp2 += (lp - lp2) * c;
            lp2 * (t / 0.004).min(1.0) * (1.0 - k).powf(1.5) * gain
        })
        .collect()
}

/// A pitch gliding from `from` to `to` Hz on `wave`, fading out.
fn glide(dur: f32, from: f32, to: f32, gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut ph = 0.0f32;
    synth(dur, 1, |t, _| {
        let k = t / dur;
        // exponential, so the ear hears an even sweep
        ph += from * (to / from).powf(k) / SRF;
        wave(ph) * (t / 0.003).min(1.0) * (1.0 - k) * gain
    })
}

/// A tone stepped down to `bits` levels and held for `hold` samples at a
/// time: the gritty, computer-ish edge the terminal enemies have.
fn crush(v: Vec<f32>, bits: f32, hold: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(v.len());
    let mut held = 0.0;
    for (i, x) in v.into_iter().enumerate() {
        if i % hold == 0 {
            held = (x * bits).round() / bits;
        }
        out.push(held);
    }
    out
}

fn render_note(i: usize) -> Vec<f32> {
    let n = 76.0 + SCALE[i % 5] + 12.0 * (i / 5) as f32;
    let f = midi(n);
    synth(0.16, 3, |t, _| {
        let env = (t / 0.002).min(1.0) * (-t * 26.0).exp();
        ((TAU * f * t).sin() * 0.7 + (TAU * 2.0 * f * t).sin() * 0.12) * env * 0.09
    })
}

fn render_sfx(s: Sfx) -> Vec<f32> {
    match s {
        // a soft, quick zip; it plays a dozen times a second, so it stays low
        Sfx::Fire => synth(0.05, 7, |t, n| {
            let k = t / 0.05;
            let f = 2600.0 * (1.0 - k * 0.6);
            ((TAU * f * t).sin() * 0.5 + n.next() * 0.25) * (t / 0.002).min(1.0) * (1.0 - k).powi(2) * 0.045
        }),
        // a glob pops: a round thump with a little fizz
        Sfx::Glob => chord(&[glide(0.12, 420.0, 120.0, 0.14, sine), rumble(0.1, 5000.0, 900.0, 0.22, 11)]),
        // a cursor blips out, stepped like a bit pattern going blank
        Sfx::Cursor => crush(glide(0.13, 1400.0, 260.0, 0.16, square), 6.0, 24),
        // a fork cracks apart
        Sfx::Fork => chord(&[rumble(0.18, 7000.0, 600.0, 0.45, 13), glide(0.18, 300.0, 90.0, 0.16, tri)]),
        Sfx::Child => glide(0.07, 1800.0, 600.0, 0.1, tri),
        // a spinner winds down
        Sfx::Spinner => crush(glide(0.25, 900.0, 140.0, 0.15, saw), 8.0, 16),
        // the null device collapses: a deep implosion, then silence rushing in
        Sfx::Null => chord(&[glide(0.9, 55.0, 25.0, 0.45, sine), rumble(0.9, 300.0, 4000.0, 0.7, 17), glide(0.5, 2400.0, 200.0, 0.05, sine)]),
        Sfx::Split => synth(0.08, 19, |t, n| n.next() * (-t * 60.0).exp() * 0.2),
        // a hit on something that takes more: a dull knock
        Sfx::Dent => synth(0.06, 23, |t, _| ((TAU * 150.0 * t).sin() * 0.8 + (TAU * 330.0 * t).sin() * 0.2) * (-t * 70.0).exp() * 0.2),
        // something fading in: a faint rising shimmer
        Sfx::Arrive => glide(0.18, 500.0, 1500.0, 0.03, tri),
        // a bomb: a hard crack, a low boom and a sweep down after the ring
        Sfx::Bomb => {
            let crack = synth(0.08, 29, |t, n| n.next() * (-t * 45.0).exp() * 0.35);
            chord(&[crack, rumble(1.5, 2500.0, 60.0, 1.0, 31), glide(1.2, 180.0, 35.0, 0.3, sine)])
        }
        // the ship goes: a long burst and a falling cry
        Sfx::Lost => chord(&[rumble(1.6, 4000.0, 80.0, 0.8, 37), glide(1.1, 880.0, 70.0, 0.08, square), glide(0.7, 120.0, 40.0, 0.22, sine)]),
        // back on the field: a rising shimmer
        Sfx::Back => chord(&[glide(0.6, 300.0, 1800.0, 0.07, tri), glide(0.6, 450.0, 2700.0, 0.04, sine)]),
        // a wave begins: two bright notes as the banner types
        Sfx::Wave => chord(&[tune(&[(76.0, 0.09), (83.0, 0.3)], 0.12, tri), tune(&[(0.0, 0.09), (71.0, 0.3)], 0.06, square)]),
        // a rush: three warning pulses
        Sfx::Rush => tune(&[(57.0, 0.12), (0.0, 0.06), (57.0, 0.12), (0.0, 0.06), (57.0, 0.12), (69.0, 0.35)], 0.14, saw),
        Sfx::Up => tune(&[(84.0, 0.05), (91.0, 0.12)], 0.07, square),
        // a pick-up: a quick bright run up and a shimmer over it
        Sfx::PowerUp => chord(&[tune(&[(79.0, 0.04), (83.0, 0.04), (86.0, 0.04), (91.0, 0.16)], 0.12, square), glide(0.3, 1200.0, 2400.0, 0.04, sine)]),
        // one running out: the same, falling and softer
        Sfx::PowerDown => tune(&[(86.0, 0.05), (79.0, 0.05), (74.0, 0.12)], 0.07, tri),
        Sfx::ExtraLife => tune(&[(72.0, 0.07), (76.0, 0.07), (79.0, 0.07), (84.0, 0.07), (88.0, 0.25)], 0.13, square),
        Sfx::GameOver => tune(&[(64.0, 0.3), (60.0, 0.3), (57.0, 0.3), (52.0, 0.9)], 0.15, tri),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_render_and_stay_in_range() {
        for s in ALL_SFX {
            let v = render_sfx(s);
            assert!(!v.is_empty(), "{s:?} is empty");
            assert!(v.iter().all(|x| x.is_finite()), "{s:?} not finite");
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.01 && peak <= 1.0, "{s:?} peak {peak}");
        }
        for i in 0..NOTES {
            let peak = render_note(i).iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.02 && peak <= 1.0, "note {i} peak {peak}");
        }
    }
}
