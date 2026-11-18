//! TAKO — deep blue water at dusk; Tako rests on a rock and keeps you company.
//! Jellies drift, plankton glow, bubbles rise.
use super::octo::{Octo, Pose, draw_octo};
use super::px::*;
use super::Scene;
use std::rc::Rc;

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
    y: f64,
    ph: f64,
    v: f64,
    src: bool,
    big: bool,
}
struct Fish {
    ox: f64,
    oy: f64,
    ph: f64,
}
struct Kelp {
    x: f64,
    base: f64,
    h: f64,
    ph: f64,
}

#[derive(Default)]
pub struct Tako {
    water: Option<Rc<Img>>,
    rays: Option<Rc<Img>>,
    far_r: Option<Rc<Img>>,
    rock: Option<Rc<Img>>,
    floor_r: Option<Rc<Img>>,
    snow: Vec<Snow>,
    plank: Vec<Plank>,
    bubbles: Vec<Bubble>,
    jellies: Vec<(f64, f64, f64)>,
    school: Vec<Fish>,
    school_x: f64,
    school_y: f64,
    kelp: Vec<Kelp>,
    rock_top: f64,
    oct_x: f64,
    blink_at: f64,
    blink: f64,
    rip: f64,
    rip_at: f64,
}

pub fn new() -> Box<dyn Scene> {
    Box::new(Tako { blink_at: 3., rip_at: 6., ..Default::default() })
}

impl Scene for Tako {
    fn build(&mut self, b: &mut Builder, v: &Vp, _t: f64) {
        let (fw, fh, w, h) = (v.fw, v.fh, v.w, v.h);
        let mut r = Rng::new(808);
        let n = Noise::new(12);
        let n2 = Noise::new(33);
        let k = w / 400.;
        let ws = stops(&[(0., "#3d8fe0"), (0.14, "#2668bf"), (0.4, "#153e86"), (0.7, "#0b2459"), (1., "#061536")]);
        let gl = hex("#b8e4ff");
        self.water = Some(b.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let mut c = ramp(&ws, (yf - v.y(0.)) / (h * 0.95), x, y);
            let g = clamp(1. - ((xf - v.x(0.45)) * 0.5).hypot(yf - v.y(-0.06)) / (h * 0.9));
            if g * g * 1.1 > bay(x, y) {
                c = mix(c, gl, 0.14 + g * 0.18);
            }
            op(c)
        })));
        let (ca, sa) = (0.3f64.cos(), 0.3f64.sin());
        let beams: Vec<(f64, f64, f64)> =
            [(-70., 7., 0.6), (-40., 11., 0.9), (-14., 5., 1.), (12., 9., 0.8), (40., 6., 0.7), (66., 10., 0.5)].iter().map(|&(o, bw, s)| (v.x(0.45) * ca + o * k, bw * k, s)).collect();
        self.rays = Some(b.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let u = xf * ca - yf * sa * 0.6;
            let mut i = 0.;
            for &(c, bw, s) in &beams {
                i += s * (-((u - c) / bw).powi(2)).exp();
            }
            i *= clamp(1. - (yf - v.y(0.)) / (h * 0.8));
            let q = (clamp(i) * 4. + bay(x, y) - 0.25).floor() / 4.;
            if q > 0. { Some([175, 225, 255, jround(q * 110.) as u8]) } else { None }
        })));
        let ft: Vec<f64> = (0..fw).map(|x| v.y(0.66) + fbm(n, x as f64 / 30., 2.1, 3) * h * 0.2).collect();
        let (fa, fb, frim) = (hex("#12397a"), hex("#0f3270"), hex("#2a62a8"));
        self.far_r = Some(b.img(paint(fw, fh, |x, y| {
            let yf = y as f64;
            let t = ft[x as usize];
            if yf < t {
                None
            } else if yf < t + 1. {
                op(frim)
            } else if fbm(n2, x as f64 / 7., yf / 7., 2) > 0.55 {
                op(fa)
            } else {
                op(fb)
            }
        })));
        // Tako's rock: a rounded outcrop on the right
        let oct_x = v.x(0.87);
        self.oct_x = oct_x;
        let rock_base = v.y(0.68);
        let rk = |s: &str| hex(s);
        let (rtop, rlit, rmid, rsh, rdk, rmoss, rrim) = (rk("#4f78b8"), rk("#33589a"), rk("#22417c"), rk("#162d5c"), rk("#0d1e42"), rk("#2f7a8a"), rk("#7fd0ff"));
        let in_rock = |x: f64, y: f64| {
            let dx = (x - oct_x) / (w * 0.13);
            let dy = (y - (rock_base + h * 0.2)) / (h * 0.2);
            dy > -1.2 && dx * dx + dy * dy * 0.9 < 1. + (fbm(n, x / 6., y / 6., 3) - 0.5) * 0.55
        };
        let mut rock = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if !in_rock(xf, yf) {
                return None;
            }
            if !in_rock(xf, yf - 1.) {
                return op(if bay(x, y) > 0.4 { rtop } else { rrim });
            }
            let mut d = 0.;
            while d < 12. && in_rock(xf, yf - d - 1.) {
                d += 1.;
            }
            let vv = fbm(n2, xf / 4. + 30., yf / 4., 3);
            let q = d / 12. + (vv - 0.5) * 0.5 + if xf > oct_x + w * 0.06 { 0.15 } else { 0. };
            let mut c = if q < 0.15 {
                rlit
            } else if q < 0.38 {
                if bay(x, y) > 0.5 { rlit } else { rmid }
            } else if q < 0.7 {
                rmid
            } else if q < 0.9 {
                rsh
            } else {
                rdk
            };
            if d < 4. && fbm(n, xf / 3. + 70., yf / 3., 2) > 0.64 {
                c = rmoss;
            }
            op(c)
        });
        {
            let co = [hex("#ff8fb8"), hex("#ffb47a"), hex("#b98cff"), hex("#7fe0d0")];
            let mut i = 0.;
            while i < 26. * k {
                i += 1.;
                let x = jround(oct_x + (r.next() - 0.5) * w * 0.24);
                let mut y = jround(rock_base - h * 0.1);
                while y < fh as f64 && !in_rock(x, y) {
                    y += 1.;
                }
                if (x - oct_x).abs() < w * 0.035 {
                    continue;
                }
                let c = co[(r.next() * 4.) as usize];
                let hh = 2 + (r.next() * 4.) as i64;
                for j in 0..hh {
                    rock.px(c, x, y - j as f64);
                    if j > 1 && j % 2 == 0 {
                        rock.px(c, x + if j & 2 != 0 { 1. } else { -1. }, y - j as f64 - 1.);
                    }
                }
            }
        }
        // locate the top of the rock under Tako
        let mut rt = jround(rock_base - h * 0.1);
        while rt < fh as f64 && !in_rock(jround(oct_x), rt) {
            rt += 1.;
        }
        self.rock_top = rt;
        self.rock = Some(b.img(rock));
        let sd = stops(&[(0., "#1d3f78"), (0.5, "#142f5f"), (1., "#0b1e44")]);
        let st: Vec<f64> = (0..fw).map(|x| v.y(0.86) + (x as f64 * 0.05).sin() * 1.5 + fbm(n, x as f64 / 18., 6., 2) * 4.).collect();
        let glint = hex("#4f7fc0");
        self.floor_r = Some(b.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if yf < st[x as usize] {
                return None;
            }
            let mut c = ramp(&sd, (yf - v.y(0.86)) / (h * 0.16), x, y);
            if (xf * 0.2 + fbm(n2, xf / 10., yf / 3., 2) * 7. + yf * 0.5).sin() > 0.86 {
                c = mix(c, glint, 0.4);
            }
            op(c)
        })));
        let ns = jround(w * h / 420.) as usize;
        self.snow = (0..ns)
            .map(|_| Snow { x: r.next() * fw as f64, y: r.next() * fh as f64, v: 1.2 + r.next() * 3., d: 0.3 + r.next() * 0.5, a: 0.2 + r.next() * 0.45, ph: r.next() * 6. })
            .collect();
        self.plank = (0..30).map(|_| Plank { x: v.x(-0.02) + r.next() * w * 1.04, y: v.y(0.12) + r.next() * h * 0.7, ph: r.next() * 6., sp: 0.6 + r.next() }).collect();
        self.bubbles = (0..16).map(|_| Bubble { y: r.next() * h, ph: r.next() * 6., v: 6. + r.next() * 7., src: r.next() >= 0.5, big: r.next() < 0.3 }).collect();
        self.jellies = vec![(v.x(0.16), v.y(0.3), 0.), (v.x(0.62), v.y(0.2), 2.4)];
        self.school = (0..14).map(|_| Fish { ox: (r.next() - 0.5) * 34., oy: (r.next() - 0.5) * 14., ph: r.next() * 6. }).collect();
        self.school_x = w * 0.9;
        self.school_y = h * 0.28;
        let nk = jround(9. * k) as usize + 3;
        self.kelp = (0..nk)
            .map(|i| {
                let x = if i % 2 == 1 { v.x(0.01 + r.next() * 0.16) } else { v.x(0.6 + r.next() * 0.12) };
                Kelp { x, base: v.y(0.9) + r.next() * h * 0.06, h: h * (0.3 + r.next() * 0.35), ph: r.next() * 6. }
            })
            .collect();
    }

    fn update(&mut self, v: &Vp, t: f64, dt: f64) {
        let fh = v.fh as f64;
        for s in &mut self.snow {
            s.y += s.v * dt;
            s.x += (t * 0.4 + s.ph).sin() * dt;
            if s.y > fh {
                s.y = 0.;
                s.x = rand() * v.fw as f64;
            }
        }
        for b in &mut self.bubbles {
            b.y -= b.v * dt;
            if b.y < v.y(0.02) {
                b.y = if b.src { self.rock_top - 18. } else { v.y(0.88) };
            }
        }
        self.school_x -= dt * 7.;
        if self.school_x < -v.pad - 60. {
            self.school_x = v.w + 60. + rand() * 100.;
            self.school_y = v.h * (0.14 + rand() * 0.2);
        }
        if t > self.blink_at {
            self.blink = 0.15;
            self.blink_at = t + 2.5 + rand() * 3.5;
        }
        self.blink = (self.blink - dt).max(0.);
        if t > self.rip_at {
            self.rip = 1.6;
            self.rip_at = t + 7. + rand() * 6.;
        }
        self.rip = (self.rip - dt).max(0.);
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(water) = &self.water else { return };
        f.layer(v, water, 0.04, 1.);
        // "lighter" isn't available: source-over at a slightly boosted alpha instead
        f.layer(v, self.rays.as_ref().unwrap(), 0.18, ((0.6 + 0.2 * (t * 0.45).sin()) * 1.25).min(1.));
        let (sx0, sy0) = v.at(0., v.y(0.03), 0.08);
        let (hi, lo) = (hex("#d4f0ff"), hex("#3f8ad8"));
        // surface shimmer: draw the light row then the dark row so runs can merge
        for pass in 0..2 {
            f.fill(if pass == 0 { hi } else { lo });
            f.alpha = 0.45;
            for x in 0..(v.w as i64 + 2) {
                let xf = x as f64;
                let yy = sy0 + jround((xf * 0.11 + t * 1.3).sin() * 1.1 + (xf * 0.04 - t * 0.7).sin() * 1.3);
                f.rect(xf + sx0 % 1., yy + pass as f64, 1., 1.);
            }
        }
        f.alpha = 1.;
        f.layer(v, self.far_r.as_ref().unwrap(), 0.2, 1.);
        f.fill_hex("#16437e");
        for fi in &self.school {
            let (a, b) = v.at(
                self.school_x + v.pad + fi.ox + (t * 1.4 + fi.ph).sin() * 2.,
                self.school_y + v.pad_y + fi.oy + (t * 2. + fi.ph).sin() * 1.4,
                0.3,
            );
            f.rect(a, b, 2., 1.);
            f.rect(a + 2., b - 1., 1., 1.);
            f.rect(a + 2., b + 1., 1., 1.);
        }
        for &(jx, jy, ph) in &self.jellies {
            let (a, b) = v.at(jx, jy + (t * 0.35 + ph).sin() * 4., 0.35);
            let pu = (t * 2.1 + ph).sin();
            f.fill_hex("#ff9fd8");
            f.alpha = 0.18;
            let mut rr = 9.;
            while rr > 4. {
                f.rect(a - rr, b - rr + 3., rr * 2., rr * 2.);
                rr -= 2.;
            }
            f.alpha = 0.85;
            for dy in -4..=0 {
                let ww = jround(((16 - dy * dy) as f64).sqrt() * (1. + pu * 0.12));
                f.rect(a - ww, b + dy as f64, 2. * ww + 1., 1.);
            }
            f.alpha = 1.;
            f.fill_hex("#ffe0f4");
            f.rect(a - 2., b - 3., 2., 1.);
            f.fill_hex("#ff9fd8");
            f.alpha = 0.55;
            for q in -2..=2 {
                for s in 1..9 {
                    let (qf, sf) = (q as f64, s as f64);
                    f.rect(jround(a + qf * 1.4 + (t * 2. + sf * 0.5 + qf).sin() * sf / 6.), b + sf, 1., 1.);
                }
            }
            f.alpha = 1.;
        }
        let kc = [hex("#0f3a6e"), hex("#1a5a8a")];
        for kp in &self.kelp {
            let (bx, by) = v.at(kp.x, kp.base, 0.55);
            let xs: Vec<f64> = (0..kp.h.ceil() as i64)
                .map(|j| {
                    let u = j as f64 / kp.h;
                    jround(bx + (t * 0.7 + kp.ph - j as f64 * 0.08).sin() * u * u * 7.)
                })
                .collect();
            f.fill(kc[0]);
            for (j, x) in xs.iter().enumerate() {
                f.rect(*x, by - j as f64, 2., 1.);
            }
            f.fill(kc[1]);
            for (j, x) in xs.iter().enumerate() {
                if j % 5 == 2 {
                    f.rect(if j % 10 < 5 { x + 2. } else { x - 2. }, by - j as f64, 2., 1.);
                }
            }
        }
        for p in &self.plank {
            let a2 = 0.5 + 0.5 * (t * p.sp + p.ph).sin();
            let (x, y) = v.at(p.x, p.y + (t * 0.6 + p.ph).sin() * 3., 0.45);
            f.fill_hex("#9fe6ff");
            f.alpha = a2 * 0.22;
            f.rect(x - 1., y - 1., 3., 3.);
            f.alpha = a2;
            f.fill_hex("#d6f6ff");
            f.rect(x, y, 1., 1.);
        }
        f.fill_hex("#a9c8ff");
        for s in &self.snow {
            f.alpha = s.a;
            let (x, y) = v.at(s.x, s.y, s.d);
            f.rect(x, y, 1., 1.);
        }
        f.alpha = 1.;
        f.layer(v, self.floor_r.as_ref().unwrap(), 0.5, 1.);
        f.layer(v, self.rock.as_ref().unwrap(), 0.62, 1.);
        // Tako, resting
        let (ox, oy) = v.at(self.oct_x, self.rock_top - 15. + (t * 1.4).sin() * 0.8, 0.62);
        let (_, fy) = v.at(0., self.rock_top + 1., 0.62);
        f.fill_hex("#4f86f2");
        f.alpha = 0.1;
        let mut rr = 30.;
        while rr > 10. {
            f.rect(ox - rr, oy - rr * 0.7, rr * 2., rr * 1.4);
            rr -= 6.;
        }
        f.alpha = 1.;
        let (oxi, oyi) = (jround(ox) as i32, jround(oy) as i32);
        let mut ras = Raster::new(oxi - 64, oyi - 40, 128, 100);
        draw_octo(
            &mut ras,
            &Octo {
                x: ox,
                y: oy,
                t,
                pose: Pose::Sit,
                floor: Some(fy),
                look: [v.mx, v.my],
                lid: if self.blink > 0. { 1. } else { 0. },
                ripple: if self.rip > 0. { ((self.rip / 1.6).min(1.) * std::f64::consts::PI).sin() } else { 0. },
                ..Default::default()
            },
        );
        ras.emit(f, 0., 0.);
        for b in &self.bubbles {
            let src = if b.src { (self.oct_x + 1., self.rock_top - 18.) } else { (v.x(0.3), v.y(0.88)) };
            if b.y > src.1 {
                continue;
            }
            let (a, c) = v.at(src.0 + (t * 2.4 + b.ph + b.y * 0.08).sin() * 1.6, b.y, 0.8);
            f.alpha = clamp((src.1 - b.y) / 8.) * 0.85;
            f.fill_hex("#dff6ff");
            if b.big {
                f.rect(a, c - 1., 1., 1.);
                f.rect(a - 1., c, 1., 1.);
                f.rect(a + 1., c, 1., 1.);
                f.rect(a, c + 1., 1., 1.);
                f.fill_hex("#ffffff");
                f.rect(a - 1., c - 1., 1., 1.);
            } else {
                f.rect(a, c, 1., 1.);
            }
        }
        f.alpha = 1.;
    }
}
