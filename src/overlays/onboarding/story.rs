//! The story world: water, rocks, kelp, fish, jellies, plankton, snow, bubbles,
//! Tako rising in, eight task cards, the clam + pearls and the bubble wordmark.
//! Port of the mock's ONBOARDING `resize` / `layout` / `update` / `draw`.
//!
//! Units are art pixels (1 art px = `s` logical px, `s = max(3, round(H / 175))`).
//! Static layers are painted once per size and uploaded through `px::Builder`; the
//! rest is recorded every frame into a `px::Frame`.
use crate::scene::octo::{Octo, draw_octo};
use crate::scene::px::{Builder, C3, Frame, Img, Layer, Raster, Rng, bay, clamp, hex, jround, rand};
use gpui_kit::RenderImage;
use std::{rc::Rc, sync::Arc};

pub const P: f64 = 14.;
const REL: [[f64; 2]; 8] = [[-66., -24.], [-76., 0.], [-68., 24.], [-46., 42.], [46., 42.], [68., 24.], [76., 0.], [66., -24.]];
const LINE_C: [&str; 4] = ["#8be9ff", "#ffc4e8", "#b8ffcf", "#ffe08a"];

fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}
pub fn ease(x: f64) -> f64 {
    if x < 0.5 { 4. * x * x * x } else { 1. - (-2. * x + 2.).powi(3) / 2. }
}
fn back(x: f64) -> f64 {
    let c = 1.70158;
    let c3 = c + 1.;
    1. + c3 * (x - 1.).powi(3) + c * (x - 1.).powi(2)
}
fn r(a: f64, b: f64) -> f64 {
    a + rand() * (b - a)
}

/// The mock's `noise1d(len, seed, step)`: 4 octaves of cosine-interpolated value noise.
fn noise1d(len: usize, seed: u32, mut step: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut out = vec![0f64; len];
    let (mut amp, mut tot) = (1., 0.);
    for _ in 0..4 {
        let n = (len as f64 / step).ceil() as usize + 2;
        let pts: Vec<f64> = (0..n).map(|_| rng.next()).collect();
        for (x, o) in out.iter_mut().enumerate() {
            let i = (x as f64 / step).floor();
            let f = (x as f64 - i * step) / step;
            let k = (1. - (f * std::f64::consts::PI).cos()) / 2.;
            let i = i as usize;
            *o += (pts[i] * (1. - k) + pts[(i + 1).min(n - 1)] * k) * amp;
        }
        tot += amp;
        amp *= 0.5;
        step = (step / 2.).max(2.);
    }
    out.iter().map(|v| v / tot).collect()
}

// ---------------------------------------------------------------- glow sprites

/// Glow colours (the mock's `GL`).
#[derive(Clone, Copy)]
pub enum Gl {
    Cyan = 0,
    Blue = 1,
    Pink = 2,
    Pearl = 3,
}
const GL_COLS: [[u8; 3]; 4] = [[130, 215, 255], [80, 140, 255], [255, 140, 210], [255, 236, 250]];
const GL_LEVELS: usize = 32;

/// Radial-gradient glow sprites with the alpha baked in (GPUI can only apply
/// opacity per element, so each alpha level is its own small image). Sampled
/// linearly by the GPU, which is what we want for a soft glow.
pub struct Glows {
    imgs: Vec<Vec<Rc<Img>>>,
}
impl Glows {
    pub fn new(b: &mut Builder) -> Self {
        let imgs = GL_COLS
            .iter()
            .map(|c| {
                (1..=GL_LEVELS)
                    .map(|lv| {
                        let a = lv as f64 / GL_LEVELS as f64;
                        let mut l = Layer::new(64, 64);
                        for y in 0..64 {
                            for x in 0..64 {
                                let d = ((x as f64 + 0.5 - 32.).hypot(y as f64 + 0.5 - 32.) / 32.).min(1.);
                                let g = if d < 0.3 { 1. - d / 0.3 * 0.67 } else { 0.33 * (1. - (d - 0.3) / 0.7) };
                                l.d[y * 64 + x] = [c[0], c[1], c[2], (g * a * 255.).round() as u8];
                            }
                        }
                        b.img_raw(l, 0., 0., 1)
                    })
                    .collect()
            })
            .collect();
        Glows { imgs }
    }
    /// The mock's `drawGlow(sp, x, y, r, a)` ("lighter" → source-over, see GAPS.md).
    pub fn draw(&self, f: &mut Frame, g: Gl, x: f64, y: f64, r: f64, a: f64) {
        let lv = (clamp(a) * GL_LEVELS as f64).round() as usize;
        if lv == 0 {
            return;
        }
        let img = &self.imgs[g as usize][lv - 1];
        f.image_dst(img, [(x - r) as f32, (y - r) as f32, (r * 2.) as f32, (r * 2.) as f32], None, 1.);
    }
}

// ---------------------------------------------------------------- world state

struct Snow {
    x: f64,
    y: f64,
    v: f64,
    d: f64,
    a: f64,
    ph: f64,
}
struct Plank {
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
}
struct Bubble {
    x: f64,
    y: f64,
    v: f64,
    ph: f64,
    big: bool,
}
struct Jelly {
    x: f64,
    y: f64,
    ph: f64,
    d: f64,
}
struct Kelp {
    x: f64,
    h: f64,
    ph: f64,
}
struct Fish {
    ox: f64,
    oy: f64,
    ph: f64,
}
#[derive(Clone)]
struct Line {
    w: f64,
    ind: f64,
    c: C3,
}
struct Task {
    i: usize,
    dx: f64,
    dy: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    a: f64,
    p: f64,
    sp: f64,
    sent: bool,
    lines: Vec<Line>,
}
struct Pearl {
    x0: f64,
    y0: f64,
    t0: f64,
    dur: f64,
    x: f64,
    y: f64,
    trail: Vec<(f64, f64)>,
}
struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
}
struct WordPx {
    tx: f64,
    ty: f64,
    b: f64,
    d: f64,
    done: bool,
    k: Option<f64>,
}
#[derive(Default)]
struct Oct {
    x: f64,
    y: f64,
    arr: f64,
    arrived_at: f64,
    blink_at: f64,
    blink: f64,
}
#[derive(Default)]
struct Clam {
    x: f64,
    y: f64,
    rise: f64,
    open: f64,
    count: usize,
    flash: f64,
}

pub struct Story {
    /// art size and logical px per art px
    pub w: f64,
    pub h: f64,
    pub s: f64,
    /// (W, H, device scale ×100) the layers were built for
    pub key: (usize, usize, u32),
    floor_y: f64,
    bg: Option<Rc<Img>>,
    rays: Option<Rc<Img>>,
    rocks_far: Option<Rc<Img>>,
    rocks_near: Option<Rc<Img>>,
    /// GPU images of the current layers (dropped from the atlas on rebuild)
    pub made: Vec<Arc<RenderImage>>,
    snow: Vec<Snow>,
    plank: Vec<Plank>,
    bubbles: Vec<Bubble>,
    jellies: Vec<Jelly>,
    school: Vec<Fish>,
    school_x: f64,
    school_y: f64,
    kelp_far: Vec<Kelp>,
    kelp_near: Vec<Kelp>,
    tasks: Vec<Task>,
    pearls: Vec<Pearl>,
    sparks: Vec<Spark>,
    word: Vec<WordPx>,
    oct: Oct,
    clam: Clam,
    reach: [f64; 8],
    pub step: usize,
    /// time in the current step / total time
    pub st: f64,
    pub t: f64,
    pub mx: f64,
    pub my: f64,
    pub tmx: f64,
    pub tmy: f64,
}

impl Story {
    pub fn new() -> Self {
        Story {
            w: 300.,
            h: 180.,
            s: 4.,
            key: (0, 0, 0),
            floor_y: 150.,
            bg: None,
            rays: None,
            rocks_far: None,
            rocks_near: None,
            made: vec![],
            snow: vec![],
            plank: vec![],
            bubbles: vec![],
            jellies: vec![],
            school: vec![],
            school_x: 0.,
            school_y: 0.,
            kelp_far: vec![],
            kelp_near: vec![],
            tasks: vec![],
            pearls: vec![],
            sparks: vec![],
            word: vec![],
            oct: Oct { arrived_at: -1., blink_at: 3., ..Default::default() },
            clam: Clam::default(),
            reach: [0.; 8],
            step: 0,
            st: 0.,
            t: 0.,
            mx: 0.,
            my: 0.,
            tmx: 0.,
            tmy: 0.,
        }
    }

    /// The mock's `open()` reset (keeps the built layers).
    pub fn reset(&mut self) {
        self.step = 0;
        self.st = 0.;
        self.t = 0.;
        self.oct.x = 0.;
        self.oct.arr = 0.;
        self.oct.arrived_at = -1.;
        self.oct.blink_at = 3.;
        self.bubbles.clear();
        self.pearls.clear();
        self.sparks.clear();
        self.tasks.clear();
        self.reach = [0.; 8];
        self.clam.rise = 0.;
        self.clam.open = 0.;
        self.clam.count = 0;
        if self.key.0 > 0 {
            self.layout();
        }
    }

    /// `go(0)` in the mock re-runs the rise.
    pub fn restart_rise(&mut self) {
        self.oct.arr = 0.;
        self.oct.arrived_at = -1.;
        self.t = 0.;
    }

    fn oct_target(&self, s: usize) -> (f64, f64) {
        (jround(self.w * 0.64), jround(self.h * if s >= 3 { 0.5 } else { 0.41 }))
    }

    /// Build the static layers for a `vw`×`vh` logical viewport at device scale `sf`.
    /// Returns false if nothing changed.
    pub fn resize(&mut self, vw: f32, vh: f32, sf: f32) -> bool {
        let s = (vh as f64 / 175.).round().max(3.);
        let (w, h) = ((vw as f64 / s).ceil(), (vh as f64 / s).ceil());
        let key = (w as usize, h as usize, (sf * 100.) as u32);
        if key == self.key && s == self.s {
            return false;
        }
        self.key = key;
        self.s = s;
        self.w = w;
        self.h = h;
        self.floor_y = jround(h * 0.86);
        let (fw, fh) = ((w + P * 2.) as usize, (h + P) as usize);
        let mut b = Builder::new((s * sf as f64) as f32);
        // water column
        let st: Vec<(f64, C3)> = [(0., "#3584d8"), (0.08, "#2c72c6"), (0.18, "#2462b4"), (0.3, "#1c509c"), (0.42, "#164083"), (0.55, "#11326b"), (0.68, "#0c2655"), (0.82, "#081b40"), (1., "#05112c")]
            .iter()
            .map(|(p, c)| (*p, hex(c)))
            .collect();
        let mut bg = Layer::new(fw, fh);
        for j in 0..fh {
            let p = j as f64 / fh as f64;
            let mut i = 0;
            while i < st.len() - 2 && p > st[i + 1].0 {
                i += 1;
            }
            let f = (p - st[i].0) / (st[i + 1].0 - st[i].0);
            for k in 0..fw {
                let (kf, jf) = (k as f64, j as f64);
                let mut c = if f > 0.5 + (bay(k as i64, j as i64) - 0.5) * 0.55 { st[i + 1].1 } else { st[i].1 };
                let gl = clamp(1. - ((kf - fw as f64 * 0.58) * 0.55).hypot(jf + 10.) / (h * 0.8));
                if gl * gl > 0.5 + (bay(k as i64 + 1, j as i64) - 0.5) * 0.6 {
                    let t = [150., 210., 255.];
                    c = [0, 1, 2].map(|q| (c[q] as f64 + (t[q] - c[q] as f64) * 0.08).round() as u8);
                }
                bg.d[j * fw + k] = [c[0], c[1], c[2], 255];
            }
        }
        self.bg = Some(b.img_raw(bg, 0., 0., 1));
        // god rays
        let (ang, wk) = (0.32f64, w / 300.);
        let (ca, sa) = (ang.cos(), ang.sin());
        let beams: Vec<(f64, f64, f64)> = [(-0.24, 7., 0.7), (-0.1, 11., 1.), (0.02, 5., 0.8), (0.14, 9., 0.9), (0.28, 6., 0.6), (0.4, 10., 0.5)]
            .iter()
            .map(|&(o, bw, s)| (fw as f64 * (0.5 + o) * ca, bw * wk, s))
            .collect();
        let mut rays = Layer::new(fw, fh);
        for j in 0..fh {
            for k in 0..fw {
                let uu = k as f64 * ca + j as f64 * sa * -0.55;
                let mut i = 0.;
                for &(c, bw, s) in &beams {
                    i += s * (-((uu - c) / bw).powi(2)).exp();
                }
                i *= clamp(1. - j as f64 / (h * 0.78));
                let q = (clamp(i) * 5.).floor() / 5.;
                if q > 0. {
                    rays.d[j * fw + k] = [170, 225, 255, (q * 90.).round() as u8];
                }
            }
        }
        self.rays = Some(b.img(rays));
        // rock silhouettes
        let ridge = |seed: u32, step: f64, base: f64, amp: f64, col: &str, rim: &str| {
            let n = noise1d(fw, seed, step);
            let (col, rim) = (hex(col), hex(rim));
            let mut l = Layer::new(fw, fh);
            for (k, nv) in n.iter().enumerate() {
                let top = jround(base - nv * amp) as i64;
                l.rect_i(col, k as i64, top, 1, fh as i64 - top);
                l.rect_i(rim, k as i64, top, 1, 1);
            }
            l
        };
        self.rocks_far = Some(b.img(ridge(11, 40., h * 0.9, h * 0.2, "#0a2556", "#1a4d8c")));
        self.rocks_near = Some(b.img(ridge(29, 26., self.floor_y + 8., h * 0.07, "#030a1f", "#16548e")));
        self.made = b.made;
        // life
        let (fwf, _) = (fw as f64, fh as f64);
        self.snow = (0..jround(w * h / 380.) as usize).map(|_| Snow { x: r(0., fwf), y: r(0., h), v: r(1.5, 5.), d: r(0.3, 0.9), a: r(0.2, 0.6), ph: r(0., 6.) }).collect();
        self.plank = (0..28).map(|_| Plank { x: r(0., fwf), y: r(h * 0.15, h * 0.9), ph: r(0., 6.), sp: r(0.6, 1.6) }).collect();
        self.jellies = vec![Jelly { x: w * 0.18, y: h * 0.28, ph: 0., d: 0.45 }, Jelly { x: w * 0.9, y: h * 0.62, ph: 2.2, d: 0.35 }];
        self.kelp_far = (0..jround(w / 22.) as usize).map(|_| Kelp { x: r(0., fwf), h: r(h * 0.25, h * 0.55), ph: r(0., 6.) }).collect();
        self.kelp_near = [(0.02, 0.62), (0.06, 0.48), (0.95, 0.55), (0.985, 0.7), (0.9, 0.4)].iter().map(|&(a, hh)| Kelp { x: fwf * a, h: h * hh, ph: r(0., 6.) }).collect();
        self.school = (0..16).map(|_| Fish { ox: r(-18., 18.), oy: r(-7., 7.), ph: r(0., 6.) }).collect();
        self.school_x = w + 40.;
        self.school_y = h * 0.22;
        self.layout();
        true
    }

    fn layout(&mut self) {
        let (tx, _) = self.oct_target(self.step);
        if self.oct.x == 0. {
            self.oct.x = tx;
            self.oct.y = self.h + 34.;
        }
        self.clam.x = tx;
        self.clam.y = self.floor_y - 1.;
        let k = (self.w / 300.).clamp(0.72, 1.1);
        let old = std::mem::take(&mut self.tasks);
        self.tasks = REL
            .iter()
            .enumerate()
            .map(|(i, &[dx, dy])| {
                let o = old.get(i);
                Task {
                    i,
                    dx: dx * k,
                    dy: dy * k,
                    x: 0.,
                    y: 0.,
                    w: 22.,
                    h: 15.,
                    a: o.map(|o| o.a).unwrap_or(0.),
                    p: o.map(|o| o.p).unwrap_or(0.),
                    sp: r(0.32, 0.55),
                    sent: o.map(|o| o.sent).unwrap_or(false),
                    lines: (0..3).map(|j| Line { w: jround(r(8., 16.)), ind: if j == 1 { 2. } else { 0. }, c: hex(LINE_C[(i + j) % 4]) }).collect(),
                }
            })
            .collect();
        // wordmark from 5×7 glyphs
        let glyph = |ch: char| -> [&'static str; 7] {
            match ch {
                'T' => ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
                'A' => ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
                'K' => ["10001", "10010", "10100", "11000", "10100", "10010", "10001"],
                _ => ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
            }
        };
        let bb = if self.w >= 230. { 3. } else { 2. };
        let gap = bb * 2.;
        let total = 4. * 5. * bb + 3. * gap;
        let fx = self.oct_target(3).0;
        let (x0, y0) = (jround(fx - total / 2.), jround(self.h * 0.1));
        self.word.clear();
        for (li, ch) in "TAKO".chars().enumerate() {
            for (j, row) in glyph(ch).iter().enumerate() {
                for (i, v) in row.chars().enumerate() {
                    if v == '1' {
                        let (lf, jf, if_) = (li as f64, j as f64, i as f64);
                        self.word.push(WordPx {
                            tx: x0 + lf * (5. * bb + gap) + if_ * bb,
                            ty: y0 + jf * bb,
                            b: bb,
                            d: (lf * 7. + jf + if_ * 0.6) * 0.028 + rand() * 0.08,
                            done: false,
                            k: None,
                        });
                    }
                }
            }
        }
    }

    /// The mock's `setStep` side effects on the world.
    pub fn set_step(&mut self, i: usize, instant: bool) {
        let prev = self.step;
        self.step = i.min(3);
        if self.step != prev || instant {
            self.st = 0.;
        }
        if self.step >= 2 && prev < 2 {
            for t in &mut self.tasks {
                t.p = 1.;
            }
        }
    }

    fn bubble(&mut self, x: f64, y: f64, fast: bool) {
        self.bubbles.push(Bubble { x, y, v: if fast { r(18., 30.) } else { r(8., 16.) }, ph: r(0., 6.), big: rand() < 0.35 });
    }

    pub fn update(&mut self, dt: f64) {
        self.t += dt;
        self.st += dt;
        let (t, st, step) = (self.t, self.st, self.step);
        let e3 = 1. - (-dt * 3.).exp();
        self.mx += (self.tmx - self.mx) * e3;
        self.my += (self.tmy - self.my) * e3;
        // octopus: rise in, then glide between poses
        let (tx, ty) = self.oct_target(step);
        if self.oct.arr < 1. {
            self.oct.arr = clamp(self.oct.arr + dt / 2.6 * if t > 0.35 { 1. } else { 0. });
            self.oct.x = tx;
            self.oct.y = lerp(self.h + 34., ty, ease(self.oct.arr));
            if self.oct.arr < 1. && rand() < dt * 14. {
                let (x, y) = (self.oct.x + r(-10., 10.), self.oct.y + r(-10., 10.));
                self.bubble(x, y, true);
            }
            if self.oct.arr >= 1. {
                self.oct.arrived_at = t;
            }
        } else {
            let e = 1. - (-dt * 2.4).exp();
            self.oct.x += (tx - self.oct.x) * e;
            self.oct.y += (ty + (t * 1.6).sin() * 1.4 - self.oct.y) * e;
        }
        // blinking
        if t > self.oct.blink_at {
            self.oct.blink = 0.16;
            self.oct.blink_at = t + r(2.5, 5.);
        }
        self.oct.blink = (self.oct.blink - dt).max(0.);
        // arms reach for tasks in beat II, hand off in beat III
        const ORDER: [usize; 8] = [3, 4, 2, 5, 1, 6, 0, 7];
        for i in 0..8 {
            let di = ORDER.iter().position(|&o| o == i).unwrap() as f64;
            let want = match step {
                1 => (st > 0.25 + di * 0.1) as i32 as f64,
                2 => (st < 0.35 + i as f64 * 0.14) as i32 as f64,
                _ => 0.,
            };
            self.reach[i] += (want - self.reach[i]) * (1. - (-dt * if want > 0. { 3.4 } else { 2.6 }).exp());
        }
        let (ox, oy) = (self.oct.x, self.oct.y);
        let mut new_pearls = vec![];
        for tk in &mut self.tasks {
            tk.x = ox + tk.dx;
            tk.y = oy + tk.dy;
            let visible = (step == 1 && self.reach[tk.i] > 0.7) || (step == 2 && !tk.sent);
            let v = if visible { 1. } else { 0. };
            tk.a += (v - tk.a) * (1. - (-dt * if visible { 7. } else { 5. }).exp());
            if step == 1 && tk.a > 0.9 {
                tk.p = (tk.p + dt * tk.sp).min(1.);
            }
            if step == 2 {
                tk.p = 1.;
                if !tk.sent && st > 0.35 + tk.i as f64 * 0.14 {
                    tk.sent = true;
                    new_pearls.push(Pearl { x0: tk.x, y0: tk.y, t0: t, dur: 1.15, x: tk.x, y: tk.y, trail: vec![] });
                }
            }
            if step < 1 {
                tk.p = 0.;
            }
            if step < 2 {
                tk.sent = false;
            }
        }
        self.pearls.extend(new_pearls);
        // clam + pearls
        self.clam.rise += ((step >= 2) as i32 as f64 - self.clam.rise) * (1. - (-dt * 3.).exp());
        self.clam.open += ((step >= 2 && self.clam.rise > 0.7) as i32 as f64 - self.clam.open) * (1. - (-dt * 4.).exp());
        self.clam.flash = (self.clam.flash - dt * 2.5).max(0.);
        let (cx_, cy_) = (self.clam.x, self.clam.y);
        let mut landed = 0;
        self.pearls.retain_mut(|p| {
            let k = clamp((t - p.t0) / p.dur);
            let e = ease(k);
            let (cx, cy) = ((p.x0 + cx_) / 2., p.y0.min(cy_) - 26.);
            p.x = (1. - e) * (1. - e) * p.x0 + 2. * (1. - e) * e * cx + e * e * cx_;
            p.y = (1. - e) * (1. - e) * p.y0 + 2. * (1. - e) * e * cy + e * e * (cy_ - 4.);
            p.trail.push((p.x, p.y));
            if p.trail.len() > 7 {
                p.trail.remove(0);
            }
            if k >= 1. {
                landed += 1;
                return false;
            }
            true
        });
        for _ in 0..landed {
            self.clam.count = (self.clam.count + 1).min(8);
            self.clam.flash = 1.;
            for _ in 0..10 {
                self.sparks.push(Spark { x: cx_, y: cy_ - 4., vx: r(-26., 26.), vy: r(-38., -8.), life: r(0.4, 0.8) });
            }
        }
        if step < 2 {
            self.clam.count = 0;
            self.pearls.clear();
        }
        self.sparks.retain_mut(|s| {
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.vy += 60. * dt;
            s.life -= dt;
            s.life > 0.
        });
        // wordmark
        for b in &mut self.word {
            if step < 3 {
                b.done = false;
                continue;
            }
            let k = (st - 0.3 - b.d) / 0.9;
            b.k = Some(k);
            if k >= 1. && !b.done {
                b.done = true;
            }
        }
        // ambient life
        let (w, h) = (self.w, self.h);
        for s in &mut self.snow {
            s.y += s.v * dt;
            s.x += (t * 0.4 + s.ph).sin() * dt * 1.2;
            if s.y > h + 2. {
                s.y = -2.;
                s.x = r(0., w + P * 2.);
            }
        }
        if rand() < dt * 1.6 && self.oct.arr >= 1. {
            let (x, y) = (self.oct.x + r(-6., 6.), self.oct.y - 18.);
            self.bubble(x, y, false);
        }
        if rand() < dt * 2.2 {
            let (x, y) = (r(0., w), self.floor_y + r(-2., 4.));
            self.bubble(x, y, false);
        }
        self.bubbles.retain_mut(|b| {
            b.y -= b.v * dt;
            b.x += (t * 3. + b.ph).sin() * dt * 5.;
            b.y > -4.
        });
        for j in &mut self.jellies {
            j.y += (t * 0.35 + j.ph).sin() * dt * 2.2;
        }
        self.school_x -= dt * 9.;
        if self.school_x < -60. {
            self.school_x = w + 60. + r(0., 120.);
            self.school_y = h * r(0.14, 0.34);
        }
    }

    fn sh(&self, d: f64) -> f64 {
        jround(-self.mx * d * 10.)
    }
    fn shy(&self, d: f64) -> f64 {
        jround(-self.my * d * 5.)
    }

    pub fn draw(&self, f: &mut Frame, gl: &Glows) {
        let (Some(bg), Some(rays), Some(far), Some(near)) = (&self.bg, &self.rays, &self.rocks_far, &self.rocks_near) else { return };
        let (t, w, h) = (self.t, self.w, self.h);
        let px = |f: &mut Frame, x: f64, y: f64, ww: f64, hh: f64| f.rect(jround(x), jround(y), ww, hh);
        f.alpha = 1.;
        f.image(bg, -P + self.sh(0.1), self.shy(0.1) - P * 0.5, 1.);
        // rays breathe ("lighter" → source-over at a slightly boosted alpha)
        f.image(rays, -P + self.sh(0.2) + (t * 0.2).sin() * 2., self.shy(0.2) - P * 0.5, ((0.38 + 0.14 * (t * 0.5).sin()) * 1.25).min(1.));
        // surface shimmer
        let (hi, lo) = (hex("#c8ecff"), hex("#4aa3e8"));
        for pass in 0..2 {
            f.fill(if pass == 0 { hi } else { lo });
            f.alpha = if pass == 0 { 0.45 } else { 0.4 };
            for x in 0..w as i64 {
                let xf = x as f64;
                let y = 2. + jround((xf * 0.12 + t * 1.4).sin() * 1.1 + (xf * 0.041 - t * 0.8).sin() * 1.3);
                f.rect(xf, y + pass as f64, 1., 1.);
            }
        }
        f.alpha = 1.;
        // far rocks + kelp
        f.image(far, -P + self.sh(0.28), self.shy(0.28), 1.);
        f.fill_hex("#0b2c62");
        for k in &self.kelp_far {
            let (bx, base) = (k.x - P + self.sh(0.32), h * 0.92 + self.shy(0.32));
            let mut j = 0.;
            while j < k.h {
                let u = j / k.h;
                let x = jround(bx + (t * 0.7 + k.ph - j * 0.08).sin() * u * u * 6.);
                px(f, x, base - j, 1., 1.);
                if j as i64 % 5 == 2 {
                    px(f, x + if (j as i64) % 10 < 5 { 1. } else { -1. }, base - j, 1., 1.);
                }
                j += 1.;
            }
        }
        // a school of fish far away
        f.fill_hex("#123c7c");
        for fi in &self.school {
            let x = self.school_x + fi.ox + (t * 1.5 + fi.ph).sin() * 2. + self.sh(0.35);
            let y = self.school_y + fi.oy + (t * 2. + fi.ph).sin() * 1.5 + self.shy(0.35);
            px(f, x, y, 2., 1.);
            px(f, x + 2., y - 1., 1., 1.);
            px(f, x + 2., y + 1., 1., 1.);
        }
        // jellyfish
        for j in &self.jellies {
            let (x, y) = (jround(j.x + self.sh(j.d)), jround(j.y + self.shy(j.d)));
            let pu = (t * 2.2 + j.ph).sin();
            gl.draw(f, Gl::Pink, x, y + 2., 16., 0.28);
            f.fill_hex("#ff9fd8");
            f.alpha = 0.8;
            for dy in -5i32..=0 {
                let ww = jround(((25 - dy * dy) as f64).sqrt() * (1. + pu * 0.12));
                f.rect(x - ww, y + dy as f64, 2. * ww + 1., 1.);
            }
            f.fill_hex("#ffd6f0");
            f.rect(x - 2., y - 4., 2., 1.);
            f.alpha = 0.55;
            f.fill_hex("#ff9fd8");
            for k in -2i32..=2 {
                for s in 1..11 {
                    let (kf, sf) = (k as f64, s as f64);
                    px(f, x + kf * 1.6 + (t * 2. + sf * 0.5 + kf).sin() * (sf / 6.), y + sf, 1., 1.);
                }
            }
            f.alpha = 1.;
        }
        // plankton + marine snow
        for p in &self.plank {
            let a = 0.5 + 0.5 * (t * p.sp + p.ph).sin();
            let (x, y) = (p.x - P + self.sh(0.5), p.y + (t * 0.6 + p.ph).sin() * 3. + self.shy(0.5));
            gl.draw(f, Gl::Cyan, x, y, 4., a * 0.5);
            f.fill_hex("#c8f4ff");
            f.alpha = a;
            px(f, x, y, 1., 1.);
        }
        f.alpha = 1.;
        f.fill_hex("#a9c8ff");
        for s in &self.snow {
            f.alpha = s.a;
            px(f, s.x - P + self.sh(s.d), s.y + self.shy(s.d), 1., 1.);
        }
        f.alpha = 1.;
        // seabed
        f.image(near, -P + self.sh(0.75), self.shy(0.75) - P * 0.2, 1.);
        // clam
        self.draw_clam(f, gl, self.clam.x + self.sh(0.75), jround(lerp(self.floor_y + 14., self.clam.y, ease(self.clam.rise))) + self.shy(0.75));
        // tasks behind the octopus' arms
        let (ox, oy) = (self.sh(0.8), self.shy(0.8));
        for tk in &self.tasks {
            self.draw_task(f, gl, tk, ox, oy);
        }
        // the octopus
        let mut tips = [[0.; 2]; 8];
        for tk in &self.tasks {
            tips[tk.i] = [tk.x + ox + if tk.dx < 0. { tk.w / 2. - 1. } else { -tk.w / 2. + 1. }, tk.y + oy + 3.];
        }
        let o = &self.oct;
        let lid_open = if o.arrived_at < 0. { 1. } else { clamp(1. - (t - o.arrived_at) / 0.35) };
        let wave_hello = if self.step == 0 && o.arrived_at > 0. { (std::f64::consts::PI * clamp((t - o.arrived_at - 0.5) / 1.9)).sin() } else { 0. };
        let wave_end = if self.step == 3 { ease(clamp((self.st - 1.4) / 0.6)) } else { 0. };
        gl.draw(f, Gl::Blue, o.x + ox, o.y + oy, 46., 0.35);
        let (cx, cy) = (jround(o.x + ox) as i32, jround(o.y + oy) as i32);
        let mut ras = Raster::new(cx - 112, cy - 48, 224, 116);
        draw_octo(
            &mut ras,
            &Octo {
                x: o.x + ox,
                y: o.y + oy,
                t,
                reach: Some(self.reach),
                tips: Some(tips),
                wave: wave_hello.max(wave_end),
                lid: lid_open.max(if o.blink > 0. { 1. } else { 0. }),
                look: [self.mx, self.my],
                ripple: match self.step {
                    1 => clamp(self.st / 0.8),
                    3 => 0.5,
                    _ => 0.,
                },
                ..Default::default()
            },
        );
        ras.emit(f, 0., 0.);
        // pearls in flight
        for p in &self.pearls {
            f.fill_hex("#ffc8ec");
            for (i, &(x, y)) in p.trail.iter().enumerate() {
                f.alpha = (i + 1) as f64 / 10.;
                px(f, x + ox, y + oy, 1., 1.);
            }
            f.alpha = 1.;
            gl.draw(f, Gl::Pearl, p.x + ox, p.y + oy, 8., 0.55);
            f.fill_hex("#f6eeff");
            disc(f, p.x + ox, p.y + oy, 1.6);
            f.fill_hex("#ffffff");
            px(f, p.x + ox - 1., p.y + oy - 1., 1., 1.);
        }
        f.fill_hex("#fff0fa");
        for s in &self.sparks {
            f.alpha = clamp(s.life * 2.);
            px(f, s.x + ox, s.y + oy, 1., 1.);
        }
        f.alpha = 1.;
        // wordmark: bubbles fly out of Tako and settle into letters
        let mut any_done = false;
        let (wx, wy) = (self.sh(0.6), self.shy(0.6));
        for b in &self.word {
            let Some(k) = b.k else { continue };
            if self.step < 3 || k < 0. {
                continue;
            }
            if b.done {
                any_done = true;
                let sh2 = 0.5 + 0.5 * (t * 2.4 - b.tx * 0.07).sin();
                f.fill_hex(if sh2 > 0.75 { "#c8ecff" } else { "#8ccfff" });
                f.rect(b.tx + wx, b.ty + wy, b.b, b.b);
                f.fill_hex("#ffffff");
                f.rect(b.tx + wx, b.ty + wy, 1., 1.);
                if b.b > 2. {
                    f.fill_hex("#3f86d8");
                    f.rect(b.tx + wx + b.b - 1., b.ty + wy + b.b - 1., 1., 1.);
                }
                continue;
            }
            let e = ease(clamp(k));
            let (sx, sy) = (o.x + ox, o.y + oy - 16.);
            let (cx, cy) = ((sx + b.tx) / 2. + (b.d * 40.).sin() * 20., sy.min(b.ty) - 14.);
            let x = (1. - e) * (1. - e) * sx + 2. * (1. - e) * e * cx + e * e * (b.tx + wx);
            let y = (1. - e) * (1. - e) * sy + 2. * (1. - e) * e * cy + e * e * (b.ty + wy);
            f.fill_hex("#d8f2ff");
            f.alpha = 0.9;
            px(f, x, y, 2., 2.);
            f.alpha = 1.;
        }
        if any_done {
            gl.draw(f, Gl::Cyan, self.oct_target(3).0 + self.sh(0.6), h * 0.1 + 10., 70., 0.16 + 0.04 * (t * 2.).sin());
        }
        // bubbles
        for b in &self.bubbles {
            let (x, y) = (jround(b.x + self.sh(0.9)), jround(b.y + self.shy(0.9)));
            f.fill_hex("#d8f4ff");
            if b.big {
                f.alpha = 0.7;
                f.rect(x, y - 1., 1., 1.);
                f.rect(x - 1., y, 1., 1.);
                f.rect(x + 1., y, 1., 1.);
                f.rect(x, y + 1., 1., 1.);
                f.alpha = 1.;
                f.fill_hex("#ffffff");
                f.rect(x - 1., y - 1., 1., 1.);
            } else {
                f.alpha = 0.75;
                f.rect(x, y, 1., 1.);
                f.alpha = 1.;
            }
        }
        // foreground kelp frames the shot
        f.fill_hex("#03102c");
        for k in &self.kelp_near {
            let (bx, base) = (k.x - P + self.sh(1.2), h + 2.);
            let mut j = 0.;
            while j < k.h {
                let u = j / k.h;
                let x = jround(bx + (t * 0.8 + k.ph - j * 0.06).sin() * u * u * 9.);
                f.rect(x, base - j, 3., 1.);
                let ji = j as i64;
                if ji % 6 == 3 {
                    let s = if ji % 12 < 6 { 1. } else { -1. };
                    f.rect(x + if s > 0. { 3. } else { -3. }, base - j, 3., 1.);
                    f.rect(x + if s > 0. { 5. } else { -4. }, base - j - 1., 2., 1.);
                }
                j += 1.;
            }
        }
    }

    fn draw_task(&self, f: &mut Frame, gl: &Glows, tk: &Task, ox: f64, oy: f64) {
        if tk.a <= 0.02 {
            return;
        }
        let a = back(clamp(tk.a));
        let (w, h) = (jround(tk.w * a), jround(tk.h * a));
        if w < 3. || h < 3. {
            return;
        }
        let (cx, cy) = (tk.x + ox, tk.y + oy);
        let (x, y) = (jround(cx - w / 2.), jround(cy - h / 2.));
        let done = tk.p >= 1.;
        let px = |f: &mut Frame, x: f64, y: f64| f.rect(jround(x), jround(y), 1., 1.);
        gl.draw(f, if done { Gl::Pearl } else { Gl::Cyan }, cx, cy, 20., 0.2 * clamp(tk.a));
        f.fill_hex("#08214a");
        f.alpha = 0.92;
        f.rect(x, y, w, h);
        f.alpha = 1.;
        f.fill_hex(if done { "#9dffd0" } else { "#7fd6ff" });
        f.rect(x, y, w, 1.);
        f.rect(x, y + h - 1., w, 1.);
        f.rect(x, y, 1., h);
        f.rect(x + w - 1., y, 1., h);
        if tk.a < 0.97 {
            return;
        }
        f.fill_hex("#ff8a9a");
        px(f, x + 2., y + 2.);
        f.fill_hex("#ffd27a");
        px(f, x + 4., y + 2.);
        f.fill_hex("#7dffb0");
        px(f, x + 6., y + 2.);
        for (j, l) in tk.lines.iter().enumerate() {
            let lw = (l.w * clamp(tk.p * 3. - j as f64)).floor();
            if lw > 0. {
                f.fill(l.c);
                f.rect(x + 2. + l.ind, y + 5. + j as f64 * 2., lw.min(w - 4. - l.ind), 1.);
            }
        }
        if !done {
            f.fill_hex("#1e4f8f");
            f.rect(x + 2., y + h - 3., w - 4., 1.);
            f.fill_hex("#7fd6ff");
            f.rect(x + 2., y + h - 3., jround((w - 4.) * tk.p), 1.);
            if (self.t * 8.).sin() > 0. {
                f.fill_hex("#ffffff");
                let j = ((tk.p * 3.).floor() as usize).min(2);
                let l = &tk.lines[j];
                px(f, x + 2. + l.ind + (l.w * clamp(tk.p * 3. - j as f64)).floor(), y + 5. + j as f64 * 2.);
            }
        } else {
            f.fill_hex("#9dffd0");
            let (c, r) = (x + w - 7., y + h - 4.);
            px(f, c, r);
            px(f, c + 1., r + 1.);
            px(f, c + 2., r);
            px(f, c + 3., r - 1.);
            px(f, c + 4., r - 2.);
        }
    }

    fn draw_clam(&self, f: &mut Frame, gl: &Glows, x: f64, y: f64) {
        let c = &self.clam;
        if c.rise < 0.02 {
            return;
        }
        let (x, y) = (jround(x), jround(y));
        let (open, lift) = (c.open, jround(c.open * 7.));
        if open > 0.1 {
            gl.draw(f, Gl::Pink, x, y - 3., 22. + c.flash * 10., 0.25 + open * 0.25 + c.flash * 0.4);
        }
        // bottom shell
        let (b1, b2, b3) = (hex("#b86a9c"), hex("#d28ab4"), hex("#eeb0d2"));
        for dy in 0..=5 {
            let w = jround(13. * (1. - (dy as f64 / 6.).powi(2)).sqrt()) as i64;
            for i in -w..=w {
                f.fill(if (i.abs() + 1) % 4 == 0 { b1 } else if dy > 3 { b2 } else { b3 });
                f.rect(x + i as f64, y + dy as f64, 1., 1.);
            }
        }
        f.fill_hex("#5a2a4c");
        f.rect(x - 13., y, 27., 1.);
        // pearls inside
        for j in 0..c.count {
            let px2 = x - 8. + (j % 4) as f64 * 5. + if j > 3 { 2. } else { 0. };
            let py2 = y - 2. - if j > 3 { 2. } else { 0. };
            f.fill_hex("#f6eeff");
            f.rect(px2, py2, 2., 2.);
            f.fill_hex("#ffffff");
            f.rect(px2, py2, 1., 1.);
        }
        // top shell, hinged open
        let (t1, t2, t3) = (hex("#c47aa8"), hex("#f6c6e0"), hex("#eaa6cc"));
        for dy in -5i64..=0 {
            let w = jround(13. * (1. - (dy as f64 / 6.).powi(2)).sqrt()) as i64;
            for i in -w..=w {
                f.fill(if (i.abs() + 1) % 4 == 0 { t1 } else if dy < -3 { t2 } else { t3 });
                f.rect(x + i as f64, y + dy as f64 - lift, 1., 1.);
            }
        }
        f.fill_hex("#5a2a4c");
        f.rect(x - 13., y - lift, 27., 1.);
    }
}

/// The mock's `disc(x, y, r)` with the current fill.
fn disc(f: &mut Frame, x: f64, y: f64, r: f64) {
    let (x, y) = (jround(x), jround(y));
    let ri = r.ceil() as i64;
    for dy in -ri..=ri {
        let q = r * r - (dy * dy) as f64;
        if q < 0. {
            continue;
        }
        let w = (q.sqrt() + 0.25).floor();
        f.rect(x - w, y + dy as f64, 2. * w + 1., 1.);
    }
}
