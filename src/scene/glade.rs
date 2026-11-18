//! GLADE — forest clearing, sun shafts, pond, butterflies.
use super::Scene;
use super::px::*;
use std::rc::Rc;

const FLC: [&str; 6] = ["#e6503c", "#f4f2ea", "#f2d24a", "#7aa8ec", "#f294b8", "#ff8a3a"];

struct Blade {
    x: f64,
    h: f64,
    c: usize,
    ph: f64,
    fl: Option<C3>,
}
struct Fly {
    c: C3,
    cx: f64,
    cy: f64,
    ph: f64,
}
struct Mote {
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
}
struct Leaf {
    x: f64,
    y: f64,
    v: f64,
    ph: f64,
    c: C3,
}

#[derive(Default)]
pub struct Glade {
    sky: Option<Rc<Img>>,
    far: Option<Rc<Img>>,
    mid: Option<Rc<Img>>,
    ground: Option<Rc<Img>>,
    pond: Option<Rc<Img>>,
    rays: Option<Rc<Img>>,
    trees: Option<Rc<Img>>,
    pond_pts: Vec<(f64, f64)>,
    blades: Vec<Blade>,
    flies: Vec<Fly>,
    motes: Vec<Mote>,
    leaves: Vec<Leaf>,
}

pub fn new() -> Box<dyn Scene> {
    Box::<Glade>::default()
}

impl Scene for Glade {
    fn build(&mut self, bld: &mut Builder, v: &Vp, _t: f64) {
        let (fw, fh, w, h, pad) = (v.fw, v.fh, v.w, v.h, v.pad);
        let mut r = Rng::new(19);
        let n = Noise::new(8);
        let k = w / 400.;
        let (sun_x, sun_y) = (v.x(0.56), v.y(0.04));
        let sk = stops(&[(0., "#86c4dc"), (0.5, "#b4e0ea"), (1., "#e4f5ea")]);
        let gl = hex("#fffbe2");
        let y55 = v.y(0.55);
        self.sky = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let mut c = ramp(&sk, yf / y55, x, y);
            let g = clamp(1. - (xf - sun_x).hypot((yf - sun_y) * 1.3) / (w * 0.22));
            if g * g * 1.3 > bay(x, y) {
                c = mix(c, gl, 0.4 + g * 0.4);
            }
            op(c)
        })));

        let (fa, fb, fc, fd, fhaze) = (hex("#78ae86"), hex("#92c29a"), hex("#b4d8b0"), hex("#5f9474"), hex("#d8ecd2"));
        let ft: Vec<f32> = (0..fw).map(|x| (v.y(0.28) + fbm(n, x as f64 / 12., 3.7, 3) * h * 0.2) as f32).collect();
        let (y68, y50, y64) = (v.y(0.68), v.y(0.5), v.y(0.64));
        self.far = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let t0 = ft[x as usize] as f64;
            if yf < t0 || yf > y68 {
                return None;
            }
            let b = bay(x, y);
            let vv = fbm(n, xf / 4. + 9., yf / 4., 2);
            let mut c = if yf < t0 + 1. || vv > 0.62 {
                fc
            } else if vv > 0.48 {
                fb
            } else if vv > 0.35 {
                fa
            } else {
                fd
            };
            let hz = clamp(1. - (xf - sun_x).abs() / (w * 0.24)) * 0.5 + sstep(y50, y64, yf) * 0.5;
            if hz > b {
                c = mix(c, fhaze, 0.35);
            }
            op(c)
        })));

        let crowns: Vec<(f64, f64, f64)> = [(0.12, 0.33, 0.1), (0.26, 0.29, 0.09), (0.37, 0.38, 0.07), (0.69, 0.36, 0.075), (0.79, 0.29, 0.1), (0.91, 0.35, 0.09), (0.47, 0.45, 0.045), (0.63, 0.44, 0.04)]
            .iter()
            .map(|&(a, b, c)| (v.x(a), v.y(b), w * c))
            .collect();
        let (mhl, ml, mm, ms, md, mtrunk, mhaze) = (hex("#a8d86c"), hex("#6cb05a"), hex("#4a8e4c"), hex("#336e40"), hex("#24503a"), hex("#4a5a3a"), hex("#cfe6c8"));
        let (y48, y62) = (v.y(0.48), v.y(0.62));
        self.mid = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let b = bay(x, y);
            for (i, &(cx, cy, cr)) in crowns.iter().enumerate().rev() {
                let (dx, dy) = (xf - cx, yf - cy);
                let d = dx.hypot(dy * 1.1);
                // the noisy radius never exceeds 1.275·r
                if d >= cr * 1.275 {
                    continue;
                }
                if d < cr * (1. + (fbm(n, xf / 5. + i as f64 * 10., yf / 5., 2) - 0.5) * 0.55) {
                    let lit = (dx * 0.6 - dy * 0.8) / cr + (fbm(n, xf / 3.5 + 20., yf / 3.5, 2) - 0.5) * 0.9;
                    let mut col = if lit > 0.55 {
                        mhl
                    } else if lit > 0.2 {
                        ml
                    } else if lit > -0.2 {
                        mm
                    } else if lit > -0.55 {
                        ms
                    } else {
                        md
                    };
                    if yf > y48 && sstep(y48, y62, yf) > b {
                        col = mix(col, mhaze, 0.35);
                    }
                    return op(col);
                }
            }
            for &(cx, cy, cr) in &crowns {
                if (xf - cx).abs() < 1.5 + cr * 0.05 && yf > cy && yf < y64 {
                    return op(if sstep(y50, y62, yf) > b { mix(mtrunk, mhaze, 0.4) } else { mtrunk });
                }
            }
            None
        })));

        // meadow
        let gr = stops(&[(0., "#b6dc78"), (0.3, "#8cc858"), (0.65, "#5ea444"), (1., "#3e8236")]);
        let (glight, gshade, gdark) = (hex("#d2ec88"), hex("#2e6a34"), hex("#2a5a2c"));
        let y60 = v.y(0.6);
        let gt: Vec<f32> = (0..fw).map(|x| (y60 + (x as f64 * 0.05).sin() * 1.5 + fbm(n, x as f64 / 20., 9., 2) * 4.) as f32).collect();
        let mut ground = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if yf < gt[x as usize] as f64 {
                return None;
            }
            let b = bay(x, y);
            let p = (yf - y60) / (h * 0.4);
            let mut c = ramp(&gr, p, x, y);
            let lp = fbm(n, xf / 14., yf / 4., 3);
            if lp > 0.56 && (lp - 0.56) * 6. > b {
                c = mix(c, glight, 0.5);
            }
            if lp < 0.4 && (0.4 - lp) * 5. > b {
                c = mix(c, gshade, 0.3);
            }
            let side = (xf - pad).min(pad + w - xf) / (w * 0.2);
            if side < 1. && (1. - side) * 0.9 > b {
                c = mix(c, gshade, 0.35);
            }
            if fbm(n, xf / 1.2, yf / 3.5, 1) > 0.72 {
                c = mix(c, gdark, 0.3);
            }
            op(c)
        });
        let flc: Vec<C3> = FLC.iter().map(|c| hex(c)).collect();
        let (stem, yellow, orange, pale) = (hex("#2f6a2c"), hex("#f2d24a"), hex("#ff8a3a"), hex("#ffe880"));
        for _ in 0..64 {
            let cy = v.y(0.62) + r.next().powf(0.8) * h * 0.4;
            let cx = v.x(-0.02) + r.next() * w * 1.04;
            let col = flc[(r.next() * 6.) as usize];
            let near = (cy - y60) / (h * 0.4);
            let mut j = 0.;
            while j < 8. + near * 14. {
                j += 1.;
                let x = jround(cx + (r.next() - 0.5) * w * 0.07);
                let y = jround(cy + (r.next() - 0.5) * h * 0.03);
                let c = if r.next() < 0.75 { col } else { flc[(r.next() * 6.) as usize] };
                if near > 0.35 {
                    ground.rect(stem, x, y + 1., 1., if near > 0.7 { 2. } else { 1. });
                }
                ground.px(c, x, y);
                if near > 0.25 && r.next() < 0.7 {
                    ground.px(c, x - 1., y);
                    ground.px(c, x + 1., y);
                    ground.px(c, x, y - 1.);
                    if near > 0.5 {
                        ground.px(c, x, y + 1.);
                    }
                    ground.px(if c == yellow { orange } else { pale }, x, y);
                }
            }
        }
        self.ground = Some(bld.img(ground));

        // pond
        let (pcx, pcy, prx, pry) = (v.x(0.4), v.y(0.8), w * 0.26, h * 0.05);
        let (pedge, pdeep, pmid, psky, phi, prim, pstone) = (hex("#2c5446"), hex("#3f7a78"), hex("#79bcd0"), hex("#b6e2ee"), hex("#eaf9fc"), hex("#6e7a48"), hex("#a2a292"));
        let pin = |x: f64, y: f64| ((x - pcx) / prx).powi(2) + ((y - pcy) / pry).powi(2) < 1. + (fbm(n, x / 10. + 30., y / 4., 2) - 0.5) * 0.9;
        let pst: Stops = vec![(0., pdeep), (0.35, pmid), (0.75, psky), (1., psky)];
        let mut pts = vec![];
        let mut pond = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if (yf - pcy).abs() > pry * 1.8 || (xf - pcx).abs() > prx * 1.6 {
                return None;
            }
            if !pin(xf, yf) {
                return if pin(xf, yf - 1.) || pin(xf, yf + 1.) || pin(xf - 1., yf) || pin(xf + 1., yf) { op(if bay(x, y) > 0.6 { pstone } else { prim }) } else { None };
            }
            if !pin(xf, yf - 1.) {
                return op(pedge);
            }
            let q = (yf - (pcy - pry)) / (pry * 2.);
            let rip = fbm(n, xf / 18., yf / 1.2, 2);
            let mut c = ramp(&pst, q, x, y);
            if rip > 0.64 {
                c = phi;
            } else if rip < 0.3 && q > 0.3 {
                c = pmid;
            }
            if (x + y) % 2 == 0 {
                pts.push((xf, yf));
            }
            op(c)
        });
        self.pond_pts = pts;
        for (i, &(a, b)) in [(0.3, 0.79), (0.36, 0.815), (0.5, 0.8), (0.45, 0.83)].iter().enumerate() {
            let (x, y) = (v.x(a), v.y(b));
            pond.rect(hex("#4f9a3c"), x - 2., y, 5., 1.);
            pond.rect(hex("#6cb04a"), x - 1., y - 1., 3., 1.);
            if i % 2 == 0 {
                pond.rect(hex("#f8b0cc"), x, y - 2., 1., 1.);
                pond.rect(hex("#ffe2ec"), x, y - 1., 1., 1.);
            }
        }
        self.pond = Some(bld.img(pond));

        // sun shafts
        let (ca, sa) = (0.42f64.cos(), 0.42f64.sin());
        let beams: Vec<(f64, f64, f64)> =
            [(-46., 7., 0.7), (-26., 4., 0.9), (-10., 9., 0.8), (8., 3., 1.), (22., 7., 0.7), (40., 5., 0.6)].iter().map(|&(o, bw, s)| (sun_x * ca + o * k, bw * k, s)).collect();
        let (y86, y08) = (v.y(0.86), v.y(0.08));
        self.rays = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let u = xf * ca + yf * sa;
            let mut i = 0.;
            for &(c, bw, s) in &beams {
                i += s * (-((u - c) / bw).powi(2)).exp();
            }
            i *= clamp(1. - yf / y86) * clamp(yf / y08);
            let q = (clamp(i) * 4. + bay(x, y) - 0.25).floor() / 4.;
            if q > 0. { Some([255, 248, 214, jround(q * 170.) as u8]) } else { None }
        })));

        // framing trees + canopy
        let (bl, bm, bd, bg, bmoss, l1, l2, l3, l4, bsky) =
            (hex("#b07a44"), hex("#7a4e2e"), hex("#4a2e1c"), hex("#2c1a10"), hex("#5c8a3a"), hex("#a2d45c"), hex("#5ea242"), hex("#2f6e36"), hex("#1b4428"), hex("#d4eef0"));
        let trunks = [(v.x(0.075), w * 0.075, 1.), (v.x(0.93), w * 0.06, -1.)];
        let cb: Vec<f32> = (0..fw)
            .map(|x| {
                let xf = x as f64;
                let dl = sstep(v.x(0.5), v.x(0.28), xf);
                let dr = sstep(v.x(0.6), v.x(0.8), xf);
                let f = fbm(n, xf / 10., 5., 3);
                let a = v.y(-0.06) + dl * (h * 0.2 + f * h * 0.12 + h * 0.1 * (-((xf - v.x(0.08)) / (w * 0.1)).powi(2)).exp());
                let b = v.y(-0.06) + dr * (h * 0.18 + f * h * 0.12);
                a.max(b) as f32
            })
            .collect();
        let branches: Vec<[f64; 4]> = [(0.1, 0.24, 0.33, 0.1), (0.07, 0.32, -0.03, 0.16), (0.92, 0.25, 0.72, 0.12), (0.94, 0.4, 1.03, 0.28)]
            .iter()
            .map(|&(a, b, c, d)| [v.x(a), v.y(b), v.x(c), v.y(d)])
            .collect();
        let seg = |x: f64, y: f64, s: &[f64; 4]| {
            let [x1, y1, x2, y2] = *s;
            let (dx, dy) = (x2 - x1, y2 - y1);
            let u = clamp(((x - x1) * dx + (y - y1) * dy) / (dx * dx + dy * dy));
            (x - x1 - u * dx).hypot(y - y1 - u * dy) / (2.4 - u * 1.2)
        };
        let (y66, y55b, x15, x86) = (v.y(0.66), v.y(0.55), v.x(0.15), v.x(0.86));
        self.trees = Some(bld.img(paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            let b = bay(x, y);
            for &(tx, tw, dir) in &trunks {
                let cx = tx + (yf / h * 2.4).sin() * w * 0.012 * dir;
                let wy = tw * (1. + sstep(y66, y86, yf) * 0.9 * (1. + (xf * 0.3).sin() * 0.2));
                if (xf - cx).abs() < wy / 2. && yf < y86 + fbm(n, xf / 3., 4., 2) * 6. {
                    let s = (xf - (cx - wy / 2.)) / wy;
                    let side = if dir > 0. { s } else { 1. - s };
                    let mut c = if side > 0.8 || (side > 0.68 && b > 0.5) {
                        bl
                    } else if side > 0.42 {
                        bm
                    } else if side > 0.18 {
                        bd
                    } else {
                        bg
                    };
                    if fbm(n, xf / 1.3, yf / 14., 2) > 0.64 {
                        c = if c == bl {
                            bm
                        } else if c == bm {
                            bd
                        } else {
                            bg
                        };
                    }
                    if yf > y55b && side < 0.6 && fbm(n, xf / 3. + 5., yf / 3., 2) > 0.62 {
                        c = bmoss;
                    }
                    return op(c);
                }
            }
            let vv = fbm(n, xf / 6. + 100., yf / 6., 3);
            let cbx = cb[x as usize] as f64;
            let in_can = yf < cbx + (fbm(n, xf / 4. + 60., yf / 4., 2) - 0.5) * 16. * k;
            if !in_can || vv < 0.45 {
                for br in &branches {
                    if seg(xf, yf, br) < 1. {
                        return op(if seg(xf, yf - 1., br) >= 1. { bl } else { bm });
                    }
                }
            }
            if !in_can {
                return None;
            }
            if fbm(n, xf / 3. + 200., yf / 3., 2) > 0.74 && yf < cbx - 8. && xf > x15 && xf < x86 {
                return op(bsky);
            }
            op(if vv > 0.63 {
                l1
            } else if vv > 0.52 {
                l2
            } else if vv > 0.4 {
                l3
            } else {
                l4
            })
        })));

        let nb = jround(190. * k) as usize;
        let mut blades: Vec<Blade> = (0..nb)
            .map(|_| {
                let x = v.x(-0.03) + r.next() * w * 1.06;
                let hh = 3. + r.next() * 10.;
                let c = (r.next() * 3.) as usize;
                let ph = r.next() * 6.;
                let fl = if r.next() < 0.08 { Some(flc[(r.next() * 6.) as usize]) } else { None };
                Blade { x, h: hh, c, ph, fl }
            })
            .collect();
        blades.sort_by_key(|b| b.c);
        self.blades = blades;
        self.flies = [("#ff8a3a", 0.3, 0.52), ("#f2603a", 0.66, 0.6), ("#f6d24a", 0.45, 0.7)]
            .iter()
            .enumerate()
            .map(|(i, &(c, a, b))| Fly { c: hex(c), cx: v.x(a), cy: v.y(b), ph: i as f64 * 2.1 })
            .collect();
        self.motes = (0..44)
            .map(|_| {
                let x = v.x(0.25) + r.next() * w * 0.5;
                let y = v.y(0.06) + r.next() * h * 0.6;
                Mote { x, y, ph: r.next() * 6., sp: 0.6 + r.next() * 1.4 }
            })
            .collect();
        self.leaves = (0..5)
            .map(|_| {
                let x = v.x(0.1) + r.next() * w * 0.8;
                let y = v.y(-0.1) - r.next() * h;
                let vv = 4. + r.next() * 4.;
                let ph = r.next() * 6.;
                Leaf { x, y, v: vv, ph, c: hex(if r.next() < 0.5 { "#c8d860" } else { "#e8b850" }) }
            })
            .collect();
    }

    fn update(&mut self, v: &Vp, t: f64, dt: f64) {
        for m in &mut self.motes {
            m.x += (t * 0.3 + m.ph).sin() * dt * 2.;
            m.y += (-0.4 + (t * 0.5 + m.ph).sin()) * dt * 1.6;
            if m.y < v.y(0.03) {
                m.y = v.y(0.66);
            }
        }
        for l in &mut self.leaves {
            l.y += l.v * dt;
            l.x += (t * 1.3 + l.ph).sin() * dt * 6.;
            if l.y > v.y(1.02) {
                l.y = v.y(-0.05);
                l.x = v.x(0.1) + rand() * v.w * 0.8;
            }
        }
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(sky) = &self.sky else { return };
        f.fill_hex("#86c4dc");
        f.rect(0., 0., v.w, v.h);
        f.layer(v, sky, 0.03, 1.);
        f.layer(v, self.far.as_ref().unwrap(), 0.1, 1.);
        f.layer(v, self.mid.as_ref().unwrap(), 0.22, 1.);
        f.layer(v, self.ground.as_ref().unwrap(), 0.4, 1.);
        f.layer(v, self.pond.as_ref().unwrap(), 0.5, 1.);
        // pond glints
        let mut rr = Rng::new(((t * 6.).floor() as i64 + 1) as u32);
        let pts = &self.pond_pts;
        let (ox, oy) = v.at(0., 0., 0.5);
        f.fill_hex("#ffffff");
        if !pts.is_empty() {
            for _ in 0..14 {
                let j = (rr.next() * pts.len() as f64) as usize;
                f.alpha = 0.5 + rr.next() * 0.5;
                let ww = 1. + (rr.next() * 2.).floor();
                f.rect(pts[j].0 + ox, pts[j].1 + oy, ww, 1.);
            }
        }
        f.alpha = 1.;
        // "screen" isn't available; with near-white rays source-over is nearly identical
        f.layer(v, self.rays.as_ref().unwrap(), 0.3, 0.75 + 0.25 * (t * 0.35).sin());
        f.fill_hex("#fffbe0");
        for m in &self.motes {
            f.alpha = 0.25 + 0.75 * (t * m.sp + m.ph).sin().max(0.);
            f.px(v, m.x, m.y, 0.45, 1., 1.);
        }
        f.alpha = 1.;
        for fl in &self.flies {
            let x = fl.cx + (t * 0.23 + fl.ph).sin() * v.w * 0.17 + (t * 0.71 + fl.ph).sin() * v.w * 0.04;
            let y = fl.cy + (t * 0.37 + fl.ph * 1.3).sin() * v.h * 0.07 + (t * 2.2 + fl.ph).sin() * 1.5;
            let (a, b) = v.at(x, y, 0.6);
            let open = (t * 14. + fl.ph).sin() > 0.;
            f.fill(fl.c);
            if open {
                f.rect(a - 2., b - 1., 2., 2.);
                f.rect(a + 1., b - 1., 2., 2.);
                f.rect(a - 1., b + 1., 1., 1.);
                f.rect(a + 1., b + 1., 1., 1.);
            } else {
                f.rect(a - 1., b - 1., 1., 2.);
                f.rect(a + 1., b - 1., 1., 2.);
            }
            f.fill_hex("#2a1a10");
            f.rect(a, b - 1., 1., 3.);
        }
        f.layer(v, self.trees.as_ref().unwrap(), 0.85, 1.);
        for l in &self.leaves {
            f.fill(l.c);
            let tilt = (t * 2. + l.ph).sin() > 0.;
            f.px(v, l.x, l.y, 0.9, if tilt { 2. } else { 1. }, if tilt { 1. } else { 2. });
        }
        let col = [hex("#2f6a2c"), hex("#3e8436"), hex("#5aa040")];
        let (bx, by) = v.at(0., v.y(1.) + 1., 1.05);
        for b in &self.blades {
            f.fill(col[b.c]);
            let sway = (t * 1.1 + b.x * 0.04).sin() * 0.6 + (t * 2.3 + b.ph).sin() * 0.25;
            // one 1px-wide run per column the blade occupies (the mock fills 1×1 per row)
            let n = b.h.ceil() as i64;
            let mut run: Option<(f64, i64, i64)> = None; // (x, first j, last j)
            for j in 0..n {
                let jf = j as f64;
                let x = jround(b.x + bx + sway * (jf / b.h).powi(2) * b.h * 0.25);
                match run {
                    Some((rx, j0, _)) if rx == x => run = Some((rx, j0, j)),
                    _ => {
                        if let Some((rx, j0, j1)) = run {
                            f.rect(rx, by - j1 as f64, 1., (j1 - j0 + 1) as f64);
                        }
                        run = Some((x, j, j));
                    }
                }
            }
            if let Some((rx, j0, j1)) = run {
                f.rect(rx, by - j1 as f64, 1., (j1 - j0 + 1) as f64);
            }
            if let Some(fc) = b.fl {
                f.fill(fc);
                let x = jround(b.x + bx + sway * b.h * 0.25);
                f.rect(x - 1., by - b.h, 3., 1.);
                f.rect(x, by - b.h - 1., 1., 3.);
            }
        }
    }
}
