//! AURORA — polar night: aurora curtains over a frozen lake, snowy pines, a cabin
//! with a lit window.
use super::Scene;
use super::px::*;
use std::rc::Rc;

struct Star {
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
    big: bool,
}
struct Band {
    y: f64,
    amp: f64,
    f1: f64,
    f2: f64,
    h: f64,
    a: f64,
    ph: f64,
}
struct Smoke {
    x: f64,
    y: f64,
    age: f64,
    life: f64,
    ph: f64,
}
struct Shoot {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
}
#[derive(Default, Clone, Copy)]
struct Rect4 {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Default)]
pub struct Aurora {
    k: f64,
    lake_y: f64,
    lake_b: f64,
    rh: f64,
    lh: f64,
    sky: Option<Rc<Img>>,
    ray: Option<Rc<Img>>,
    far: Option<Rc<Img>>,
    near: Option<Rc<Img>>,
    lake: Option<Rc<Img>>,
    lake_glow: Option<Rc<Img>>,
    fg: Option<Rc<Img>>,
    win_glow: Option<Rc<Img>>,
    stars: Vec<Star>,
    bands: Vec<Band>,
    chim: (f64, f64),
    win: Rect4,
    smoke: Vec<Smoke>,
    ss: Option<Shoot>,
    next_ss: f64,
}

pub fn new() -> Box<dyn Scene> {
    Box::<Aurora>::default()
}

struct RangePal {
    rock: C3,
    rock_s: C3,
    snow: C3,
    snow_s: C3,
    haze: C3,
    rim: C3,
}
fn rp(a: [&str; 6]) -> RangePal {
    RangePal { rock: hex(a[0]), rock_s: hex(a[1]), snow: hex(a[2]), snow_s: hex(a[3]), haze: hex(a[4]), rim: hex(a[5]) }
}

impl Aurora {
    fn inten(b: &Band, u: f64, t: f64) -> f64 {
        let slow = 0.5 + 0.5 * (u * 0.019 - t * 0.28 + b.ph).sin();
        let fold = 0.5 + 0.5 * (u * 0.047 + t * 0.2 + b.ph * 2.).sin();
        let rays = 0.55 + 0.45 * (u * 0.85 + (u * 0.11 + t * 0.6).sin() * 4.).sin();
        clamp(slow * 0.65 + fold * 0.35).powf(1.4) * rays
    }
}

impl Scene for Aurora {
    fn build(&mut self, b: &mut Builder, v: &Vp, t: f64) {
        let (fw, fh, w, h) = (v.fw, v.fh, v.w, v.h);
        let mut r = Rng::new(41);
        let n = Noise::new(17);
        let k = w / 400.;
        let hz = v.y(0.6);
        let lake_y = jround(v.y(0.67));
        let lake_b = jround(v.y(0.8));
        self.k = k;
        self.lake_y = lake_y;
        self.lake_b = lake_b;

        // Sky and stars
        let sk = stops(&[(0., "#04061a"), (0.45, "#0a1134"), (0.8, "#162256"), (1., "#24346e")]);
        let mut sky = paint(fw, fh, |x, y| op(ramp(&sk, y as f64 / hz, x, y)));
        let (sb, sd) = (hex("#b8c4f4"), hex("#56639e"));
        let nst = fw as f64 * hz * 0.006;
        let mut i = 0.;
        while i < nst {
            i += 1.;
            let x = r.next() * fw as f64;
            let y = r.next() * hz * 0.92;
            let bb = r.next();
            if bb < 0.14 {
                let ph = r.next() * 6.;
                let sp = 0.8 + r.next() * 2.2;
                let big = r.next() < 0.25;
                self.stars.push(Star { x, y, ph, sp, big });
            } else {
                sky.px(if bb > 0.75 { sb } else { sd }, x, y);
            }
        }
        self.sky = Some(b.img(sky));

        // Aurora ray texture: bright mint hem at the bottom, fading through teal to violet
        let rh = (h * 0.42).round().max(10.);
        self.rh = rh;
        let ast = stops(&[(0., "#eafff4"), (0.05, "#86f7c6"), (0.22, "#38d99c"), (0.45, "#22a8a2"), (0.7, "#6a54d8"), (1., "#b45ad2")]);
        let ray = paint(4, rh as usize, |x, y| {
            let vv = 1. - y as f64 / (rh - 1.);
            let a = if vv < 0.035 { 0.55 + vv / 0.035 * 0.45 } else { (1. - vv).powf(1.35) };
            let q = (a * 6. + bay(x, y)).floor() / 6.;
            if q <= 0. {
                return None;
            }
            let c = ramp(&ast, vv, x, y);
            Some([c[0], c[1], c[2], jround(q.min(1.) * 255.) as u8])
        });
        // rows repeated so the GPU's linear stretch stays (nearly) nearest-neighbour
        self.ray = Some(b.img_raw(ray, 0., 0., 8));
        self.bands = vec![
            Band { y: v.y(0.36), amp: h * 0.06, f1: 0.012, f2: 0.031, h: 1., a: 0.9, ph: 0. },
            Band { y: v.y(0.24), amp: h * 0.045, f1: 0.017, f2: 0.009, h: 0.7, a: 0.5, ph: 2.3 },
        ];

        // Mountains: a far snowy range and a darker near one
        let ridge = |seed: u32, base: f64, lo: f64, amp: f64, sc: f64| -> Vec<f64> {
            let nn = Noise::new(seed);
            (0..fw)
                .map(|x| {
                    let xf = x as f64;
                    let f = ((fbm(nn, xf / sc, 0.5, 3) - 0.5) * 2.4).clamp(-1., 1.);
                    let rd = 1. - f.abs();
                    base - lo - rd * rd * amp - (nn.at(xf / 3., 9.5) - 0.5) * 1.5
                })
                .collect()
        };
        let range = |top: &[f64], base: f64, kp: &RangePal, seed: u32, rim: bool| -> Layer {
            let nn = Noise::new(seed);
            paint(fw, fh, |x, y| {
                let (xf, yf) = (x as f64, y as f64);
                let xu = x as usize;
                if yf < top[xu] || yf > base {
                    return None;
                }
                let bb = bay(x, y);
                let d = yf - top[xu];
                let lit = (top[(xu + 4).min(fw - 1)] - top[xu.saturating_sub(4)]) > 0.5;
                let sd = h * (0.025 + fbm(nn, xf / 6., yf / 9., 2) * 0.07);
                let snow = d < sd || (d < sd * 2.2 && nn.at(xf / 4., yf / 3.) > 0.64);
                let mut c = if snow {
                    if lit { kp.snow } else { kp.snow_s }
                } else if lit {
                    kp.rock
                } else {
                    kp.rock_s
                };
                if !snow && nn.at(xf / 4., yf / 4. + 30.) > 0.72 {
                    c = mix(c, kp.snow_s, 0.35);
                }
                if rim && d < 1. && lit {
                    c = kp.rim;
                }
                let hk = (yf - (base - h * 0.1)) / (h * 0.1);
                if hk > bb {
                    c = mix(c, kp.haze, clampr(0.25 + hk * 0.4, 0., 0.7));
                }
                op(c)
            })
        };
        let far_top = ridge(5, hz + 2., h * 0.05, h * 0.24, 70. * k);
        self.far = Some(b.img(range(&far_top, hz + 2., &rp(["#222c5e", "#1a2352", "#93a3d8", "#5d6ca6", "#2c3c78", "#a8f0d8"]), 6, true)));
        let near_top = ridge(9, lake_y, h * 0.07, h * 0.12, 46. * k);
        self.near = Some(b.img(range(&near_top, lake_y, &rp(["#131a3c", "#0e1332", "#6e80ba", "#414f88", "#1c2a5c", "#7fe6c4"]), 12, true)));

        // Frozen lake with the near range mirrored in the ice
        let (ice, streak, deep, refl, lrim) = (hex("#16224e"), hex("#22336a"), hex("#0f1838"), hex("#0a1030"), hex("#3a4c8c"));
        self.lake = Some(b.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if yf < lake_y || yf > lake_b + 6. {
                return None;
            }
            let bb = bay(x, y);
            let m = 2. * lake_y - yf;
            let dy = (yf - lake_y) as i64;
            let mut c = if dy % 3 == 0 && fbm(n, xf / 16., yf / 2., 2) > 0.52 { streak } else { ice };
            if yf == lake_y {
                c = lrim;
            }
            if m >= near_top[x as usize] && yf > lake_y {
                c = mix(c, refl, if dy % 2 != 0 { 0.8 } else { 0.55 });
            }
            let dp = (yf - lake_y) / (lake_b - lake_y);
            if dp * 0.7 > bb {
                c = mix(c, deep, 0.35);
            }
            op(c)
        })));
        let lh = lake_b - lake_y + 4.;
        self.lh = lh;
        let lg = paint(4, lh as usize, |x, y| {
            if y % 3 == 2 {
                return None;
            }
            let a = (1. - y as f64 / lh).powf(1.6);
            let q = (a * 4. + bay(x, y)).floor() / 4.;
            if q <= 0. {
                return None;
            }
            let al = jround(q * 255.) as u8;
            Some(if (y as f64) < lh * 0.35 { [120, 240, 196, al] } else { [96, 140, 220, al] })
        });
        self.lake_glow = Some(b.img_raw(lg, 0., 0., 1));

        // Foreground: snow bank, pines, a little cabin
        let fg_top: Vec<f64> = (0..fw)
            .map(|x| {
                let xf = x as f64;
                let e = (xf - v.x(0.5)).abs() / (w * 0.5);
                v.y(0.79) + (fbm(n, xf / (34. * k), 7.1, 3) - 0.5) * h * 0.07 - e * e * h * 0.06
            })
            .collect();
        let (sa, slit, sshade, sdeep, smint) = (hex("#b4c2e8"), hex("#dbe5ff"), hex("#7a8abe"), hex("#5a6aa4"), hex("#b8ecdc"));
        let mut fg = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let xu = x as usize;
            if yf < fg_top[xu] {
                return None;
            }
            let bb = bay(x, y);
            let d = yf - fg_top[xu];
            let sl = fg_top[(xu + 3).min(fw - 1)] - fg_top[xu.saturating_sub(3)];
            let mut c = sa;
            if d < 5. * k {
                c = if sl > 0.6 {
                    slit
                } else if sl < -0.6 {
                    sshade
                } else {
                    sa
                };
            }
            if d < 1. {
                c = smint;
            } else if fbm(n, xf / 9. + 40., yf / 3., 2) > 0.6 && bb > 0.4 {
                c = sshade;
            }
            let dp = (yf - v.y(0.84)) / (h * 0.2);
            if dp > bb {
                c = mix(c, sdeep, 0.35);
            }
            op(c)
        });
        let dark = hex("#070b22");
        let pine = |g: &mut Layer, x: f64, hh: f64, c: C3, cs: C3| {
            let x = jround(x);
            let bse = jround(fg_top[(x.max(0.) as usize).min(fw - 1)] + 2.);
            let tt = jround(hh / 7.).max(6.) as i64;
            for dy in 0..hh as i64 {
                let tier = dy % tt;
                let hw = (dy as f64 * 0.34 * (0.45 + tier as f64 / tt as f64 * 0.6)).floor() + 1.;
                let yy = bse - hh + dy as f64;
                g.rect(c, x - hw, yy, hw * 2. + 1., 1.);
                if tier < 2 && dy > 3 {
                    g.rect(cs, x - hw + if tier != 0 { 1. } else { 0. }, yy, jround(hw * if tier != 0 { 0.9 } else { 1.3 }).max(1.), 1.);
                }
                if tier == tt - 1 {
                    g.rect(dark, x - hw, yy, hw * 2. + 1., 1.);
                }
            }
            g.rect(c, x - 1., bse, 3., 3.);
        };
        let mut trees: Vec<(f64, f64)> = vec![
            (-0.03, 0.46), (0.02, 0.34), (0.065, 0.42), (0.11, 0.26), (0.145, 0.2), (0.04, 0.22),
            (0.86, 0.22), (0.9, 0.36), (0.95, 0.46), (0.995, 0.3), (0.93, 0.2),
        ];
        trees.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let (pa, pb, pcs) = (hex("#0b1230"), hex("#0e1638"), hex("#c3d0f0"));
        for (i, (f, th)) in trees.iter().enumerate() {
            pine(&mut fg, v.x(*f), jround(h * th), if i % 2 == 1 { pa } else { pb }, pcs);
        }
        // cabin
        let g = &mut fg;
        let cx = jround(v.x(0.765));
        let cw = jround(30. * k);
        let ch = jround(13. * k);
        let cb = jround(fg_top[cx as usize] + 3.);
        let cy = cb - ch;
        g.rect(hex("#2a1c2c"), cx, cy, cw, ch);
        let mut y = cy + 2.;
        while y < cb {
            g.rect(hex("#1c121e"), cx, y, cw, 1.);
            y += 3.;
        }
        g.rect(hex("#3c2a3a"), cx, cy, 1., ch);
        let rh2 = jround(cw * 0.4);
        for i in 0..rh2 as i64 {
            let i = i as f64;
            let ww = jround((i / rh2) * (cw + 8.));
            g.rect(if i < rh2 - 2. { hex("#d4def8") } else { hex("#9aa8d8") }, cx + cw / 2. - ww / 2., cy - rh2 + i, ww, 1.);
            if i > 1. {
                g.rect(hex("#eef3ff"), cx + cw / 2. - ww / 2., cy - rh2 + i, (ww * 0.35).max(1.), 1.);
            }
        }
        g.rect(hex("#3a2a3a"), cx - 4., cy - 1., cw + 8., 1.);
        let chx = jround(cx + cw * 0.72);
        let chy = jround(cy - rh2 * 0.75);
        g.rect(hex("#3a2a34"), chx, chy, 4., jround(rh2 * 0.5));
        g.rect(hex("#e6edff"), chx - 1., chy - 1., 6., 2.);
        self.chim = (chx + 2., chy - 2.);
        let wx = jround(cx + cw * 0.18);
        let wy = jround(cy + ch * 0.28);
        let ww = jround(7. * k).max(5.);
        let wh = jround(5. * k).max(4.);
        self.win = Rect4 { x: wx, y: wy, w: ww, h: wh };
        let dx = jround(cx + cw * 0.62);
        g.rect(hex("#140c16"), dx, cb - jround(ch * 0.75), jround(5. * k), jround(ch * 0.75));
        self.fg = Some(b.img(fg));
        let (gw, gh) = (ww * 5., wh * 4.);
        let wg = paint(jround(gw) as usize, jround(gh) as usize, |x, y| {
            let d = ((x as f64 - gw / 2.) / (gw / 2.)).hypot((y as f64 - gh * 0.42) / (gh / 2.));
            let f = clamp(1. - d);
            let q = (f * f * 4. + bay(x, y)).floor() / 4.;
            if q > 0. { Some([255, 176, 90, jround(q * 90.) as u8]) } else { None }
        });
        self.win_glow = Some(b.img_raw(wg, 0., 0., 1));

        self.smoke = vec![];
        self.ss = None;
        self.next_ss = t + 3. + r.next() * 4.;
    }

    fn update(&mut self, v: &Vp, t: f64, dt: f64) {
        if rand() < dt * 3. {
            self.smoke.push(Smoke { x: self.chim.0, y: self.chim.1, age: 0., life: 3. + rand() * 2., ph: rand() * 6. });
        }
        let k = self.k;
        self.smoke.retain_mut(|s| {
            s.age += dt;
            s.y -= dt * 4. * k;
            s.x += dt * (1.6 + (t + s.ph).sin() * 1.2) * k;
            s.age < s.life
        });
        if let Some(s) = &mut self.ss {
            s.age += dt;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            if s.age > s.life {
                self.ss = None;
                self.next_ss = t + 6. + rand() * 9.;
            }
        } else if t > self.next_ss {
            self.ss = Some(Shoot {
                x: v.x(0.12 + rand() * 0.6),
                y: v.y(0.04 + rand() * 0.14),
                vx: v.w * (0.45 + rand() * 0.25),
                vy: v.h * (0.12 + rand() * 0.1),
                age: 0.,
                life: 0.55 + rand() * 0.3,
            });
        }
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(sky) = &self.sky else { return };
        f.fill_hex("#04061a");
        f.rect(0., 0., v.w, v.h);
        f.layer(v, sky, 0.02, 1.);
        f.fill_hex("#eef2ff");
        for s in &self.stars {
            let a = 0.5 + 0.5 * (t * s.sp + s.ph).sin();
            f.alpha = 0.25 + a * 0.75;
            f.px(v, s.x, s.y, 0.02, 1., 1.);
            if s.big && a > 0.8 {
                f.alpha = (a - 0.8) * 3.;
                f.px(v, s.x - 1., s.y, 0.02, 1., 1.);
                f.px(v, s.x + 1., s.y, 0.02, 1., 1.);
                f.px(v, s.x, s.y - 1., 0.02, 1., 1.);
                f.px(v, s.x, s.y + 1., 0.02, 1., 1.);
            }
        }
        f.alpha = 1.;
        if let Some(s) = &self.ss {
            let fa = 1. - s.age / s.life;
            let len = 12. * self.k;
            let mut i = 0.;
            while i < len {
                f.alpha = fa * (1. - i / len);
                f.fill_hex(if i < 2. { "#ffffff" } else { "#bcd0ff" });
                f.px(v, s.x - s.vx / v.w * i * 2.2, s.y - s.vy / v.w * i * 2.2, 0.02, 1., 1.);
                i += 1.;
            }
            f.alpha = 1.;
        }
        // aurora curtains — "lighter" is approximated with source-over, grouped by alpha
        let ray = self.ray.as_ref().unwrap();
        let (ox, oy) = (v.sh(0.05), v.shy(0.05));
        let (rh, k) = (self.rh, self.k);
        let wn = v.w as usize;
        let mut lam = vec![0f64; wn + 1];
        let mut groups: Vec<Vec<([f32; 4], [f32; 4])>> = vec![vec![]; 9];
        for (bi, bd) in self.bands.iter().enumerate() {
            for sx in 0..=wn {
                let sxf = sx as f64;
                let u = (sxf + v.pad - ox) / k;
                let i = Self::inten(bd, u, t);
                if bi == 0 {
                    lam[sx] = i;
                }
                let a = jround(bd.a * i * 8.);
                if a <= 0. {
                    continue;
                }
                let yb = bd.y + bd.amp * ((u * bd.f1 + t * 0.15 + bd.ph).sin() * 0.7 + (u * bd.f2 - t * 0.09).sin() * 0.3);
                let hh = rh * bd.h * (0.55 + 0.45 * i);
                let top = v.snap(yb - v.pad_y + oy - hh);
                let c0 = (sx & 3) as f64;
                groups[a as usize].push(([(sxf - c0) as f32, top as f32, 4., hh as f32], [sxf as f32, top as f32, 1., hh as f32]));
            }
        }
        for (a, g) in groups.iter().enumerate() {
            if g.is_empty() {
                continue;
            }
            f.set_pass(a as f64 / 8.);
            for (dst, clip) in g {
                f.image_dst(ray, *dst, Some(*clip), 1.);
            }
        }
        f.set_pass(1.);
        f.layer(v, self.far.as_ref().unwrap(), 0.12, 1.);
        f.layer(v, self.near.as_ref().unwrap(), 0.25, 1.);
        f.layer(v, self.lake.as_ref().unwrap(), 0.25, 1.);
        // the aurora's glow on the ice
        let lg = self.lake_glow.as_ref().unwrap();
        let (_, ly) = v.at(0., self.lake_y + 1., 0.25);
        for level in 1..=6 {
            let mut any = false;
            for (sx, l) in lam.iter().enumerate() {
                if jround(l * 0.4 * 6.) as i32 != level {
                    continue;
                }
                if !any {
                    f.set_pass(level as f64 / 6.);
                    any = true;
                }
                let sxf = sx as f64;
                let c0 = (sx & 3) as f64;
                f.image_dst(lg, [(sxf - c0) as f32, ly as f32, 4., self.lh as f32], Some([sxf as f32, ly as f32, 1., self.lh as f32]), 1.);
            }
        }
        f.set_pass(1.);
        f.layer(v, self.fg.as_ref().unwrap(), 0.6, 1.);
        // cabin window + chimney smoke
        let w = self.win;
        let fl = 0.85 + 0.15 * (t * 5.3).sin() * (t * 2.1 + 1.).sin();
        f.fill_hex("#3a2420");
        f.px(v, w.x - 1., w.y - 1., 0.6, w.w + 2., w.h + 2.);
        f.alpha = fl;
        f.fill_hex("#ffc86a");
        f.px(v, w.x, w.y, 0.6, w.w, w.h);
        f.fill_hex("#ffe4a8");
        f.px(v, w.x, w.y, 0.6, (w.w / 2.).ceil() - 1., (w.h / 2.).ceil() - 1.);
        f.alpha = 1.;
        f.fill_hex("#3a2420");
        f.px(v, w.x + (w.w / 2.).floor(), w.y, 0.6, 1., w.h);
        f.px(v, w.x, w.y + (w.h / 2.).floor(), 0.6, w.w, 1.);
        let wg = self.win_glow.as_ref().unwrap();
        let (gx, gy) = v.at(w.x + w.w / 2. - wg.w / 2., w.y + w.h / 2. - wg.h * 0.42, 0.6);
        f.image(wg, gx, gy, 0.5 + fl * 0.3);
        f.fill_hex("#8a96c4");
        for s in &self.smoke {
            let fa = s.age / s.life;
            f.alpha = 0.35 * (1. - fa);
            let z = if fa > 0.5 { 2. } else { 1. };
            f.px(v, s.x, s.y, 0.6, z, z);
        }
        f.alpha = 1.;
    }
}
