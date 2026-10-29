//! Music, synthesised like the effects: one endless source on the mixer,
//! steered every frame by the game (as Kaiku and Ceres do theirs).
//!
//! - **The waves drive it.** A minor-key pulse (i, VI, III, VII) whose tempo
//!   climbs from 112 to 140 bpm as the waves go by.
//! - **The crowd is the arrangement.** Bass and kick always; hats from the
//!   second wave; an arpeggio and a snare come in as the field fills up.
//! - **Each stage has its key,** moved on with the grid's colour.
//! - **Between lives, in pause and after the game** it drops to a soft pad.

use std::sync::{Arc, Mutex};

pub const SR: u32 = 44_100;
const SRF: f32 = SR as f32;

/// What the game tells the music, every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ctrl {
    /// how far into the game: 0 at the first wave, 1 by the thirtieth
    pub heat: f32,
    /// how full the field is, 0..1
    pub crowd: f32,
    /// semitones off A minor, one key per stage
    pub key: i32,
    /// the ship is flying (not lost, paused or over)
    pub playing: bool,
    pub muted: bool,
}

/// The progression, a bar each: roots as MIDI notes and the chord's
/// intervals (minor or major triad).
const BARS: [(i32, [i32; 3]); 4] = [(45, [0, 3, 7]), (41, [0, 4, 7]), (48, [0, 4, 7]), (43, [0, 4, 7])];

/// The keys the stages move through.
pub const STAGE_KEYS: [i32; 4] = [0, 3, -2, 5];

#[derive(Clone, Copy)]
enum Wave {
    Saw,
    Square,
    Tri,
}

struct Voice {
    freq: f32,
    phase: f32,
    wave: Wave,
    vol: f32,
    attack: f32,
    release: f32,
    age: f32,
    /// how much goes to the echo
    send: f32,
}

impl Voice {
    fn env(&self) -> f32 {
        if self.age < self.attack {
            self.age / self.attack
        } else {
            (1.0 - (self.age - self.attack) / self.release).max(0.0)
        }
    }

    fn done(&self) -> bool {
        self.age >= self.attack + self.release
    }

    fn sample(&mut self) -> f32 {
        self.phase = (self.phase + self.freq / SRF).fract();
        let p = self.phase;
        let x = match self.wave {
            Wave::Saw => p * 2.0 - 1.0,
            Wave::Square => if p < 0.5 { 0.6 } else { -0.6 },
            Wave::Tri => 1.0 - 4.0 * (p - 0.5).abs(),
        };
        let v = x * self.vol * self.env();
        self.age += 1.0;
        v
    }
}

fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// The endless music source.
pub struct Band {
    ctrl: Arc<Mutex<Ctrl>>,
    poll: u32,
    want: Ctrl,
    /// smoothed from `want`, so nothing jumps
    heat: f32,
    crowd: f32,
    level: f32,
    /// 1 while flying, easing to 0 for the pad
    drive: f32,
    voices: Vec<Voice>,
    in_step: f32,
    step: u64,
    kick: f32,
    kick_phase: f32,
    hat: f32,
    snare: f32,
    noise: u32,
    /// the bass's low-pass, and the echo line
    lp: f32,
    delay: Vec<f32>,
    at: usize,
}

impl Band {
    pub fn new(ctrl: Arc<Mutex<Ctrl>>) -> Band {
        Band {
            ctrl,
            poll: 0,
            want: Ctrl::default(),
            heat: 0.0,
            crowd: 0.0,
            level: 0.0,
            drive: 0.0,
            voices: Vec::with_capacity(24),
            in_step: 0.0,
            step: 0,
            kick: 0.0,
            kick_phase: 0.0,
            hat: 0.0,
            snare: 0.0,
            noise: 0x1357_9bdf,
            lp: 0.0,
            delay: vec![0.0; SR as usize],
            at: 0,
        }
    }

    fn rand(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// A sixteenth in samples.
    fn step_len(&self) -> f32 {
        let bpm = 112.0 + 28.0 * self.heat;
        SRF * 60.0 / bpm / 4.0
    }

    fn play(&mut self, midi: f32, wave: Wave, vol: f32, attack: f32, release: f32, send: f32) {
        if self.voices.len() >= 24 {
            self.voices.remove(0);
        }
        self.voices.push(Voice { freq: hz(midi), phase: 0.0, wave, vol, attack: attack * SRF, release: release * SRF, age: 0.0, send });
    }

    /// The start of a sixteenth: which notes and drums it brings.
    fn on_step(&mut self) {
        let s = (self.step % 16) as u32;
        let (root, chord) = BARS[(self.step / 16 % 4) as usize];
        let root = (root + self.want.key) as f32;
        let step_s = self.step_len() / SRF;
        let d = self.drive;
        // the pad, every bar: soft between lives, a bed under the drive
        if s == 0 {
            let pad = 0.05 + 0.05 * (1.0 - d);
            for &i in &chord {
                self.play(root + 12.0 + i as f32, Wave::Tri, pad, 0.4, step_s * 16.0, 0.3);
            }
        }
        if d < 0.1 {
            return;
        }
        // bass: eighths on the root, an octave jump on the last of each beat
        if s % 2 == 0 {
            let up = if s % 4 == 2 { 12.0 } else { 0.0 };
            self.play(root - 12.0 + up, Wave::Saw, 0.22 * d, 0.004, step_s * 1.6, 0.0);
        }
        // four on the floor
        if s % 4 == 0 {
            self.kick = d;
            self.kick_phase = 0.0;
        }
        // hats on the off-beats from the second wave, every sixteenth later
        if (self.heat > 0.02 && s % 4 == 2) || (self.heat > 0.4 && s % 2 == 1) {
            self.hat = 0.5 * d;
        }
        // snare on two and four once it is busy
        if (self.crowd > 0.45 || self.heat > 0.5) && (s == 4 || s == 12) {
            self.snare = 0.8 * d;
        }
        // the arpeggio, up the chord and an octave, louder in a crowd
        let arp = (self.crowd - 0.15).max(0.0).min(0.6) / 0.6;
        if arp > 0.0 {
            let n = [0, 1, 2, 3][(s % 4) as usize];
            let note = if n == 3 { chord[0] + 12 } else { chord[n] };
            self.play(root + 24.0 + note as f32, Wave::Square, 0.07 * arp * d, 0.003, step_s * 0.9, 0.5);
        }
    }

    fn sample(&mut self) -> f32 {
        if self.poll == 0 {
            if let Ok(c) = self.ctrl.lock() {
                self.want = *c;
            }
            self.heat += (self.want.heat - self.heat) * 0.01;
            self.crowd += (self.want.crowd - self.crowd) * 0.03;
            let level = if self.want.muted { 0.0 } else { 1.0 };
            self.level += (level - self.level) * 0.05;
            let drive = if self.want.playing { 1.0 } else { 0.0 };
            self.drive += (drive - self.drive) * 0.04;
            self.poll = 512;
        }
        self.poll -= 1;

        self.in_step += 1.0;
        if self.in_step >= self.step_len() {
            self.in_step = 0.0;
            self.on_step();
            self.step += 1;
        }

        let (mut dry, mut send) = (0.0, 0.0);
        let mut bass = 0.0;
        for v in &mut self.voices {
            let x = v.sample();
            if matches!(v.wave, Wave::Saw) {
                bass += x;
            } else {
                dry += x;
                send += x * v.send;
            }
        }
        self.voices.retain(|v| !v.done());
        // the bass through a low-pass, so it thumps rather than buzzes
        self.lp += (bass - self.lp) * 0.08;
        dry += self.lp;

        // kick: a falling sine; hat and snare: noise
        if self.kick > 0.001 {
            let f = 45.0 + 110.0 * self.kick.powi(3);
            self.kick_phase = (self.kick_phase + f / SRF).fract();
            dry += (self.kick_phase * std::f32::consts::TAU).sin() * self.kick * 0.45;
            self.kick *= 0.9994;
        }
        let n = self.rand();
        if self.hat > 0.001 {
            dry += n * self.hat * 0.06;
            self.hat *= 0.9975;
        }
        if self.snare > 0.001 {
            dry += n * self.snare * 0.12;
            self.snare *= 0.9988;
        }

        // a dotted-eighth echo on the arpeggio and pad
        let len = (self.step_len() * 3.0) as usize % self.delay.len();
        let back = (self.at + self.delay.len() - len.max(1)) % self.delay.len();
        let echo = self.delay[back];
        self.delay[self.at] = send + echo * 0.35;
        self.at = (self.at + 1) % self.delay.len();

        ((dry + echo * 0.5) * self.level).clamp(-1.0, 1.0)
    }
}

impl Iterator for Band {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        Some(self.sample())
    }
}

impl rodio::Source for Band {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        std::num::NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        std::num::NonZero::new(SR).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

pub fn shared() -> Arc<Mutex<Ctrl>> {
    Arc::new(Mutex::new(Ctrl::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(ctrl: Ctrl, secs: f32) -> Vec<f32> {
        let mut band = Band::new(Arc::new(Mutex::new(ctrl)));
        // settle the smoothed values first
        band.heat = ctrl.heat;
        band.crowd = ctrl.crowd;
        band.drive = if ctrl.playing { 1.0 } else { 0.0 };
        band.level = if ctrl.muted { 0.0 } else { 1.0 };
        (0..(secs * SRF) as usize).map(|_| band.sample()).collect()
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    #[test]
    fn it_plays_in_range_and_the_crowd_is_louder() {
        for heat in [0.0, 0.5, 1.0] {
            for crowd in [0.0, 1.0] {
                for playing in [false, true] {
                    let s = render(Ctrl { heat, crowd, playing, ..Ctrl::default() }, 3.0);
                    assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
                    assert!(rms(&s) > 0.005, "heat {heat} crowd {crowd} playing {playing}: {}", rms(&s));
                }
            }
        }
        let calm = rms(&render(Ctrl { heat: 0.3, crowd: 0.0, playing: true, ..Ctrl::default() }, 4.0));
        let busy = rms(&render(Ctrl { heat: 0.3, crowd: 1.0, playing: true, ..Ctrl::default() }, 4.0));
        let pad = rms(&render(Ctrl { heat: 0.3, crowd: 1.0, playing: false, ..Ctrl::default() }, 4.0));
        assert!(busy > calm && calm > pad, "busy {busy} calm {calm} pad {pad}");
    }

    #[test]
    fn muted_is_silent() {
        let s = render(Ctrl { heat: 0.5, crowd: 1.0, playing: true, muted: true, ..Ctrl::default() }, 2.0);
        assert!(rms(&s) < 0.001);
    }

    /// Not a check: how much of the audio thread the music needs.
    /// `cargo test --release music_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn music_cost() {
        let t0 = std::time::Instant::now();
        let s = render(Ctrl { heat: 1.0, crowd: 1.0, playing: true, ..Ctrl::default() }, 20.0);
        println!("20 s of music in {:?}, rms {:.3}", t0.elapsed(), rms(&s));
    }
}
