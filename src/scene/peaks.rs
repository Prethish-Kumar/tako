//! PEAKS — sunset pinnacles, cliff-top towers, waterfalls, mist.
use super::Scene;
use super::px::*;
use std::rc::Rc;

struct Spire {
    cx: f64,
    w: f64,
    h: f64,
}
struct SpirePal {
    lit: C3,
    mid: C3,
    shade: C3,
    haze: C3,
    moss: C3,
}
struct Mist {
    img: Rc<Img>,
    y: f64,
    v: f64,
    d: f64,
    a: f64,
}
struct Win {
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
}
struct Fall {
    x: f64,
    w: i64,
    y0: i64,
    y1: i64,
}
struct Bird {
    x: f64,
    y: f64,
    v: f64,
    ph: f64,
}

#[derive(Default)]
pub struct Peaks {
    sky: Option<Rc<Img>>,
    far: Option<Rc<Img>>,
    mid: Option<Rc<Img>>,
    cliff: Option<Rc<Img>>,
    fg: Option<Rc<Img>>,
    mists: Vec<Mist>,
    wins: Vec<Win>,
    falls: Vec<Fall>,
    birds: Vec<Bird>,
}

pub fn new() -> Box<dyn Scene> {
    Box::<Peaks>::default()
}

/// The mock's `spires(seed, list, P, base, hazeH, mossy, floor)`.
fn spires(v: &Vp, sun_x: f64, seed: u32, list: &[Spire], k: &SpirePal, base: f64, haze_h: f64, mossy: bool, floor: &Stops) -> Layer {
    let (fw, fh, h) = (v.fw, v.fh, v.h);
    let nn = Noise::new(seed);
    struct Sp {
        cx: f64,
        w: f64,
        h: f64,
        top: f64,
        rows: Vec<f32>,
        lean: f64,
    }
    let sp: Vec<Sp> = list
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let mut rr = Rng::new(seed.wrapping_mul(31).wrapping_add(i as u32));
            let top = jround(base - o.h);
            let mut rows = vec![0f32; fh + 1];
            let mut j = 0f64;
            let mut y = top.max(0.) as i64;
            while y <= fh as i64 {
                if (y - top as i64) % 3 == 0 {
                    j = (rr.next() - 0.5) * o.w * 0.25;
                }
                rows[y as usize] = j as f32;
                y += 1;
            }
            Sp { cx: o.cx, w: o.w, h: o.h, top, rows, lean: (rr.next() - 0.5) * o.w * 0.5 }
        })
        .collect();
    paint(fw, fh, |x, y| {
        let (xf, yf) = (x as f64, y as f64);
        let mut hit: Option<(&Sp, f64, f64)> = None;
        for q in sp.iter().rev() {
            if yf < q.top || yf > base + 3. {
                continue;
            }
            let uu = (yf - q.top) / q.h;
            let prof = 0.3 + uu.min(1.).powf(0.45) * 0.7 + (uu - 1.).max(0.) * 0.15;
            let cx = q.cx + q.lean * (1. - uu).max(0.);
            let hw = q.w / 2. * prof + q.rows[y as usize] as f64;
            if (xf - cx).abs() <= hw {
                hit = Some((q, uu, (xf - cx) / hw.max(1.)));
                break;
            }
        }
        let b = bay(x, y);
        let Some((o, u, s)) = hit else {
            return if yf > base - h * 0.03 { op(ramp(floor, (yf - base + h * 0.03) / (h * 0.3), x, y)) } else { None };
        };
        let side = if o.cx < sun_x { s } else { -s };
        let mut c = if side > 0.42 {
            k.lit
        } else if side > -0.25 {
            k.mid
        } else {
            k.shade
        };
        if (side - 0.42).abs() < 0.12 && b > 0.5 {
            c = if side > 0.42 { k.mid } else { k.lit };
        }
        if (side + 0.25).abs() < 0.12 && b > 0.5 {
            c = if side > -0.25 { k.shade } else { k.mid };
        }
        let st = nn.at(xf / 1.7, yf / 8.);
        if st > 0.72 {
            c = mix(c, k.shade, 0.55);
        } else if st < 0.2 && side > -0.1 {
            c = mix(c, k.lit, 0.45);
        }
        if mossy && u > 0.1 && nn.at(xf / 3. + 50., yf / 2.) > 0.74 {
            c = if b > 0.4 { k.moss } else { mix(k.moss, c, 0.5) };
        }
        if u < 0.035 && side > -0.4 {
            c = k.lit;
        }
        let hk = (yf - (base - haze_h)) / haze_h;
        if hk > b {
            c = mix(c, k.haze, clampr(0.3 + hk * 0.45, 0., 0.85));
        }
        op(c)
    })
}

fn mist(v: &Vp, seed: u32, h: usize, col: &str, thr: f64) -> Layer {
    let nm = Noise::new(seed);
    let fw = v.fw as f64;
    let p = jround(fw / 40.).max(2.);
    let kc = hex(col);
    paint(v.fw, h, |x, y| {
        let (xf, yf) = (x as f64, y as f64);
        if fbmp(nm, xf / fw * p, yf / 6., 4, p) * (std::f64::consts::PI * yf / h as f64).sin() > thr + bay(x, y) * 0.3 { op(kc) } else { None }
    })
}

impl Scene for Peaks {
    fn build(&mut self, bld: &mut Builder, v: &Vp, _t: f64) {
        let (fw, fh, w, h) = (v.fw, v.fh, v.w, v.h);
        let mut r = Rng::new(77);
        let n = Noise::new(5);
        let n2 = Noise::new(9);
        let k = w / 400.;
        let (sun_x, sun_y, hz) = (v.x(0.44), v.y(0.52), v.y(0.66));
        let sky = stops(&[(0., "#1f1840"), (0.2, "#38215f"), (0.4, "#6e3483"), (0.56, "#b94d8a"), (0.7, "#ea7a5c"), (0.84, "#ffb36e"), (1., "#ffe7a8")]);
        let (glow, hi, pink, violet) = (hex("#ffcf8a"), hex("#ffd9a0"), hex("#ef8cae"), hex("#a462ae"));
        self.sky = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let p = yf / hz;
            let b = bay(x, y);
            let mut col = ramp(&sky, p, x, y);
            let gl = clamp(1. - ((xf - sun_x) * 0.7).hypot(yf - sun_y) / (h * 0.7));
            if gl * gl * 1.3 > b {
                col = mix(col, glow, 0.25 + gl * 0.3);
            }
            if p < 0.92 {
                let c = fbm(n, xf / 64., yf / 6., 4) + (fbm(n2, xf / 18., yf / 2.5, 2) - 0.5) * 0.22;
                let kk = sstep(0.5, 0.66, c) * sstep(0.04, 0.22, p);
                if kk > b {
                    col = mix(col, if p > 0.6 { hi } else if p > 0.34 { pink } else { violet }, if kk > 0.75 { 0.7 } else { 0.45 });
                }
            }
            op(col)
        })));

        let pal = |lit: &str, mid: &str, shade: &str, haze: &str, moss: &str| SpirePal { lit: hex(lit), mid: hex(mid), shade: hex(shade), haze: hex(haze), moss: hex(moss) };
        let mut far: Vec<Spire> = (0..22)
            .map(|i| {
                let cx = v.x(-0.05 + (i as f64 + r.next() * 0.8) / 22. * 1.1);
                let ww = (14. + r.next() * 18.) * k;
                let hh = h * (0.14 + r.next() * 0.26);
                Spire { cx, w: ww, h: hh }
            })
            .collect();
        far.push(Spire { cx: v.x(0.5), w: 26. * k, h: h * 0.44 });
        far.push(Spire { cx: v.x(0.39), w: 20. * k, h: h * 0.36 });
        self.far = Some(bld.img(spires(
            v,
            sun_x,
            11,
            &far,
            &pal("#ffc9a4", "#b98ac6", "#8b6db6", "#9396d8", "#7aa8a0"),
            v.y(0.72),
            h * 0.3,
            false,
            &stops(&[(0., "#9396d8"), (1., "#7a7cc4")]),
        )));
        let mid_l: Vec<Spire> = [(0.03, 40., 0.7), (0.18, 30., 0.58), (0.32, 38., 0.78), (0.45, 22., 0.46), (0.58, 34., 0.7), (0.68, 26., 0.52), (0.84, 44., 0.84), (0.98, 30., 0.62)]
            .iter()
            .map(|&(f, ww, hh)| Spire { cx: v.x(f), w: ww * k, h: h * hh })
            .collect();
        self.mid = Some(bld.img(spires(
            v,
            sun_x,
            23,
            &mid_l,
            &pal("#ffa36a", "#b2488c", "#5c2b72", "#5b66b8", "#4f9a6a"),
            v.y(0.86),
            h * 0.32,
            true,
            &stops(&[(0., "#5b66b8"), (0.5, "#3e4590"), (1., "#2a2a5e")]),
        )));

        let m0 = mist(v, 3, jround(h * 0.2) as usize, "#c6b2e8", 0.2);
        let m1 = mist(v, 4, jround(h * 0.18) as usize, "#6ccfd0", 0.22);
        self.mists = vec![
            Mist { img: bld.img(m0), y: v.y(0.58), v: 2.2, d: 0.4, a: 0.5 },
            Mist { img: bld.img(m1), y: v.y(0.8), v: -1.6, d: 0.75, a: 0.3 },
        ];

        // Near cliffs framing the canyon
        let nc = Noise::new(31);
        let (ca, cb, cc, clit, cmoss_d, cmoss, cmoss_l, cteal) =
            (hex("#17121f"), hex("#261c38"), hex("#3c2a4c"), hex("#b65c78"), hex("#24503a"), hex("#3f8048"), hex("#88c060"), hex("#3a8a90"));
        let lt: Vec<f32> = (0..fw)
            .map(|x| {
                let xf = x as f64;
                ((if xf < v.x(0.07) { v.y(0.34) } else { v.y(0.43) }) + (fbm(nc, xf / 16., 0.5, 4) - 0.5) * h * 0.08) as f32
            })
            .collect();
        let rt: Vec<f32> = (0..fw)
            .map(|x| {
                let xf = x as f64;
                let base = if xf < v.x(0.79) {
                    v.y(0.53)
                } else if xf < v.x(0.885) {
                    v.y(0.44)
                } else {
                    v.y(0.32)
                };
                (base + (fbm(nc, xf / 16. + 40., 0.5, 4) - 0.5) * h * 0.07) as f32
            })
            .collect();
        let le: Vec<f32> = (0..fh).map(|y| (v.x(0.25) + (fbm(nc, 3.3, y as f64 / 12., 4) - 0.5) * w * 0.08 + (y as f64 - v.y(0.5)) * 0.1) as f32).collect();
        let re: Vec<f32> = (0..fh).map(|y| (v.x(0.71) + (fbm(nc, 7.7, y as f64 / 12., 4) - 0.5) * w * 0.08 - (y as f64 - v.y(0.5)) * 0.14) as f32).collect();
        let y72 = v.y(0.72);
        let mut cliff = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let (ltx, rtx, ley, rey) = (lt[x as usize] as f64, rt[x as usize] as f64, le[y as usize] as f64, re[y as usize] as f64);
            let in_l = xf < ley && yf > ltx;
            let in_r = xf > rey && yf > rtx;
            if !in_l && !in_r {
                return None;
            }
            let b = bay(x, y);
            let kk = yf - if in_l { ltx } else { rtx };
            let e = if in_l { ley - xf } else { xf - rey };
            let f = fbm(nc, xf / 7., yf / 10., 3);
            let mut c = if f > 0.6 {
                cc
            } else if f > 0.44 {
                cb
            } else {
                ca
            };
            if (f - 0.6).abs() < 0.025 && b > 0.5 {
                c = cb;
            }
            let m = fbm(nc, xf / 4. + 90., yf / 2.6, 2);
            if kk < 2. + f * 5. {
                c = if kk < 1. {
                    cmoss_l
                } else if b > 0.45 {
                    cmoss
                } else {
                    cmoss_d
                };
            } else if m > 0.68 && f > 0.4 {
                c = if m > 0.76 && b > 0.3 { cmoss } else { cmoss_d };
            }
            if e < 2. {
                c = if kk < 3. { cmoss_l } else { clit };
            } else if e < 5. && b > 0.6 - (5. - e) * 0.1 {
                c = mix(c, clit, 0.5);
            }
            if yf > y72 && fbm(nc, xf / 5. + 7., yf / 4., 2) > 0.62 && b > 0.45 {
                c = cteal;
            }
            op(c)
        });

        // Towers — each glowing window is an "agent"
        let (tl, tm, td, troof, trl, tslit) = (hex("#e08448"), hex("#ad4f4c"), hex("#5a2448"), hex("#2a1428"), hex("#e88a52"), hex("#2a0f1e"));
        let mut wins = vec![];
        let mut tower = |cl: &mut Layer, r: &mut Rng, fx: f64, w0: f64, h0: f64, dir: i32, tops: &[f32]| {
            let cxr = jround(v.x(fx));
            let w = jround(w0 * k).max(6.);
            let hh = jround(h0 * k);
            let base = jround(tops[cxr as usize] as f64) + 3.;
            let x0 = cxr - ((w as i64) >> 1) as f64;
            let y0 = base - hh;
            for y in 0..(hh as i64 + 6) {
                for x in 0..w as i64 {
                    let (xf, yf) = (x as f64, y as f64);
                    let s = if dir > 0 { xf / (w - 1.) } else { 1. - xf / (w - 1.) };
                    let b = bay(x0 as i64 + x, y0 as i64 + y);
                    let mut c = if s > 0.62 {
                        tl
                    } else if s > 0.4 {
                        if b > 0.5 { tl } else { tm }
                    } else if s > 0.18 {
                        tm
                    } else if b > 0.6 {
                        tm
                    } else {
                        td
                    };
                    if y < 2 {
                        c = td;
                    } else if yf > hh - 3. {
                        c = mix(c, td, 0.5);
                    }
                    cl.px(c, x0 + xf, y0 + yf);
                }
            }
            let roof = |cl: &mut Layer, yy: f64, ww: f64| {
                let xl = jround(x0 + w / 2. - ww / 2.);
                cl.rect(trl, xl + 2., yy, ww - 4., 1.);
                cl.rect(troof, xl + 1., yy + 1., ww - 2., 1.);
                cl.rect(troof, xl, yy + 2., ww, 1.);
                cl.px(troof, xl - 1., yy + 1.);
                cl.px(troof, xl + ww, yy + 1.);
                cl.px(trl, if dir > 0 { xl + ww } else { xl - 1. }, yy + 1.);
            };
            roof(cl, y0 - 3., w + 6.);
            let cw = jround(w * 0.5).max(4.);
            let cx0 = jround(x0 + w / 2. - cw / 2.);
            let ch = jround(w * 0.45).max(4.);
            cl.rect(tm, cx0, y0 - 3. - ch, cw, ch);
            cl.rect(tl, if dir > 0 { cx0 + cw - 2. } else { cx0 }, y0 - 3. - ch, 2., ch);
            cl.rect(td, cx0, y0 - 3. - ch, cw, 1.);
            roof(cl, y0 - 6. - ch, cw + 6.);
            cl.rect(troof, jround(x0 + w / 2.), y0 - 9. - ch, 1., 3.);
            if hh > w * 2.2 {
                roof(cl, y0 + jround(hh * 0.45), w + 4.);
            }
            let mut yy = y0 + 6.;
            while yy < base - 6. {
                cl.rect(tslit, jround(x0 + w * if dir > 0 { 0.28 } else { 0.62 }), yy, 1., 2.);
                yy += 7.;
            }
            let wx = jround(x0 + w * if dir > 0 { 0.62 } else { 0.38 }) - 1.;
            wins.push(Win { x: wx, y: y0 + 5., ph: r.next() * 6., sp: 0.5 + r.next() });
            if hh > 28. {
                wins.push(Win { x: wx, y: y0 + jround(hh * 0.62), ph: r.next() * 6., sp: 0.5 + r.next() });
            }
            wins.push(Win { x: jround(x0 + w / 2.) - 1., y: y0 - 2. - ch + 1., ph: r.next() * 6., sp: 0.5 + r.next() });
        };
        tower(&mut cliff, &mut r, 0.13, 15., 44., 1, &lt);
        tower(&mut cliff, &mut r, 0.205, 10., 20., 1, &lt);
        tower(&mut cliff, &mut r, 0.75, 12., 22., -1, &rt);
        tower(&mut cliff, &mut r, 0.835, 14., 30., -1, &rt);
        tower(&mut cliff, &mut r, 0.94, 17., 40., -1, &rt);
        self.wins = wins;
        self.cliff = Some(bld.img(cliff));

        self.falls = [(0.045, 2, &lt), (0.215, 1, &lt), (0.775, 2, &rt), (0.9, 3, &rt)]
            .iter()
            .map(|&(f, ww, tops)| {
                let x = jround(v.x(f));
                let y0 = jround(tops[x as usize] as f64 + 7.);
                Fall { x, w: ww, y0: y0 as i64, y1: jround(y0 + h * (0.22 + r.next() * 0.14)) as i64 }
            })
            .collect();

        // Dark foreground foliage in the bottom corners
        let nf = Noise::new(61);
        let blobs: Vec<(f64, f64, f64, f64)> = (0..14)
            .map(|i| {
                let left = i < 7;
                let u = r.next();
                let bx = if left { v.x(-0.04 + u * 0.22) } else { v.x(0.8 + u * 0.24) };
                let by = v.y(0.92 + r.next() * 0.1);
                let rx = w * (0.04 + r.next() * 0.05);
                let ry = h * (0.05 + r.next() * 0.06);
                (bx, by, rx, ry)
            })
            .collect();
        let in_b = |x: f64, y: f64| {
            let th = 1. + (fbm(nf, x / 3., y / 3., 2) - 0.5) * 0.9;
            blobs.iter().any(|&(bx, by, rx, ry)| ((x - bx) / rx).powi(2) + ((y - by) / ry).powi(2) < th)
        };
        let y78 = v.y(0.78);
        self.fg = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if yf < y78 || !in_b(xf, yf) {
                None
            } else if !in_b(xf, yf - 1.) {
                Some([70, 150, 132, 255])
            } else if !in_b(xf, yf - 2.) && bay(x, y) > 0.5 {
                Some([34, 84, 88, 255])
            } else {
                Some([18, 13, 30, 255])
            }
        })));

        self.birds = (0..7)
            .map(|_| {
                let x = v.x(0.25) + r.next() * w * 0.5;
                let y = v.y(0.55) + r.next() * h * 0.16;
                Bird { x, y, v: 2.5 + r.next() * 2.5, ph: r.next() * 6. }
            })
            .collect();
    }

    fn update(&mut self, v: &Vp, _t: f64, dt: f64) {
        for b in &mut self.birds {
            b.x += b.v * dt;
            if b.x > v.x(1.05) {
                b.x = v.x(-0.05);
                b.y = v.y(0.52) + rand() * v.h * 0.2;
            }
        }
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(sky) = &self.sky else { return };
        f.fill_hex("#1f1840");
        f.rect(0., 0., v.w, v.h);
        f.layer(v, sky, 0.04, 1.);
        f.layer(v, self.far.as_ref().unwrap(), 0.12, 1.);
        f.layer(v, self.mid.as_ref().unwrap(), 0.26, 1.);
        let fw = v.fw as f64;
        let mist_at = |f: &mut Frame, m: &Mist| {
            let o = ((t * m.v) % fw + fw) % fw;
            let (x, y) = v.at(0., m.y, m.d);
            f.set_pass(m.a);
            f.image(&m.img, v.snap(x + o), y, 1.);
            f.image(&m.img, v.snap(x + o - fw), y, 1.);
            f.set_pass(1.);
        };
        mist_at(f, &self.mists[0]);
        f.fill_hex("#f6eefc");
        for b in &self.birds {
            let (x, y) = v.at(b.x, b.y + (t * 0.8 + b.ph).sin() * 2., 0.5);
            let glide = (t * 0.6 + b.ph).sin() < 0.2;
            let up = !glide && ((t * 7. + b.ph).floor() as i64) % 2 != 0;
            f.rect(x - 1., y, 3., 1.);
            if glide {
                f.rect(x - 2., y, 1., 1.);
                f.rect(x + 2., y, 1., 1.);
            } else {
                let dy = if up { -1. } else { 1. };
                f.rect(x - 2., y + dy, 1., 1.);
                f.rect(x + 2., y + dy, 1., 1.);
            }
        }
        f.layer(v, self.cliff.as_ref().unwrap(), 0.62, 1.);
        let (ox, oy) = v.at(0., 0., 0.62);
        let fc = ["#e4f6ff", "#86cbe6", "#4a86c0"];
        let tf = (t * 20.).floor() as i64;
        for (ci, col) in fc.iter().enumerate() {
            f.fill_hex(col);
            for fl in &self.falls {
                for y in fl.y0..fl.y1 {
                    let yf = y as f64;
                    let fade = (yf - fl.y0 as f64) / (fl.y1 - fl.y0) as f64;
                    for i in 0..fl.w {
                        let ph = (((yf + i as f64 * 3. - t * 24. + fl.x * 7.) % 7.) + 7.) % 7.;
                        if fade > 0.6 && bay(fl.x as i64 + i, y + tf) < (fade - 0.6) * 2.5 {
                            continue;
                        }
                        let band = if ph < 2. {
                            0
                        } else if ph < 5. {
                            1
                        } else {
                            2
                        };
                        if band == ci {
                            f.rect(fl.x + i as f64 + ox, yf + oy, 1., 1.);
                        }
                    }
                }
            }
        }
        f.fill_hex("#dff4ff");
        for fl in &self.falls {
            for i in 0..6 {
                let a = t * 1.7 + i as f64 * 1.3 + fl.x;
                let rr = a % 1.;
                f.alpha = 0.5 * (1. - rr);
                f.rect(fl.x + ox + jround((a * 3.1).sin() * (2. + rr * 4.)), fl.y1 as f64 + oy - jround(rr * 4.), 1., 1.);
            }
        }
        f.alpha = 1.;
        for w in &self.wins {
            let fl = 0.8 + 0.2 * (t * w.sp + w.ph).sin() * (t * w.sp * 2.3 + 1.).sin();
            f.alpha = 0.3 * fl;
            f.fill_hex("#ffb256");
            f.rect(w.x + ox - 1., w.y + oy - 1., 4., 4.);
            f.alpha = fl;
            f.fill_hex("#ffe49c");
            f.rect(w.x + ox, w.y + oy, 2., 2.);
        }
        f.alpha = 1.;
        mist_at(f, &self.mists[1]);
        f.layer(v, self.fg.as_ref().unwrap(), 1., 1.);
    }
}
