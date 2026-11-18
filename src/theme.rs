//! Design tokens + the theme engine (presets and "Auto", which derives the palette
//! from the backdrop scene). Port of the mock's THEME ENGINE.

use gpui_kit::{App, Global, Hsla, Rgba, hsla, rgba};

pub type Rgb = [f32; 3]; // 0..255

pub fn hex(h: &str) -> Rgb {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32;
    [p(0), p(2), p(4)]
}

pub fn rgb2hsl([r, g, b]: Rgb) -> [f32; 3] {
    let (r, g, b) = (r / 255., g / 255., b / 255.);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let l = (mx + mn) / 2.;
    let d = mx - mn;
    if d == 0. {
        return [0., 0., l];
    }
    let s = d / (1. - (2. * l - 1.).abs());
    let h = if mx == r {
        ((g - b) / d + 6.) % 6.
    } else if mx == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    };
    [h * 60., s, l]
}

pub fn hsl2rgb([h, s, l]: [f32; 3]) -> Rgb {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let x = c * (1. - ((h / 60.) % 2. - 1.).abs());
    let m = l - c / 2.;
    let (r, g, b) = if h < 60. {
        (c, x, 0.)
    } else if h < 120. {
        (x, c, 0.)
    } else if h < 180. {
        (0., c, x)
    } else if h < 240. {
        (0., x, c)
    } else if h < 300. {
        (x, 0., c)
    } else {
        (c, 0., x)
    };
    [((r + m) * 255.).round(), ((g + m) * 255.).round(), ((b + m) * 255.).round()]
}

pub fn rgb_a(c: Rgb, a: f32) -> Hsla {
    Rgba { r: c[0] / 255., g: c[1] / 255., b: c[2] / 255., a }.into()
}
fn hsl_c(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    rgb_a(hsl2rgb([h, s.clamp(0., 1.), l.clamp(0., 1.)]), a)
}

pub fn tune_accent(c: Rgb, light: bool) -> Rgb {
    let [h, s, l] = rgb2hsl(c);
    if s < 0.12 {
        return hsl2rgb([h, s, if light { 0.42 } else { 0.78 }]);
    }
    hsl2rgb([
        h,
        s.clamp(0.5, 0.88),
        if light { l.clamp(0.42, 0.5) } else { l.clamp(0.62, 0.72) },
    ])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub accent: Rgb,
    pub second: Rgb,
    pub tint: (f32, f32),
    pub light: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub light: bool,
    pub bg: Hsla,
    pub bg_glow_1: Hsla,
    pub bg_glow_2: Hsla,
    pub panel: Hsla,
    pub panel_solid: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_active: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_2: Hsla,
    pub text_3: Hsla,
    pub accent: Hsla,
    pub accent_2: Hsla,
    pub green: Hsla,
    pub red: Hsla,
    pub yellow: Hsla,
    pub blue: Hsla,
    pub composer_card: Hsla,
    pub composer_foot: Hsla,
    pub hero_ink: Hsla,
    pub hero_sub: Hsla,
    pub hero_chip: Hsla,
    pub hero_halo: Hsla,
    pub hero_fade: Hsla,
}

fn c(v: u32) -> Hsla {
    rgba(v).into()
}

impl Theme {
    /// The stock "Ember" dark palette from :root.
    pub fn ember(light: bool) -> Self {
        if light {
            Theme {
                light,
                bg: c(0xf5f2efff),
                bg_glow_1: c(0xe2795a2e),
                bg_glow_2: c(0xb478a024),
                panel: c(0xffffffbf),
                panel_solid: c(0xffffffff),
                surface: c(0x00000009),
                surface_hover: c(0x0000000f),
                surface_active: c(0x00000017),
                border: c(0x00000014),
                text: c(0x1f1c1aff),
                text_2: c(0x5b5652ff),
                text_3: c(0x9a948fff),
                accent: c(0xe2795aff),
                accent_2: c(0x8a3a7aff),
                green: c(0x5fc48aff),
                red: c(0xe5675fff),
                yellow: c(0xe2b75aff),
                blue: c(0x7aa7e8ff),
                composer_card: c(0xffffffff),
                composer_foot: c(0xefebe7ff),
                hero_ink: c(0x2a1d22ff),
                hero_sub: c(0x6b5f5aff),
                hero_chip: c(0xffffff99),
                hero_halo: c(0xe2795a1a),
                hero_fade: c(0xf5f2eff2),
            }
        } else {
            Theme {
                light,
                bg: c(0x0d0c0eff),
                bg_glow_1: c(0xa0322847),
                bg_glow_2: c(0x5a1e3c38),
                panel: c(0x161417b8),
                panel_solid: c(0x161417ff),
                surface: c(0xffffff0a),
                surface_hover: c(0xffffff12),
                surface_active: c(0xffffff1a),
                border: c(0xffffff12),
                text: c(0xece9e6ff),
                text_2: c(0xa9a4a0ff),
                text_3: c(0x6f6a67ff),
                accent: c(0xe2795aff),
                accent_2: c(0x8a3a7aff),
                green: c(0x5fc48aff),
                red: c(0xe5675fff),
                yellow: c(0xe2b75aff),
                blue: c(0x7aa7e8ff),
                composer_card: c(0x1e1d20ff),
                composer_foot: c(0x141316ff),
                hero_ink: c(0xf7efe8ff),
                hero_sub: c(0xece4de9e),
                hero_chip: c(0x1c1a1e8c),
                hero_halo: c(0xe2795a1f),
                hero_fade: c(0x161417f2),
            }
        }
    }

    /// tokens() from the mock: derive the palette from an accent + tint.
    pub fn from_spec(sp: Spec) -> Self {
        let light = sp.light;
        let (h, s0) = sp.tint;
        let s = s0.clamp(0.02, 0.35);
        let a = tune_accent(sp.accent, light);
        let a2 = sp.second;
        let mut t = Theme::ember(light);
        if light {
            t.bg = hsl_c(h, s * 0.5, 0.955, 1.);
            t.panel = hsl_c(h, s * 0.4, 0.995, 0.75);
            t.panel_solid = hsl_c(h, s * 0.4, 0.995, 1.);
            t.text = hsl_c(h, 0.18, 0.11, 1.);
            t.text_2 = hsl_c(h, 0.07, 0.35, 1.);
            t.text_3 = hsl_c(h, 0.05, 0.6, 1.);
            t.bg_glow_1 = rgb_a(a, 0.16);
            t.bg_glow_2 = rgb_a(a2, 0.13);
            t.composer_card = hsl_c(h, s * 0.3, 1., 1.);
            t.composer_foot = hsl_c(h, s * 0.45, 0.925, 1.);
            t.hero_ink = hsl_c(h, 0.25, 0.14, 1.);
            t.hero_sub = hsl_c(h, 0.08, 0.38, 1.);
            t.hero_chip = hsl_c(h, 0.3, 0.99, 0.6);
            t.hero_halo = rgb_a(a, 0.1);
            t.hero_fade = hsl_c(h, s * 0.5, 0.955, 0.95);
        } else {
            t.bg = hsl_c(h, s * 0.55, 0.055, 1.);
            t.panel = hsl_c(h, s * 0.45, 0.085, 0.72);
            t.panel_solid = hsl_c(h, s * 0.45, 0.085, 1.);
            t.text = hsl_c(h, 0.14, 0.92, 1.);
            t.text_2 = hsl_c(h, 0.07, 0.66, 1.);
            t.text_3 = hsl_c(h, 0.05, 0.43, 1.);
            t.bg_glow_1 = rgb_a(a, 0.26);
            t.bg_glow_2 = rgb_a(a2, 0.2);
            t.composer_card = hsl_c(h, s * 0.4, 0.12, 1.);
            t.composer_foot = hsl_c(h, s * 0.45, 0.08, 1.);
            t.hero_ink = hsl_c(h, 0.3, 0.96, 1.);
            t.hero_sub = hsl_c(h, 0.12, 0.9, 0.62);
            t.hero_chip = hsl_c(h, s * 0.4, 0.11, 0.55);
            t.hero_halo = rgb_a(a, 0.12);
            t.hero_fade = hsl_c(h, s * 0.45, 0.085, 0.95);
        }
        t.accent = rgb_a(a, 1.);
        t.accent_2 = rgb_a(a2, 1.);
        t
    }

    /// Linear blend between two themes — drives the 0.9s colour transition.
    pub fn lerp(&self, o: &Theme, f: f32) -> Theme {
        fn m(a: Hsla, b: Hsla, f: f32) -> Hsla {
            let (ra, rb) = (Rgba::from(a), Rgba::from(b));
            Rgba {
                r: ra.r + (rb.r - ra.r) * f,
                g: ra.g + (rb.g - ra.g) * f,
                b: ra.b + (rb.b - ra.b) * f,
                a: ra.a + (rb.a - ra.a) * f,
            }
            .into()
        }
        macro_rules! l { ($($f:ident),*) => { Theme { light: if f < 0.5 { self.light } else { o.light }, $($f: m(self.$f, o.$f, f)),* } } }
        l!(bg, bg_glow_1, bg_glow_2, panel, panel_solid, surface, surface_hover, surface_active, border, text, text_2, text_3, accent, accent_2, green, red, yellow, blue, composer_card, composer_foot, hero_ink, hero_sub, hero_chip, hero_halo, hero_fade)
    }
}

pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub accent: &'static str,
    pub second: &'static str,
    pub tint: (f32, f32),
}

pub const PRESETS: &[Preset] = &[
    Preset { id: "ember", name: "Ember", accent: "#e2795a", second: "#8a3a7a", tint: (350., 0.10) },
    Preset { id: "tako", name: "Tako", accent: "#6f9bff", second: "#2f5fd6", tint: (222., 0.26) },
    Preset { id: "tide", name: "Tide", accent: "#4fb3c8", second: "#3a5aa8", tint: (205., 0.22) },
    Preset { id: "moss", name: "Moss", accent: "#7cc56a", second: "#2f7a5a", tint: (140., 0.16) },
    Preset { id: "iris", name: "Iris", accent: "#9b87f5", second: "#5a3aa8", tint: (255., 0.22) },
    Preset { id: "rose", name: "Rose", accent: "#ec7aa0", second: "#8a3a6a", tint: (335., 0.2) },
    Preset { id: "reef", name: "Reef", accent: "#ff8f6b", second: "#1f86b8", tint: (198., 0.26) },
    Preset { id: "mono", name: "Mono", accent: "#cfc9c1", second: "#5a5652", tint: (30., 0.03) },
];

/// Theme state: which palette is picked, the last Auto extraction, and the
/// animated "current" theme (blends from `from` to `target` over 0.9s).
pub struct ThemeState {
    pub pal: String, // "auto" | preset id
    pub light: bool,
    pub auto: Option<Spec>,
    pub current: Theme,
    from: Theme,
    target: Theme,
    started: Option<std::time::Instant>,
}

impl Global for ThemeState {}

impl ThemeState {
    pub fn new() -> Self {
        // Default the Auto extraction to what the Tako scene yields in the mock.
        let auto = Some(Spec { accent: [101., 147., 216.], second: [51., 29., 113.], tint: (214., 0.3), light: false });
        let t = Theme::from_spec(auto.unwrap());
        ThemeState { pal: "auto".into(), light: false, auto, current: t, from: t, target: t, started: None }
    }
    pub fn spec(&self) -> Option<Spec> {
        if self.pal == "auto" {
            return self.auto;
        }
        if self.pal == "ember" {
            return None;
        }
        PRESETS.iter().find(|p| p.id == self.pal).map(|p| Spec { accent: hex(p.accent), second: hex(p.second), tint: p.tint, light: self.light })
    }
    pub fn apply(&mut self) {
        let t = match self.spec() {
            Some(mut s) => {
                if self.pal != "auto" {
                    s.light = self.light;
                }
                Theme::from_spec(s)
            }
            None => Theme::ember(self.light),
        };
        self.from = self.current;
        self.target = t;
        self.started = Some(std::time::Instant::now());
    }
    /// Advance the colour transition; returns true while animating.
    pub fn tick(&mut self) -> bool {
        let Some(st) = self.started else { return false };
        let p = (st.elapsed().as_secs_f32() / 0.9).min(1.);
        let e = p * p * (3. - 2. * p);
        self.current = self.from.lerp(&self.target, e);
        if p >= 1. {
            self.started = None;
        }
        true
    }
    pub fn set_auto(&mut self, s: Spec) {
        if self.auto != Some(s) {
            self.auto = Some(s);
            if self.pal == "auto" {
                self.apply();
            }
        }
    }
}

pub trait ActiveTheme {
    fn t(&self) -> Theme;
}
impl ActiveTheme for App {
    fn t(&self) -> Theme {
        self.global::<ThemeState>().current
    }
}

#[allow(dead_code)]
pub fn alpha(c: Hsla, a: f32) -> Hsla {
    hsla(c.h, c.s, c.l, c.a * a)
}
