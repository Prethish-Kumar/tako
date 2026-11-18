//! VALLEY — the original dusk / golden-morning lake (theme aware).
//!
//! The mock picks the dark (dusk) or light (golden morning) palette from the document
//! theme at build time. `Scene::build` gets no `cx`, so the palette comes from
//! [`set_light`]; whoever owns the theme can call it and rebuild the scene.
use super::Scene;
use super::px::*;
use std::cell::Cell;
use std::rc::Rc;

thread_local!(static LIGHT: Cell<bool> = const { Cell::new(false) });

/// Choose the palette used by the next Valley build (the mock's `data-theme === "light"`).
pub fn set_light(light: bool) {
    LIGHT.with(|l| l.set(light));
}
pub fn is_light() -> bool {
    LIGHT.with(|l| l.get())
}

struct RangeP {
    top: &'static str,
    bot: &'static str,
    rim: &'static str,
    snow: Option<&'static str>,
    haze: &'static str,
}
struct Pal {
    sky: &'static [(f64, &'static str)],
    sun: &'static str,
    glow: &'static str,
    star: &'static str,
    stars: bool,
    cloud: [&'static str; 3],
    ranges: [RangeP; 3],
    treeline: &'static str,
    tree: &'static str,
    tree_rim: &'static str,
    water: &'static str,
    water_alpha: f64,
    sparkle: &'static str,
    bird: &'static str,
    fly: &'static str,
}

static DARK: Pal = Pal {
    sky: &[(0., "#1a0f24"), (0.32, "#3b1838"), (0.58, "#7a2a45"), (0.8, "#c8504a"), (0.93, "#f08a5a"), (1., "#ffc58a")],
    sun: "#ffe6b8",
    glow: "#ff9a6a",
    star: "#ffeedd",
    stars: true,
    cloud: ["#ffb48a", "#b4506a", "#6e2c50"],
    ranges: [
        RangeP { top: "#b5506e", bot: "#8a3d62", rim: "#ffb08a", snow: Some("#f6c3b0"), haze: "#d0645e" },
        RangeP { top: "#76305a", bot: "#5a2549", rim: "#ec8a6c", snow: None, haze: "#a54456" },
        RangeP { top: "#3f1a3a", bot: "#2d1330", rim: "#c0605a", snow: None, haze: "#6a2842" },
    ],
    treeline: "#22101f",
    tree: "#140912",
    tree_rim: "#6a2638",
    water: "#1e0f22",
    water_alpha: 0.5,
    sparkle: "#ffd9a8",
    bird: "#2a1228",
    fly: "#ffd79a",
};
static LIGHT_P: Pal = Pal {
    sky: &[(0., "#e9a6a0"), (0.35, "#f4bfa6"), (0.65, "#fbd8b6"), (0.88, "#fee9cb"), (1., "#fff5e3")],
    sun: "#fffdf6",
    glow: "#ffe2b8",
    star: "#ffffff",
    stars: false,
    cloud: ["#fffaf3", "#f8d9cc", "#eebfb6"],
    ranges: [
        RangeP { top: "#d79aac", bot: "#c98a9f", rim: "#fff0e6", snow: Some("#fff8f3"), haze: "#f6cdbd" },
        RangeP { top: "#b0708f", bot: "#9a5f80", rim: "#ffd8c8", snow: None, haze: "#e3a9a8" },
        RangeP { top: "#7c4568", bot: "#683a5a", rim: "#f0b0a0", snow: None, haze: "#c08390" },
    ],
    treeline: "#4d2843",
    tree: "#3d2035",
    tree_rim: "#9a5468",
    water: "#a86a82",
    water_alpha: 0.35,
    sparkle: "#ffffff",
    bird: "#6a3550",
    fly: "#fff6e0",
};

struct RangeO {
    seed: u32,
    step: f64,
    sharp: f64,
    min_h: f64,
    amp: f64,
    snow_at: f64,
    haze: f64,
    depth: f64,
    valley: f64,
    treeline: bool,
}
const RANGES: [RangeO; 3] = [
    RangeO { seed: 7, step: 70., sharp: 1.7, min_h: 0.10, amp: 0.42, snow_at: 0.40, haze: 0.22, depth: 0.18, valley: 0.13, treeline: false },
    RangeO { seed: 21, step: 50., sharp: 1.4, min_h: 0.06, amp: 0.26, snow_at: 0., haze: 0.14, depth: 0.32, valley: 0.18, treeline: false },
    RangeO { seed: 42, step: 34., sharp: 1.2, min_h: 0.03, amp: 0.15, snow_at: 0., haze: 0.08, depth: 0.5, valley: 0.24, treeline: true },
];
const SKY_D: f64 = 0.05;
const BIRD_D: f64 = 0.42;
const FG_D: f64 = 1.;
const FLY_D: f64 = 1.15;

struct Cloud {
    spr: Rc<Img>,
    w: f64,
    x: f64,
    y: f64,
    v: f64,
    d: f64,
}
struct Star {
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
    big: bool,
}
struct Bird {
    dx: f64,
    dy: f64,
    ph: f64,
}
struct Fly {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    ph: f64,
}

pub struct Valley {
    p: &'static Pal,
    lake_y: f64,
    sun_x: f64,
    sky: Option<Rc<Img>>,
    /// (layer, flipped + water-tinted copy for the reflection, depth)
    ranges: Vec<(Rc<Img>, Rc<Img>, f64)>,
    sky_refl: Option<Rc<Img>>,
    refl_rows: usize,
    fg: Option<Rc<Img>>,
    clouds: Vec<Cloud>,
    stars: Vec<Star>,
    birds: Vec<Bird>,
    birds_x: f64,
    birds_y: f64,
    flies: Vec<Fly>,
}

pub fn new() -> Box<dyn Scene> {
    Box::new(Valley {
        p: &DARK,
        lake_y: 0.,
        sun_x: 0.,
        sky: None,
        ranges: vec![],
        sky_refl: None,
        refl_rows: 0,
        fg: None,
        clouds: vec![],
        stars: vec![],
        birds: vec![],
        birds_x: 0.,
        birds_y: 0.,
        flies: vec![],
    })
}

/// The mock's `noise1d` (Float32Array accumulation kept).
fn noise1d(len: usize, seed: u32, mut step: f64) -> Vec<f32> {
    let mut r = Rng::new(seed);
    let mut out = vec![0f32; len];
    let mut amp = 1f64;
    for _ in 0..6 {
        let np = (len as f64 / step).ceil() as usize + 2;
        let pts: Vec<f64> = (0..np).map(|_| r.next()).collect();
        for (x, o) in out.iter_mut().enumerate() {
            let xf = x as f64;
            let i = (xf / step).floor();
            let f = (xf - i * step) / step;
            let s = (1. - (f * std::f64::consts::PI).cos()) / 2.;
            let i = i as usize;
            *o = (*o as f64 + (pts[i] * (1. - s) + pts[i + 1] * s) * amp) as f32;
        }
        amp *= 0.5;
        step = (step / 2.).max(1.5);
    }
    let lo = out.iter().fold(f32::INFINITY, |a, &b| a.min(b)) as f64;
    let hi = out.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)) as f64;
    out.iter().map(|&v| ((v as f64 - lo) / (hi - lo)) as f32).collect()
}

/// The mock's `pine(g, x, baseY, h, col, rim, side)` (integer coordinates).
fn pine(l: &mut Layer, x: i64, base_y: i64, h: i64, col: C3, rim: Option<C3>, side: i64) {
    let tier_h = h as f64 / ((h as f64 / 7.).round().max(2.));
    for dy in 0..h {
        let d = dy as f64;
        let local = (d % tier_h) / tier_h;
        let hw = (d * 0.26 * (0.45 + 0.55 * local) + if dy > 1 { 0.5 } else { 0. }).floor() as i64;
        let y = base_y - h + dy;
        l.rect_i(col, x - hw, y, hw * 2 + 1, 1);
        if let Some(rc) = rim {
            if hw > 0 && local > 0.3 {
                l.rect_i(rc, if side > 0 { x + hw } else { x - hw }, y, 1, 1);
            }
        }
    }
    l.rect_i(col, x, base_y, 1, 3);
}

/// Mirror rows `[0, top]` of a layer upside down (row k = layer row `top - k`) for
/// `rows` rows, pre-mixed with the lake colour at the mock's water alpha.
fn reflect(l: &Layer, top: i64, rows: usize, water: C3, a: f64) -> Layer {
    let mut out = Layer::new(l.w, rows);
    for k in 0..rows {
        let sy = top - k as i64;
        if sy < 0 || sy >= l.h as i64 {
            continue;
        }
        for x in 0..l.w {
            let p = l.get(x, sy as usize);
            if p[3] == 0 {
                continue;
            }
            let m = mix([p[0], p[1], p[2]], water, a);
            out.d[k * l.w + x] = [m[0], m[1], m[2], 255];
        }
    }
    out
}

impl Valley {
    fn build_sky(&self, v: &Vp, sun_y: f64) -> Layer {
        let p = self.p;
        let (w, h) = (v.fw, v.fh);
        let st = stops(p.sky);
        let (glow, sun) = (hex(p.glow), hex(p.sun));
        let rr = jround(v.h * 0.09).max(6.);
        let g = v.h * 0.6;
        let horizon = v.pad_y + self.lake_y;
        let sun_x = self.sun_x;
        let mut l = Layer::new(w, h);
        for y in 0..h {
            let yf = y as f64;
            let pp = clamp(yf / horizon);
            let mut i = 0;
            while i < st.len() - 2 && pp > st[i + 1].0 {
                i += 1;
            }
            let ((p0, c0), (p1, c1)) = (st[i], st[i + 1]);
            let f = (pp - p0) / (p1 - p0);
            for x in 0..w {
                let xf = x as f64;
                let b = bay(x as i64, y as i64);
                let mut col = if f > b { c1 } else { c0 };
                let dist = (xf - sun_x).hypot((yf - sun_y) * 1.15);
                let gl = (1. - dist / g).max(0.);
                if gl * gl * 1.1 > b {
                    col = mix(col, glow, 0.45);
                }
                if gl > 0.75 && (gl - 0.75) * 5. > b {
                    col = mix(col, glow, 0.8);
                }
                if dist <= rr {
                    let kk = yf - sun_y;
                    let cut = kk > rr * 0.15 && (kk % 4.) < ((kk / rr) * 2.6).floor();
                    if !cut {
                        col = if dist > rr - 1.2 { mix(sun, glow, 0.4) } else { sun };
                    }
                }
                l.d[y * w + x] = [col[0], col[1], col[2], 255];
            }
        }
        l
    }

    fn build_range(&self, v: &Vp, o: &RangeO, pl: &RangeP) -> Layer {
        let (w, h) = (v.fw, v.fh);
        let nn = noise1d(w, o.seed, o.step);
        let base = v.pad_y + self.lake_y;
        let (top, bot, rim, haze) = (hex(pl.top), hex(pl.bot), hex(pl.rim), hex(pl.haze));
        let snow = pl.snow.map(hex);
        let snow_lit = snow.map(|s| mix(s, hex(self.p.sun), 0.5));
        let snow_line = base - o.snow_at * v.h;
        let hz0 = base - o.haze * v.h;
        let sun_x = self.sun_x;
        let tops: Vec<f64> = nn
            .iter()
            .enumerate()
            .map(|(x, &nv)| {
                let vl = 1. - 0.85 * (-((x as f64 - sun_x) / (v.w * o.valley)).powi(2)).exp();
                jround(base - (o.min_h + (nv as f64).powf(o.sharp) * o.amp) * v.h * vl)
            })
            .collect();
        let mut l = Layer::new(w, h);
        for x in 0..w {
            let t0 = tops[x];
            let lit = if x + 1 < w && tops[x + 1] > t0 { (tops[x + 1] - t0).min(4.) } else { 0. };
            let s_depth = if snow.is_some() { (snow_line - t0) * 0.55 } else { 0. };
            for y in (t0.max(0.) as usize)..h {
                let yf = y as f64;
                let b = bay(x as i64, y as i64);
                let kk = yf - t0;
                let mut col = if kk / (base - t0 + 1.) > b { bot } else { top };
                if lit != 0. && kk <= lit {
                    col = rim;
                }
                if s_depth > 0. && kk < s_depth * (0.6 + b * 0.8) {
                    col = if lit != 0. && kk <= lit + 1. { snow_lit.unwrap() } else { snow.unwrap() };
                }
                if (yf - hz0) / (o.haze * v.h) > b {
                    col = haze;
                }
                l.d[y * w + x] = [col[0], col[1], col[2], 255];
            }
        }
        if o.treeline {
            let mut r = Rng::new(o.seed + 99);
            let tl = hex(self.p.treeline);
            let mut x = 0i64;
            while x < w as i64 {
                let ph = 3 + (r.next() * 7.) as i64;
                pine(&mut l, x, base as i64 + 1, ph, tl, None, 0);
                x += 1 + (r.next() * 3.) as i64;
            }
        }
        l
    }

    fn build_foreground(&self, v: &Vp, r: &mut Rng) -> Layer {
        let (w, h) = (v.fw, v.fh);
        let mut l = Layer::new(w, h);
        let base = v.pad_y + v.h;
        let (tree, rim) = (hex(self.p.tree), hex(self.p.tree_rim));
        let sides = [(0., jround(v.pad + v.w * 0.3), 1i64), (jround(v.pad + v.w * 0.74), w as f64, -1)];
        for &(x0, x1, dir) in &sides {
            let span = x1 - x0;
            let mh = v.h * 0.12;
            let surf = |x: f64| {
                let u = if dir > 0 { (x - x0) / span } else { (x1 - x) / span };
                jround(base - mh * (1. - u * u) - 2. + (x * 0.3).sin())
            };
            let mut x = x0;
            while x < x1 {
                let s = surf(x);
                l.rect_i(tree, x as i64, s as i64, 1, h as i64 - s as i64);
                x += 1.;
            }
            let mut trees: Vec<(f64, f64)> = (0..9)
                .map(|_| {
                    let u = r.next();
                    let tx = jround(if dir > 0 { x0 + u * span * 0.85 } else { x1 - u * span * 0.85 });
                    let th = jround(v.h * (0.12 + (1. - u) * 0.32 + r.next() * 0.08));
                    (tx, th)
                })
                .collect();
            trees.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            for (tx, th) in trees {
                pine(&mut l, tx as i64, surf(tx) as i64 + 1, th as i64, tree, Some(rim), dir);
            }
        }
        l
    }

    fn cloud_sprite(&self, r: &mut Rng) -> Layer {
        let w = 24 + (r.next() * 48.) as usize;
        let h = 7 + (r.next() * 8.) as usize;
        let (wf, hf) = (w as f64, h as f64);
        let nb = 4 + (r.next() * 4.) as usize;
        let blobs: Vec<(f64, f64, f64, f64)> = (0..nb)
            .map(|_| {
                let x = wf * (0.15 + r.next() * 0.7);
                let y = hf * (0.5 + r.next() * 0.2);
                let rx = wf * (0.12 + r.next() * 0.15);
                let ry = hf * (0.3 + r.next() * 0.25);
                (x, y, rx, ry)
            })
            .collect();
        let inside = |x: f64, y: f64| y < hf - 1. && blobs.iter().any(|&(bx, by, rx, ry)| ((x - bx) / rx).powi(2) + ((y - by) / ry).powi(2) < 1.);
        let [cl, cm, cd] = self.p.cloud.map(hex);
        let mut l = Layer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let (xf, yf) = (x as f64, y as f64);
                if !inside(xf, yf) {
                    continue;
                }
                let mut kb = 0.;
                while inside(xf, yf + kb + 1.) {
                    kb += 1.;
                }
                let b = bay(x as i64, y as i64);
                let c = if kb < 1. {
                    cl
                } else if kb < 3. {
                    if b > 0.5 { cl } else { cm }
                } else if yf / hf < 0.4 && b < 0.6 {
                    cd
                } else {
                    cm
                };
                l.d[y * w + x] = [c[0], c[1], c[2], 255];
            }
        }
        l
    }

    fn new_fly(v: &Vp, lake_y: f64, r: &mut dyn FnMut() -> f64, anywhere: bool) -> Fly {
        let x = r() * v.w;
        let y = if anywhere { lake_y + r() * (v.h - lake_y) } else { v.h + 2. };
        let vx = (r() - 0.5) * 2.;
        let vy = -(0.6 + r() * 1.8);
        Fly { x, y, vx, vy, ph: r() * 6.28 }
    }
}

impl Scene for Valley {
    fn build(&mut self, bld: &mut Builder, v: &Vp, _t: f64) {
        self.p = if is_light() { &LIGHT_P } else { &DARK };
        let p = self.p;
        self.lake_y = jround(v.h * 0.7);
        self.sun_x = v.pad + jround(v.w * 0.6);
        let sun_y = v.pad_y + self.lake_y - jround(v.h * 0.13);
        let mut r = Rng::new(1234);
        // the lake mirrors screen rows above it; reflect the layers' rows (with room for the
        // vertical parallax) instead of reading the frame back
        let rows = (v.h - self.lake_y) as usize + 2 * v.pad_y as usize + 1;
        let refl_top = (2. * v.pad_y + self.lake_y - 1.) as i64;
        let water = hex(p.water);
        self.refl_rows = rows;
        let sky = self.build_sky(v, sun_y);
        self.sky_refl = Some(bld.img(reflect(&sky, refl_top, rows, water, p.water_alpha)));
        self.sky = Some(bld.img(sky));
        self.ranges = RANGES
            .iter()
            .zip(p.ranges.iter())
            .map(|(o, pl)| {
                let l = self.build_range(v, o, pl);
                let rf = bld.img(reflect(&l, refl_top, rows, water, p.water_alpha));
                (bld.img(l), rf, o.depth)
            })
            .collect();
        self.fg = Some(bld.img(self.build_foreground(v, &mut r)));
        self.clouds = (0..7)
            .map(|_| {
                let spr = self.cloud_sprite(&mut r);
                let w = spr.w as f64;
                let x = r.next() * v.fw as f64 - v.pad;
                let y = jround(v.h * (0.06 + r.next() * 0.34));
                let vv = 0.6 + r.next() * 1.6;
                let d = 0.1 + r.next() * 0.15;
                Cloud { spr: bld.img(spr), w, x, y, v: vv, d }
            })
            .collect();
        let ns = jround(v.w * v.h / 220.) as usize;
        let lake_y = self.lake_y;
        self.stars = (0..ns)
            .map(|_| {
                let x = (r.next() * v.fw as f64).floor();
                let y = (r.next() * lake_y * 0.55).floor();
                let ph = r.next() * 6.28;
                let sp = 0.5 + r.next() * 2.;
                Star { x, y, ph, sp, big: r.next() < 0.08 }
            })
            .collect();
        self.birds = (0..5).map(|i| Bird { dx: (i * 5 - (i % 2) * 2) as f64, dy: ((i - 2) as f64).abs() * 2., ph: r.next() * 6. }).collect();
        self.birds_x = -30.;
        self.birds_y = v.h * 0.3;
        self.flies = (0..26).map(|_| Self::new_fly(v, lake_y, &mut || r.next(), true)).collect();
    }

    fn update(&mut self, v: &Vp, t: f64, dt: f64) {
        for c in &mut self.clouds {
            c.x += c.v * dt;
            if c.x > v.w + v.pad {
                c.x = -c.w - v.pad;
            }
        }
        self.birds_x += 6. * dt;
        if self.birds_x > v.w + v.pad + 40. {
            self.birds_x = -40. - rand() * 200.;
            self.birds_y = v.h * (0.18 + rand() * 0.2);
        }
        let lake_y = self.lake_y;
        for f in &mut self.flies {
            f.x += (f.vx + (t + f.ph).sin() * 1.5) * dt;
            f.y += f.vy * dt;
            if f.y < lake_y - v.h * 0.2 {
                *f = Self::new_fly(v, lake_y, &mut rand, false);
            }
        }
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(sky) = &self.sky else { return };
        let p = self.p;
        let (w, h, lake_y) = (v.w, v.h, self.lake_y);
        f.fill_hex(p.sky[0].1);
        f.rect(0., 0., w, h);
        f.layer(v, sky, SKY_D, 1.);
        if p.stars {
            f.fill_hex(p.star);
            let (ox, oy) = (-v.pad + v.sh(SKY_D + 0.01), v.shy(SKY_D));
            for s in &self.stars {
                let vv = (t * s.sp + s.ph).sin();
                if vv < -0.2 {
                    continue;
                }
                f.alpha = 0.35 + 0.65 * (vv + 0.2) / 1.2;
                f.rect(s.x + ox, s.y + oy, 1., 1.);
                if s.big && vv > 0.8 {
                    f.alpha *= 0.5;
                    f.rect(s.x + ox - 1., s.y + oy, 3., 1.);
                    f.rect(s.x + ox, s.y + oy - 1., 1., 3.);
                }
            }
            f.alpha = 1.;
        }
        for c in &self.clouds {
            f.image(&c.spr, v.snap(c.x + v.sh(c.d)), c.y + v.shy(c.d), 1.);
        }
        for (img, _, d) in &self.ranges {
            f.layer(v, img, *d, 1.);
        }
        f.fill_hex(p.bird);
        let bx = jround(self.birds_x + v.sh(BIRD_D));
        let by = jround(self.birds_y + v.shy(BIRD_D));
        for b in &self.birds {
            let x = bx - b.dx;
            let y = by + b.dy + jround((t * 2. + b.ph).sin());
            let up = ((t * 5. + b.ph).floor() as i64) % 2 != 0;
            f.rect(x - 1., y, 3., 1.);
            let dy = if up { -1. } else { 1. };
            f.rect(x - 2., y + dy, 1., 1.);
            f.rect(x + 2., y + dy, 1., 1.);
        }
        // lake: water, then the wobbling mirror image (pre-tinted with the water's
        // `waterAlpha` wash), grouped into runs of rows that share an offset
        f.fill_hex(p.water);
        f.rect(0., lake_y, w, h - lake_y);
        let nrows = (h - lake_y) as i64;
        let mut runs: Vec<(f64, i64, i64)> = vec![];
        for j in 0..nrows {
            if lake_y - 1. - j as f64 <= -1. {
                break;
            }
            let dx = jround((j as f64 * 0.6 - t * 1.8).sin() * (0.5 + j as f64 * 0.08).min(3.));
            match runs.last_mut() {
                Some((rdx, _, n)) if *rdx == dx => *n += 1,
                _ => runs.push((dx, j, 1)),
            }
        }
        let layers: Vec<(&Rc<Img>, f64)> = std::iter::once((self.sky_refl.as_ref().unwrap(), SKY_D)).chain(self.ranges.iter().map(|(_, rf, d)| (rf, *d))).collect();
        for (img, d) in layers {
            let ix = -v.pad + v.sh(d) + img.x;
            let iy = lake_y - v.pad_y - v.shy(d) + img.y;
            for &(dx, j0, n) in &runs {
                let dst = [(ix + dx) as f32, iy as f32, img.w as f32, img.h as f32];
                let clip = [dx as f32, (lake_y + j0 as f64) as f32, w as f32, n as f32];
                f.image_dst(img, dst, Some(clip), 1.);
            }
        }
        f.alpha = 0.35;
        f.fill_hex(p.ranges[0].haze);
        f.rect(0., lake_y, w, 1.);
        let sx = self.sun_x - v.pad + v.sh(SKY_D);
        f.fill_hex(p.sparkle);
        let mut j = 1;
        while lake_y + (j as f64) < h {
            let mut rr = Rng::new((((t * 5.).floor() as i64) * 997 + j) as u32);
            let spread = 2. + j as f64 * 0.7;
            if rr.next() < 0.6 {
                f.alpha = 0.5 + rr.next() * 0.5;
                let x = jround(sx + (rr.next() - 0.5) * spread * 2.);
                f.rect(x, lake_y + j as f64, 1. + (rr.next() * 4.).floor(), 1.);
            }
            j += 2;
        }
        f.alpha = 1.;
        f.layer(v, self.fg.as_ref().unwrap(), FG_D, 1.);
        f.fill_hex(p.fly);
        let (fx, fy) = (v.sh(FLY_D), v.shy(FLY_D));
        for fl in &self.flies {
            let a = 0.5 + 0.5 * (t * 3. + fl.ph).sin();
            let (x, y) = (jround(fl.x + fx), jround(fl.y + fy));
            f.alpha = a;
            f.rect(x, y, 1., 1.);
            f.alpha = a * 0.3;
            f.rect(x - 1., y, 3., 1.);
            f.rect(x, y - 1., 1., 3.);
        }
        f.alpha = 1.;
    }
}
