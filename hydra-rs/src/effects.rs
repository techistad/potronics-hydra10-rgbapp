use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::device::Rgb;
use crate::layout::{key_by_id, KeyDef, KEYS, LED_COUNT};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Solid,
    Gradient,
    Rainbow,
    Wave,
    Breathing,
    Reactive,
    Ripple,
    Paint,
    Afterglow,
    Scan,
    Pulse,
    Sparkle,
}

impl Mode {
    pub const ALL: [Mode; 12] = [
        Mode::Solid,
        Mode::Gradient,
        Mode::Rainbow,
        Mode::Wave,
        Mode::Breathing,
        Mode::Reactive,
        Mode::Ripple,
        Mode::Paint,
        Mode::Afterglow,
        Mode::Scan,
        Mode::Pulse,
        Mode::Sparkle,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Solid => "Solid",
            Mode::Gradient => "Gradient",
            Mode::Rainbow => "Rainbow",
            Mode::Wave => "Wave",
            Mode::Breathing => "Breathing",
            Mode::Reactive => "Reactive",
            Mode::Ripple => "Ripple",
            Mode::Paint => "Per-key paint",
            Mode::Afterglow => "Afterglow",
            Mode::Scan => "Scan",
            Mode::Pulse => "Pulse",
            Mode::Sparkle => "Sparkle",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Mode::Solid => "Any single color, not just the preset palette.",
            Mode::Gradient => "Blend two colors across the board.",
            Mode::Rainbow => "A built-in spectrum. It does not use a color you pick.",
            Mode::Wave => "A brightness wave moving through your color.",
            Mode::Breathing => "One color that fades in and out.",
            Mode::Reactive => "Keys flare in your color when you type.",
            Mode::Ripple => "Each keypress sends a ring across the board.",
            Mode::Paint => "The glowing keys are chosen on the virtual keyboard.",
            Mode::Afterglow => "Press a key and it flares, then fades back off.",
            Mode::Scan => "A bright band travels across the keys.",
            Mode::Pulse => "Rings expand from the middle of the board.",
            Mode::Sparkle => "Random keys flash, then settle back.",
        }
    }

    pub fn uses_primary(self) -> bool {
        !matches!(self, Mode::Rainbow)
    }

    pub fn uses_second_color(self) -> bool {
        matches!(self, Mode::Gradient | Mode::Wave | Mode::Ripple | Mode::Scan | Mode::Pulse | Mode::Afterglow)
    }

    pub fn uses_speed(self) -> bool {
        !matches!(self, Mode::Solid | Mode::Paint)
    }

    pub fn uses_rest_glow(self) -> bool {
        matches!(self, Mode::Reactive | Mode::Ripple)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomPattern {
    Spread,
    Cycle,
    Blocks,
    Chase,
}

impl Default for CustomPattern {
    fn default() -> Self {
        Self::Spread
    }
}

impl CustomPattern {
    pub const ALL: [CustomPattern; 4] = [
        CustomPattern::Spread,
        CustomPattern::Cycle,
        CustomPattern::Blocks,
        CustomPattern::Chase,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CustomPattern::Spread => "Spread",
            CustomPattern::Cycle => "Cycle",
            CustomPattern::Blocks => "Blocks",
            CustomPattern::Chase => "Chase",
        }
    }

    pub fn uses_speed(self) -> bool {
        matches!(self, CustomPattern::Cycle | CustomPattern::Chase)
    }

    pub fn blurb(self) -> &'static str {
        match self {
            CustomPattern::Spread => "Your five colors blend from left to right. This pattern stays still.",
            CustomPattern::Cycle => "Your five colors scroll across the keys.",
            CustomPattern::Blocks => "Each color owns a band of keys. This pattern stays still.",
            CustomPattern::Chase => "A bright band runs through your five colors.",
        }
    }
}

pub fn palette_from(primary: Rgb, secondary: Rgb) -> [Rgb; 5] {
    [
        primary,
        mix(primary, secondary, 0.45),
        secondary,
        mix(secondary, [0xbf, 0x5a, 0xff], 0.65),
        [0xff, 0xc4, 0x3a],
    ]
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub mode: Mode,
    pub primary: Rgb,
    pub secondary: Rgb,
    pub brightness: f32,
    pub speed: f32,
    pub base_level: f32,
    pub paint: HashMap<String, Rgb>,
}

pub struct EffectEngine {
    pub mode: Mode,
    pub primary: Rgb,
    pub secondary: Rgb,
    pub brightness: f32,
    pub speed: f32,
    pub base_level: f32,
    pub paint: HashMap<String, Rgb>,
    pub custom_on: bool,
    pub palette: [Rgb; 5],
    pub pattern: CustomPattern,
    pressed: HashMap<String, f32>,
    ripples: Vec<(f32, f32, f32)>,
    started: Instant,
}

impl EffectEngine {
    pub fn new() -> Self {
        Self {
            mode: Mode::Reactive,
            primary: [0x2e, 0xe6, 0xc7],
            secondary: [0xff, 0x5a, 0x36],
            brightness: 0.85,
            speed: 0.55,
            base_level: 0.12,
            paint: HashMap::new(),
            custom_on: false,
            palette: palette_from([0x2e, 0xe6, 0xc7], [0xff, 0x5a, 0x36]),
            pattern: CustomPattern::Cycle,
            pressed: HashMap::new(),
            ripples: Vec::new(),
            started: Instant::now(),
        }
    }

    pub fn snapshot(&self) -> Profile {
        Profile {
            mode: self.mode,
            primary: self.primary,
            secondary: self.secondary,
            brightness: self.brightness,
            speed: self.speed,
            base_level: self.base_level,
            paint: self.paint.clone(),
        }
    }

    pub fn apply(&mut self, profile: Profile) {
        self.mode = profile.mode;
        self.primary = profile.primary;
        self.secondary = profile.secondary;
        self.brightness = profile.brightness.clamp(0.0, 1.0);
        self.speed = profile.speed.clamp(0.05, 1.0);
        self.base_level = profile.base_level.clamp(0.0, 1.0);
        self.paint = profile
            .paint
            .into_iter()
            .filter(|(id, _)| key_by_id(id).is_some())
            .collect();
        self.custom_on = false;
    }

    pub fn begin_custom(&mut self, palette: [Rgb; 5], pattern: CustomPattern) {
        self.custom_on = true;
        self.palette = palette;
        self.pattern = pattern;
        self.primary = palette[0];
        self.secondary = palette[2];
    }

    pub fn note_press(&mut self, key_id: &str) {
        let Some(key) = key_by_id(key_id) else {
            return;
        };
        let now = self.started.elapsed().as_secs_f32();
        self.pressed.insert(key_id.to_string(), now);
        self.ripples.push((key.col, key.row as f32, now));
        if self.ripples.len() > 24 {
            let extra = self.ripples.len() - 24;
            self.ripples.drain(0..extra);
        }
    }

    pub fn frame(&mut self) -> (Vec<Rgb>, HashMap<String, Rgb>) {
        let now = self.started.elapsed().as_secs_f32();
        let tempo = 0.35 + self.speed * 2.4;
        self.ripples.retain(|ripple| now - ripple.2 <= 1.6);
        self.pressed.retain(|_, pressed_at| now - *pressed_at <= 3.0);

        let mut colors = HashMap::new();
        for key in KEYS {
            let painted = self.paint_key(key, now, tempo);
            colors.insert(key.id.to_string(), scale(painted, self.brightness));
        }

        let mut leds = vec![[0, 0, 0]; LED_COUNT];
        for key in KEYS {
            if let Some(color) = colors.get(key.id) {
                if key.led < leds.len() {
                    leds[key.led] = *color;
                }
            }
        }
        (leds, colors)
    }

    fn paint_key(&mut self, key: &KeyDef, now: f32, tempo: f32) -> Rgb {
        if self.custom_on {
            return self.paint_custom(key, now, tempo);
        }
        let primary = self.primary;
        let secondary = self.secondary;
        match self.mode {
            Mode::Solid => primary,
            Mode::Gradient => mix(primary, secondary, key.col / 14.0),
            Mode::Rainbow => hsv(key.col / 16.0 + key.row as f32 * 0.04 + now * tempo * 0.15, 1.0, 1.0),
            Mode::Wave => {
                let tint = mix(
                    primary,
                    secondary,
                    0.5 + 0.5 * (key.col * 0.55 - now * tempo * 3.0).sin(),
                );
                let amount = 0.25 + 0.75 * (0.5 + 0.5 * (key.col * 0.7 + key.row as f32 * 0.4 - now * tempo * 4.0).sin());
                scale(tint, amount)
            }
            Mode::Breathing => scale(primary, 0.15 + 0.85 * (0.5 + 0.5 * (now * tempo * 2.2).sin())),
            Mode::Reactive => self.reactive(key, now, primary),
            Mode::Ripple => self.ripple(key, now, primary, secondary),
            Mode::Paint => self.paint.get(key.id).copied().unwrap_or([0, 0, 0]),
            Mode::Afterglow => self.afterglow(key, now, primary, secondary),
            Mode::Scan => {
                let pos = (now * tempo * 2.2).rem_euclid(18.0) - 1.0;
                let band = (-(key.col - pos).powi(2) / 1.1).exp();
                mix(scale(primary, 0.12), secondary, band)
            }
            Mode::Pulse => {
                let dist = ((key.col - 8.0).powi(2) + ((key.row as f32 - 2.0) * 1.35).powi(2)).sqrt();
                let wave = 0.5 + 0.5 * (dist * 0.65 - now * tempo * 3.2).sin();
                scale(mix(primary, secondary, (dist / 12.0).min(1.0)), 0.18 + 0.82 * wave)
            }
            Mode::Sparkle => {
                let bucket = (now * tempo * 3.0) as i32;
                if spark_on(key.led, bucket) {
                    primary
                } else {
                    scale(primary, 0.07)
                }
            }
        }
    }

    fn paint_custom(&self, key: &KeyDef, now: f32, tempo: f32) -> Rgb {
        let colors = &self.palette;
        let count = colors.len() as f32;
        match self.pattern {
            CustomPattern::Spread => {
                let pos = (key.col / 15.0).clamp(0.0, 0.999) * (count - 1.0);
                let index = pos.floor() as usize;
                mix(colors[index], colors[(index + 1).min(colors.len() - 1)], pos - index as f32)
            }
            CustomPattern::Blocks => {
                let index = ((key.col / 16.0) * count).floor() as usize;
                colors[index.min(colors.len() - 1)]
            }
            CustomPattern::Cycle => {
                let pos = (key.col * 0.42 + now * tempo * 1.3).rem_euclid(count);
                let index = pos.floor() as usize % colors.len();
                let next = (index + 1) % colors.len();
                mix(colors[index], colors[next], pos - pos.floor())
            }
            CustomPattern::Chase => {
                let head = (now * tempo * 2.4).rem_euclid(16.0);
                let band = (-(key.col - head).powi(2) / 2.4).exp();
                let index = ((key.col / 16.0) * count).floor() as usize % colors.len();
                let next = (index + 1) % colors.len();
                mix(scale(colors[index], 0.16), colors[next], band)
            }
        }
    }

    fn afterglow(&self, key: &KeyDef, now: f32, primary: Rgb, secondary: Rgb) -> Rgb {
        let life = 0.28 + (1.0 - self.speed) * 1.15;
        let mut best = 0.0;
        let mut age_at = 0.0;
        for (id, pressed_at) in &self.pressed {
            let Some(source) = key_by_id(id) else {
                continue;
            };
            let age = now - pressed_at;
            if age < 0.0 || age > life {
                continue;
            }
            let dist = ((key.col - source.col).powi(2) + ((key.row as f32 - source.row as f32) * 1.45).powi(2)).sqrt();
            let reach = if dist < 0.15 {
                1.0
            } else {
                (1.0 - (dist - 0.15) / 1.7).clamp(0.0, 0.4)
            };
            let fade = (1.0 - age / life).powf(1.5);
            let amount = fade * reach;
            if amount > best {
                best = amount;
                age_at = age / life;
            }
        }
        if best < 0.02 {
            return [0, 0, 0];
        }
        scale(mix(primary, secondary, age_at), best)
    }

    fn reactive(&mut self, key: &KeyDef, now: f32, primary: Rgb) -> Rgb {
        let mut energy = 0.0;
        if let Some(pressed_at) = self.pressed.get(key.id).copied() {
            let age = now - pressed_at;
            energy = (-age * (1.4 + (1.0 - self.speed) * 4.5)).exp();
            if energy < 0.02 {
                self.pressed.remove(key.id);
                energy = 0.0;
            }
        }
        mix(scale(primary, self.base_level), primary, energy)
    }

    fn ripple(&self, key: &KeyDef, now: f32, primary: Rgb, secondary: Rgb) -> Rgb {
        let mut energy = self.base_level * 0.35;
        let mut tint = primary;
        for (col, row, started) in &self.ripples {
            let age = now - started;
            let radius = age * (4.5 + self.speed * 8.0);
            let dist = ((key.col - col).powi(2) + ((key.row as f32 - row) * 1.35).powi(2)).sqrt();
            let band = (-((dist - radius).powi(2)) / 0.55).exp() * (-age * 1.6).exp();
            if band > energy {
                energy = band;
                tint = mix(primary, secondary, age.min(1.0));
            }
        }
        scale(tint, (self.base_level * 0.45).max(energy.min(1.0)))
    }
}

fn spark_on(led: usize, bucket: i32) -> bool {
    let mut n = (led as u32).wrapping_mul(374761393) ^ (bucket as u32).wrapping_mul(668265263);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n >> 24) % 9 == 0
}

fn scale(color: Rgb, amount: f32) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    color.map(|channel| (channel as f32 * amount) as u8)
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0);
    let sector = (h * 6.0).floor();
    let f = h * 6.0 - sector;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match sector as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}
