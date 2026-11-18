//! Pixel-art toolkit shared by every backdrop scene: the mock's helpers (bayer dither,
//! value noise, fbm, ramps, `paint`, `painter`, `mix`…), CPU layers, the per-frame
//! draw recorder (`Frame`) and the viewport/parallax maths (`Vp`).
//!
//! Units: everything a scene does is in *art pixels* (1 art px = `px` logical px, 3 in
//! the mock). Static layers are painted once on the CPU, upscaled nearest-neighbour to
//! device pixels and uploaded as `RenderImage`s; animated details are recorded per
//! frame as rects / image draws and turned into GPUI quads + sprites.
#![allow(clippy::too_many_arguments)]

use gpui_kit::{Bounds, Corners, Hsla, Pixels, RenderImage, Rgba, Window, fill, point, px as lpx, size};
use std::{rc::Rc, sync::Arc};

pub type C3 = [u8; 3];
pub type C4 = [u8; 4];

// ---------------------------------------------------------------- maths

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
/// Ordered-dither threshold in 0..1 (the mock's `bay`).
#[inline]
pub fn bay(x: i64, y: i64) -> f64 {
    (BAYER[(y & 3) as usize][(x & 3) as usize] as f64 + 0.5) / 16.
}
#[inline]
pub fn bayf(x: f64, y: f64) -> f64 {
    bay(x as i64, y as i64)
}
/// JavaScript's Math.round (halves go up).
#[inline]
pub fn jround(v: f64) -> f64 {
    (v + 0.5).floor()
}
#[inline]
pub fn clamp(v: f64) -> f64 {
    v.clamp(0., 1.)
}
#[inline]
pub fn clampr(v: f64, a: f64, b: f64) -> f64 {
    v.max(a).min(b)
}
pub fn sstep(a: f64, b: f64, v: f64) -> f64 {
    let v = clamp((v - a) / (b - a));
    v * v * (3. - 2. * v)
}
pub fn hex(h: &str) -> C3 {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    [p(0), p(2), p(4)]
}
pub fn mix(a: C3, b: C3, f: f64) -> C3 {
    let m = |i: usize| jround(a[i] as f64 + (b[i] as f64 - a[i] as f64) * f).clamp(0., 255.) as u8;
    [m(0), m(1), m(2)]
}
#[inline]
pub fn op(c: C3) -> Option<C4> {
    Some([c[0], c[1], c[2], 255])
}

/// The mock's seeded `rng(s)` (a mulberry-style generator), bit-for-bit.
pub struct Rng(u32);
impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed)
    }
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        let s = self.0;
        self.0 = (s ^ (s >> 15)).wrapping_mul(1 | s).wrapping_add(0x6d2b79f5);
        self.0 as f64 / 4294967296.
    }
}

/// Stand-in for `Math.random()` used by the scenes' update steps.
pub fn rand() -> f64 {
    use std::cell::Cell;
    thread_local!(static S: Cell<u64> = const { Cell::new(0x9e3779b97f4a7c15) });
    S.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// Value noise with optional horizontal period (the mock's `vnoise(seed)`).
#[derive(Clone, Copy)]
pub struct Noise(u32);
impl Noise {
    pub fn new(seed: u32) -> Self {
        Noise(seed)
    }
    #[inline]
    fn h(&self, x: i64, y: i64) -> f64 {
        let mut n = (x as i32 as u32)
            .wrapping_mul(374761393)
            .wrapping_add((y as i32 as u32).wrapping_mul(668265263))
            .wrapping_add(self.0.wrapping_mul(982451653));
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        (n ^ (n >> 16)) as f64 / 4294967296.
    }
    pub fn at(&self, x: f64, y: f64) -> f64 {
        self.p(x, y, 0.)
    }
    pub fn p(&self, x: f64, y: f64, period: f64) -> f64 {
        let xi = x.floor();
        let yi = y.floor();
        let (xf, yf) = (x - xi, y - yi);
        let u = xf * xf * (3. - 2. * xf);
        let v = yf * yf * (3. - 2. * yf);
        let w = |i: f64| -> i64 {
            if period != 0. { (((i % period) + period) % period) as i64 } else { i as i64 }
        };
        let (xi0, yi0) = (w(xi), yi as i64);
        let xi1 = w(xi + 1.);
        let a = self.h(xi0, yi0);
        let b = self.h(xi1, yi0);
        let c = self.h(xi0, yi0 + 1);
        let d = self.h(xi1, yi0 + 1);
        a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
    }
}
pub fn fbm(n: Noise, x: f64, y: f64, o: u32) -> f64 {
    fbmp(n, x, y, o, 0.)
}
pub fn fbmp(n: Noise, mut x: f64, mut y: f64, o: u32, mut p: f64) -> f64 {
    let (mut s, mut a, mut tot) = (0., 0.5, 0.);
    for _ in 0..o {
        s += n.p(x, y, p) * a;
        tot += a;
        x *= 2.;
        y *= 2.;
        p *= 2.;
        a *= 0.5;
    }
    s / tot
}

pub type Stops = Vec<(f64, C3)>;
pub fn stops(a: &[(f64, &str)]) -> Stops {
    a.iter().map(|(p, c)| (*p, hex(c))).collect()
}
/// Dithered gradient lookup (the mock's `ramp`).
pub fn ramp(st: &Stops, p: f64, x: i64, y: i64) -> C3 {
    let p = clamp(p);
    let mut i = 0;
    while i < st.len() - 2 && p > st[i + 1].0 {
        i += 1;
    }
    let (p0, c0) = st[i];
    let (p1, c1) = st[i + 1];
    let d = if p1 - p0 == 0. { 1. } else { p1 - p0 };
    if (p - p0) / d > bay(x, y) { c1 } else { c0 }
}

// ---------------------------------------------------------------- CPU layers

/// An art-resolution RGBA buffer (straight alpha), like the mock's offscreen canvases.
#[derive(Clone)]
pub struct Layer {
    pub w: usize,
    pub h: usize,
    pub d: Vec<C4>,
}
impl Layer {
    pub fn new(w: usize, h: usize) -> Self {
        let (w, h) = (w.max(1), h.max(1));
        Layer { w, h, d: vec![[0; 4]; w * h] }
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> C4 {
        self.d[y * self.w + x]
    }
    /// `fillRect` with an opaque colour, rounding like the mock's `painter`.
    pub fn rect(&mut self, c: C3, x: f64, y: f64, w: f64, h: f64) {
        let x0 = jround(x) as i64;
        let y0 = jround(y) as i64;
        let w = jround(w).max(1.) as i64;
        let h = jround(h).max(1.) as i64;
        self.rect_i(c, x0, y0, w, h);
    }
    pub fn rect_i(&mut self, c: C3, x0: i64, y0: i64, w: i64, h: i64) {
        let xa = x0.max(0);
        let xb = (x0 + w).min(self.w as i64);
        let ya = y0.max(0);
        let yb = (y0 + h).min(self.h as i64);
        for y in ya..yb {
            for x in xa..xb {
                self.d[y as usize * self.w + x as usize] = [c[0], c[1], c[2], 255];
            }
        }
    }
    pub fn px(&mut self, c: C3, x: f64, y: f64) {
        self.rect(c, x, y, 1., 1.)
    }
}
/// The mock's `paint(w, h, fn)`.
pub fn paint(w: usize, h: usize, mut f: impl FnMut(i64, i64) -> Option<C4>) -> Layer {
    let mut l = Layer::new(w, h);
    for y in 0..l.h {
        for x in 0..l.w {
            if let Some(c) = f(x as i64, y as i64) {
                l.d[y * l.w + x] = c;
            }
        }
    }
    l
}

/// A layer uploaded to the GPU: `art` is the (cropped) CPU copy, placed at (`x`,`y`)
/// inside the scene layer; `img` is the nearest-neighbour upscaled BGRA image.
pub struct Img {
    pub img: Arc<RenderImage>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub art: Layer,
}

/// Turns CPU layers into GPU images at a given device scale and remembers them so the
/// engine can drop them from the sprite atlas when the scene goes away.
pub struct Builder {
    /// device pixels per art pixel
    pub scale: f32,
    pub made: Vec<Arc<RenderImage>>,
}
impl Builder {
    pub fn new(scale: f32) -> Self {
        Builder { scale, made: vec![] }
    }
    /// Upload a whole layer, cropped to its opaque bounding box.
    pub fn img(&mut self, l: Layer) -> Rc<Img> {
        let (mut x0, mut y0, mut x1, mut y1) = (l.w, l.h, 0, 0);
        for y in 0..l.h {
            for x in 0..l.w {
                if l.d[y * l.w + x][3] != 0 {
                    x0 = x0.min(x);
                    x1 = x1.max(x + 1);
                    y0 = y0.min(y);
                    y1 = y1.max(y + 1);
                }
            }
        }
        if x1 <= x0 || y1 <= y0 {
            (x0, y0, x1, y1) = (0, 0, 1, 1);
        }
        let mut c = Layer::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            let src = &l.d[y * l.w + x0..y * l.w + x1];
            c.d[(y - y0) * c.w..(y - y0 + 1) * c.w].copy_from_slice(src);
        }
        self.img_raw(c, x0 as f64, y0 as f64, 1)
    }
    /// Upload without cropping; `vrep` repeats each row (used for textures that get
    /// stretched vertically so GPU linear filtering stays close to nearest).
    pub fn img_raw(&mut self, l: Layer, x: f64, y: f64, vrep: usize) -> Rc<Img> {
        let s = (self.scale.round() as usize).max(1);
        let sy = s * vrep.max(1);
        let (dw, dh) = (l.w * s, l.h * sy);
        let mut buf = vec![0u8; dw * dh * 4];
        for ay in 0..l.h {
            // one device row, then replicate it
            let row0 = ay * sy * dw * 4;
            for ax in 0..l.w {
                let c = l.d[ay * l.w + ax];
                // premultiply? GPUI sprites expect straight BGRA.
                let px = [c[2], c[1], c[0], c[3]];
                for k in 0..s {
                    let o = row0 + (ax * s + k) * 4;
                    buf[o..o + 4].copy_from_slice(&px);
                }
            }
            for r in 1..sy {
                let (a, b) = buf.split_at_mut(row0 + r * dw * 4);
                b[..dw * 4].copy_from_slice(&a[row0..row0 + dw * 4]);
            }
        }
        let im = image::RgbaImage::from_raw(dw as u32, dh as u32, buf).expect("image buffer");
        let ri = Arc::new(RenderImage::new(vec![image::Frame::new(im)]));
        self.made.push(ri.clone());
        Rc::new(Img { img: ri, x, y, w: l.w as f64, h: l.h as f64, art: l })
    }
}

// ---------------------------------------------------------------- viewport

/// Shared viewport state (the mock's W, H, pad, padY, FW, FH, S, mx, my, SX, SY).
#[derive(Clone, Copy, Debug)]
pub struct Vp {
    pub w: f64,
    pub h: f64,
    pub pad: f64,
    pub pad_y: f64,
    pub fw: usize,
    pub fh: usize,
    /// snapping resolution: device pixels per art pixel
    pub s: f64,
    pub sx: f64,
    pub sy: f64,
    pub mx: f64,
    pub my: f64,
}
impl Vp {
    pub fn new(w: usize, h: usize, s: f64, shift: (f64, f64)) -> Self {
        let pad = shift.0 + 2.;
        let pad_y = shift.1 + 2.;
        Vp { w: w as f64, h: h as f64, pad, pad_y, fw: w + 2 * pad as usize, fh: h + 2 * pad_y as usize, s, sx: shift.0, sy: shift.1, mx: 0., my: 0. }
    }
    #[inline]
    pub fn x(&self, f: f64) -> f64 {
        self.pad + f * self.w
    }
    #[inline]
    pub fn y(&self, f: f64) -> f64 {
        self.pad_y + f * self.h
    }
    #[inline]
    pub fn snap(&self, v: f64) -> f64 {
        jround(v * self.s) / self.s
    }
    pub fn sh(&self, d: f64) -> f64 {
        self.snap(-self.mx * d * self.sx)
    }
    pub fn shy(&self, d: f64) -> f64 {
        self.snap(-self.my * d * self.sy)
    }
    /// Layer coordinates → screen art coordinates at depth `d`.
    pub fn at(&self, x: f64, y: f64, d: f64) -> (f64, f64) {
        (self.snap(x - self.pad + self.sh(d)), self.snap(y - self.pad_y + self.shy(d)))
    }
}

// ---------------------------------------------------------------- per-frame recorder

#[derive(Clone)]
pub enum Op {
    /// art-space rect with straight-alpha colour (0..1)
    Rect { x: f32, y: f32, w: f32, h: f32, c: [f32; 4] },
    /// image drawn into `dst` (art space), optionally clipped to `clip`
    Img { img: Rc<Img>, dst: [f32; 4], clip: Option<[f32; 4]> },
}

#[derive(Clone)]
pub struct Pass {
    pub opacity: f32,
    pub ops: Vec<Op>,
}

/// Records one frame of drawing with a canvas-like API (`fill`, `alpha`, `rect`,
/// `layer`…). Image draws with a global alpha become their own opacity pass, since
/// GPUI applies opacity per element.
pub struct Frame {
    pub passes: Vec<Pass>,
    pub fill: C3,
    pub alpha: f64,
    snap: f64,
    pass_op: f32,
}
impl Frame {
    pub fn new(snap: f64) -> Self {
        Frame { passes: vec![Pass { opacity: 1., ops: vec![] }], fill: [0, 0, 0], alpha: 1., snap, pass_op: 1. }
    }
    fn sn(&self, v: f64) -> f32 {
        (jround(v * self.snap) / self.snap) as f32
    }
    /// Route everything drawn next into a pass with this opacity (GPUI applies opacity
    /// per element). Group same-alpha draws to keep the pass count low.
    pub fn set_pass(&mut self, opacity: f64) {
        self.pass_op = ((opacity.clamp(0., 1.) * 1000.).round() / 1000.) as f32;
    }
    fn cur(&mut self, opacity: f32) -> &mut Vec<Op> {
        if self.passes.last().map(|p| p.opacity) != Some(opacity) {
            self.passes.push(Pass { opacity, ops: vec![] });
        }
        &mut self.passes.last_mut().unwrap().ops
    }
    pub fn fill(&mut self, c: C3) -> &mut Self {
        self.fill = c;
        self
    }
    pub fn fill_hex(&mut self, c: &str) -> &mut Self {
        self.fill = hex(c);
        self
    }
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let a = self.alpha as f32;
        if a <= 0.003 || w <= 0. || h <= 0. {
            return;
        }
        let f = self.fill;
        self.rect_c(x, y, w, h, [f[0] as f32 / 255., f[1] as f32 / 255., f[2] as f32 / 255., a.min(1.)]);
    }
    pub fn rect_c(&mut self, x: f64, y: f64, w: f64, h: f64, c: [f32; 4]) {
        let (x0, y0) = (self.sn(x), self.sn(y));
        let (x1, y1) = (self.sn(x + w), self.sn(y + h));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let ops = self.cur(self.pass_op);
        // merge with the previous rect when it continues it horizontally or vertically
        if let Some(Op::Rect { x, y, w, h, c: pc }) = ops.last_mut() {
            if *pc == c {
                if *y == y0 && *h == y1 - y0 && *x + *w == x0 {
                    *w += x1 - x0;
                    return;
                }
                if *x == x0 && *w == x1 - x0 && *y + *h == y0 {
                    *h += y1 - y0;
                    return;
                }
            }
        }
        ops.push(Op::Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0, c });
    }
    /// Draw a whole uploaded layer whose layer origin lands at screen art (x, y).
    pub fn image(&mut self, img: &Rc<Img>, x: f64, y: f64, opacity: f64) {
        let dst = [self.sn(x + img.x), self.sn(y + img.y), img.w as f32, img.h as f32];
        self.image_dst(img, dst, None, opacity);
    }
    pub fn image_dst(&mut self, img: &Rc<Img>, dst: [f32; 4], clip: Option<[f32; 4]>, opacity: f64) {
        if opacity <= 0.003 {
            return;
        }
        let prev = self.pass_op;
        let op = ((opacity.min(1.) * 1000.).round() / 1000.) as f32 * prev;
        self.cur(op).push(Op::Img { img: img.clone(), dst, clip });
        if op != prev {
            self.passes.push(Pass { opacity: prev, ops: vec![] });
        }
    }
    /// The mock's `layer(c, d)`: a full-size scene layer with parallax depth `d`.
    pub fn layer(&mut self, vp: &Vp, img: &Rc<Img>, d: f64, opacity: f64) {
        self.image(img, -vp.pad + vp.sh(d), -vp.pad_y + vp.shy(d), opacity);
    }
    /// The mock's `px(x, y, d, w, h)`.
    pub fn px(&mut self, vp: &Vp, x: f64, y: f64, d: f64, w: f64, h: f64) {
        let (a, b) = vp.at(x, y, d);
        self.rect(a, b, w, h);
    }
    pub fn finish(mut self) -> Vec<Pass> {
        self.passes.retain(|p| !p.ops.is_empty());
        self.passes
    }
}

/// Paint recorded ops inside `bounds` (1 art px = `unit` logical px).
pub fn paint_ops(ops: &[Op], bounds: Bounds<Pixels>, unit: f32, window: &mut Window) {
    let o = bounds.origin;
    let r = |x: f32, y: f32, w: f32, h: f32| Bounds { origin: point(o.x + lpx(x * unit), o.y + lpx(y * unit)), size: size(lpx(w * unit), lpx(h * unit)) };
    for op in ops {
        match op {
            Op::Rect { x, y, w, h, c } => {
                let col: Hsla = Rgba { r: c[0], g: c[1], b: c[2], a: c[3] }.into();
                window.paint_quad(fill(r(*x, *y, *w, *h), col));
            }
            Op::Img { img, dst, clip } => {
                let ib = r(dst[0], dst[1], dst[2], dst[3]);
                let b = match clip {
                    Some(c) => r(c[0], c[1], c[2], c[3]),
                    None => ib,
                };
                window.paint_image(b, ib, Corners::default(), img.img.clone(), 0, false).ok();
            }
        }
    }
}

/// Software-composite recorded passes into a W×H art buffer (RGB 0..1) — used for
/// the theme's Auto extraction (the mock reads the canvas back).
pub fn composite(passes: &[Pass], w: usize, h: usize) -> Vec<[f32; 3]> {
    let mut out = vec![[0f32; 3]; w * h];
    let mut blend = |x: i64, y: i64, c: [f32; 3], a: f32| {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 || a <= 0. {
            return;
        }
        let p = &mut out[y as usize * w + x as usize];
        for i in 0..3 {
            p[i] += (c[i] - p[i]) * a.min(1.);
        }
    };
    for pass in passes {
        for op in &pass.ops {
            match op {
                Op::Rect { x, y, w: rw, h: rh, c } => {
                    let (x0, y0) = (x.round() as i64, y.round() as i64);
                    let (x1, y1) = ((x + rw).round() as i64, (y + rh).round() as i64);
                    for yy in y0..y1.max(y0 + 1) {
                        for xx in x0..x1.max(x0 + 1) {
                            blend(xx, yy, [c[0], c[1], c[2]], c[3] * pass.opacity);
                        }
                    }
                }
                Op::Img { img, dst, clip } => {
                    let [dx, dy, dw, dh] = *dst;
                    let (mut cx0, mut cy0, mut cx1, mut cy1) = (dx, dy, dx + dw, dy + dh);
                    if let Some([a, b, c, d]) = clip {
                        cx0 = cx0.max(*a);
                        cy0 = cy0.max(*b);
                        cx1 = cx1.min(a + c);
                        cy1 = cy1.min(b + d);
                    }
                    let l = &img.art;
                    for yy in (cy0.round() as i64).max(0)..(cy1.round() as i64).min(h as i64) {
                        for xx in (cx0.round() as i64).max(0)..(cx1.round() as i64).min(w as i64) {
                            let u = (((xx as f32 + 0.5 - dx) / dw) * l.w as f32).floor() as i64;
                            let v = (((yy as f32 + 0.5 - dy) / dh) * l.h as f32).floor() as i64;
                            if u < 0 || v < 0 || u >= l.w as i64 || v >= l.h as i64 {
                                continue;
                            }
                            let p = l.get(u as usize, v as usize);
                            blend(xx, yy, [p[0] as f32 / 255., p[1] as f32 / 255., p[2] as f32 / 255.], p[3] as f32 / 255. * pass.opacity);
                        }
                    }
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- per-frame raster

/// Something the octopus (and other sprites) can be painted into, in integer art px.
pub trait PixelTarget {
    /// Source-over fill of a w×h block with colour `c` at alpha `a`.
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: C3, a: f32);
}

/// A small premultiplied RGBA buffer for per-frame sprites; turned into run-length
/// quads with [`Raster::emit`].
pub struct Raster {
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    buf: Vec<[f32; 4]>,
}
impl Raster {
    pub fn new(x0: i32, y0: i32, w: i32, h: i32) -> Self {
        Raster { x0, y0, w, h, buf: vec![[0.; 4]; (w.max(1) * h.max(1)) as usize] }
    }
    /// Push the raster into a frame as quads, offset by (`ox`, `oy`) art px.
    pub fn emit(&self, f: &mut Frame, ox: f64, oy: f64) {
        let q = |p: [f32; 4]| -> [u8; 4] { [(p[0] * 255.).round() as u8, (p[1] * 255.).round() as u8, (p[2] * 255.).round() as u8, (p[3] * 255.).round() as u8] };
        for y in 0..self.h {
            let row = &self.buf[(y * self.w) as usize..((y + 1) * self.w) as usize];
            let mut x = 0;
            while x < self.w {
                let p = q(row[x as usize]);
                let mut e = x + 1;
                while e < self.w && q(row[e as usize]) == p {
                    e += 1;
                }
                if p[3] > 0 {
                    let a = p[3] as f32 / 255.;
                    let c = [p[0] as f32 / 255. / a, p[1] as f32 / 255. / a, p[2] as f32 / 255. / a, a];
                    f.rect_c(ox + (self.x0 + x) as f64, oy + (self.y0 + y) as f64, (e - x) as f64, 1., [c[0].min(1.), c[1].min(1.), c[2].min(1.), c[3]]);
                }
                x = e;
            }
        }
    }
}
impl PixelTarget for Raster {
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: C3, a: f32) {
        let (xa, ya) = ((x - self.x0).max(0), (y - self.y0).max(0));
        let (xb, yb) = ((x + w - self.x0).min(self.w), (y + h - self.y0).min(self.h));
        let s = [c[0] as f32 / 255. * a, c[1] as f32 / 255. * a, c[2] as f32 / 255. * a, a];
        for yy in ya..yb {
            for xx in xa..xb {
                let p = &mut self.buf[(yy * self.w + xx) as usize];
                for i in 0..4 {
                    p[i] = s[i] + p[i] * (1. - a);
                }
            }
        }
    }
}
/// Paint straight into a static layer (opaque fills only; `a` is honoured by mixing).
impl PixelTarget for Layer {
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: C3, a: f32) {
        for yy in y.max(0)..(y + h).min(self.h as i32) {
            for xx in x.max(0)..(x + w).min(self.w as i32) {
                let p = &mut self.d[yy as usize * self.w + xx as usize];
                if p[3] == 0 {
                    *p = [c[0], c[1], c[2], (a * 255.).round() as u8];
                    continue;
                }
                let m = mix([p[0], p[1], p[2]], c, a as f64);
                let na = (p[3] as f32 + (255. - p[3] as f32) * a).round() as u8;
                *p = [m[0], m[1], m[2], na];
            }
        }
    }
}
