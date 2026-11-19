//! NEON RAIN — a rainy rooftop at night: skyline, flickering neon (one of them a
//! certain octopus), puddles, a drifting blimp.
use super::Scene;
use super::px::*;
use std::rc::Rc;

struct Blink {
    x: f64,
    y: f64,
    c: C3,
    ph: f64,
    sp: f64,
}
struct Sign {
    pts: Vec<(f64, f64)>,
    z: f64,
    x: f64,
    y: f64,
    m: f64,
    col: C3,
    dim: C3,
    g: Rc<Img>,
    on: bool,
    burst: i32,
    tm: f64,
    every: f64,
}
struct Puddle {
    x: f64,
    y: f64,
    rx: f64,
    ry: f64,
    c: C3,
}
struct Drop {
    x: f64,
    y: f64,
    near: bool,
    len: f64,
    v: f64,
    stop: f64,
}
struct Fx {
    x: f64,
    y: f64,
    age: f64,
    life: f64,
    ph: f64,
    c: C3,
}

#[derive(Default)]
pub struct Neon {
    k: f64,
    sky: Option<Rc<Img>>,
    vfar: Option<Rc<Img>>,
    far: Option<Rc<Img>>,
    haze: Option<Rc<Img>>,
    mid: Option<Rc<Img>>,
    fg: Option<Rc<Img>>,
    spills: Vec<Rc<Img>>,
    blimp_c: Option<Rc<Img>>,
    blink: Vec<Blink>,
    ant: Option<(f64, f64)>,
    signs: Vec<Sign>,
    puddles: Vec<Puddle>,
    fan: (f64, f64, f64),
    vent: (f64, f64),
    rain: Vec<Drop>,
    splash: Vec<Fx>,
    steam: Vec<Fx>,
    ripples: Vec<Fx>,
    blimp: (f64, f64, f64, f64),
}

pub fn new() -> Box<dyn Scene> {
    Box::<Neon>::default()
}

fn glyph(s: u32) -> Vec<String> {
    let mut g = vec![];
    for j in 0..5u32 {
        let mut row = String::new();
        for i in 0..5u32 {
            let on = if j == 0 || j == 4 || i == 0 || i == 4 {
                ((s >> ((i + j * 3) % 12)) & 1) != 0 || i == 2 || j == 2
            } else {
                (i == 2 || j == 2) && ((s >> (i * 2 + j)) & 1) != 0
            };
            row.push(if on { '#' } else { '.' });
        }
        g.push(row);
    }
    g
}

const OCTO: [&str; 14] = [
    ".....#####.....",
    "...##.....##...",
    "..#.........#..",
    ".#...........#.",
    ".#..##...##..#.",
    ".#..##...##..#.",
    ".#...........#.",
    "..#...###...#..",
    "..#.........#..",
    ".#.#.#...#.#.#.",
    ".#..#.#.#.#..#.",
    "#...#.#.#.#...#",
    "#..#..#.#..#..#",
    ".##..#...#..##.",
];

impl Scene for Neon {
    fn build(&mut self, b: &mut Builder, v: &Vp, _t: f64) {
        let (fw, fh, w, h) = (v.fw, v.fh, v.w, v.h);
        let fhf = fh as f64;
        let mut r = Rng::new(63);
        let n = Noise::new(23);
        let k = w / 400.;
        let hz = jround(v.y(0.8));
        let sc = jround(k * 2.).max(2.);
        self.k = k;
        let step = |s: f64| jround(s).max(1.);

        // Sky: city glow from below, low rain clouds lit magenta
        let sk = stops(&[(0., "#07040f"), (0.4, "#130a26"), (0.72, "#2c1244"), (1., "#5c1e5e")]);
        let (clc, clhi) = (hex("#2a1440"), hex("#46205e"));
        self.sky = Some(b.img(paint(fw, fh, |x, y| {
            let p = y as f64 / hz;
            let bb = bay(x, y);
            let mut c = ramp(&sk, p, x, y);
            let cl = fbm(n, x as f64 / (60. * k), y as f64 / (10. * k), 4) * sstep(0.02, 0.2, p) * (1. - sstep(0.5, 0.8, p));
            if cl > 0.32 + bb * 0.12 {
                c = mix(c, if cl > 0.42 { clhi } else { clc }, 0.6);
            }
            op(c)
        })));

        // Skylines: a pale far one, then a lit one with windows
        let mut ant = None;
        let mut city = |col: &str, edge: &str, hmin: f64, hmax: f64, win: Option<&[&str]>, seed: u32| -> (Layer, Vec<Blink>) {
            let (col, edge) = (hex(col), hex(edge));
            let mut rr = Rng::new(seed);
            let mut c = Layer::new(fw, fh);
            let mut out = vec![];
            let mut x = -2.;
            while x < fw as f64 {
                let bw = jround((5. + rr.next() * 13.) * k).max(1.);
                let bh = jround(h * (hmin + rr.next() * (hmax - hmin)));
                let top = hz - bh;
                c.rect(col, x, top, bw, fhf - top);
                c.rect(edge, x, top, 1., fhf - top);
                c.rect(edge, x, top, bw, 1.);
                if rr.next() < 0.3 {
                    let sw = jround(bw * 0.4).max(2.);
                    c.rect(col, x + jround((bw - sw) / 2.), top - jround(3. * k), sw, jround(3. * k));
                }
                if let Some(win) = win {
                    let mut wy = top + 2.;
                    while wy < hz + 4. {
                        let mut wx = x + 2.;
                        while wx < x + bw - 1. {
                            if rr.next() < 0.3 {
                                let cc = hex(win[(rr.next() * win.len() as f64) as usize]);
                                c.rect(cc, wx, wy, 1., 1.);
                                if rr.next() < 0.05 {
                                    let ph = rr.next() * 6.;
                                    let sp = 0.15 + rr.next() * 0.4;
                                    out.push(Blink { x: wx, y: wy, c: cc, ph, sp });
                                }
                            }
                            wx += 2.;
                        }
                        wy += 3.;
                    }
                    if bh > h * (hmax - 0.06) && ant.is_none() {
                        let ah = jround(10. * k);
                        c.rect(col, x + jround(bw / 2.), top - ah, 1., ah);
                        ant = Some((x + jround(bw / 2.), top - ah - 1.));
                    }
                }
                x += bw + if rr.next() < 0.25 { jround(2. * k) } else { 0. };
            }
            (c, out)
        };
        self.vfar = Some(b.img(city("#341d52", "#442866", 0.2, 0.46, None, 5).0));
        let (far, blink) = city("#221438", "#36205a", 0.1, 0.36, Some(&["#5ad8ff", "#ff7ad0", "#ffcf6a", "#9a8cff", "#5ad8ff", "#7a5aa8"]), 9);
        self.far = Some(b.img(far));
        self.blink = blink;
        self.ant = ant;
        self.haze = Some(b.img(paint(fw, fh, |x, y| {
            let p = (y as f64 - (hz - h * 0.22)) / (h * 0.22);
            if p > 0. && p * 1.15 > bay(x, y) + 0.1 { Some([150, 40, 130, jround(50. + p * 70.).min(255.) as u8]) } else { None }
        })));

        // Near towers framing the view
        let mut mid = Layer::new(fw, fh);
        let lit = ["#c89a50", "#4ab0d0", "#c04a96"];
        for (a, bb, tp) in [(-0.06, 0.17, 0.06), (0.17, 0.27, 0.38), (0.74, 0.85, 0.3), (0.85, 1.06, 0.1)] {
            let (x0, x1, y0) = (jround(v.x(a)), jround(v.x(bb)), jround(v.y(tp)));
            mid.rect(hex("#150d26"), x0, y0, x1 - x0, fhf - y0);
            mid.rect(hex("#2a1c44"), x0, y0, x1 - x0, 1.);
            let mut x = x0 + jround(6. * k);
            while x < x1 - 2. {
                mid.rect(hex("#1c1230"), x, y0 + 2., 1., fhf - y0);
                x += step(9. * k);
            }
            let mut y = y0 + 4.;
            while y < fhf {
                let mut x = x0 + 3.;
                while x < x1 - 3. {
                    if r.next() < 0.2 {
                        let c = if r.next() < 0.3 { hex(lit[(r.next() * 3.) as usize]) } else { hex("#2c2046") };
                        mid.rect(c, x, y, jround(2. * k).max(1.), jround(k).max(1.));
                    }
                    x += step(4. * k);
                }
                y += step(5. * k);
            }
            let cen = (a + bb) / 2. < 0.5;
            mid.rect(hex(if cen { "#8a3a8a" } else { "#2a8aa8" }), if cen { x1 - 1. } else { x0 }, y0, 1., fhf - y0);
            mid.rect(hex(if cen { "#4a1e50" } else { "#18485a" }), if cen { x1 - 2. } else { x0 + 1. }, y0, 1., fhf - y0);
        }

        // Neon signs
        let mut glow = |gw: f64, gh: f64, c: C3, m: f64| {
            b.img_raw(
                paint((gw + m * 2.) as usize, (gh + m * 2.) as usize, |x, y| {
                    let d = ((x as f64 - (gw / 2. + m)) / (gw / 2. + m)).hypot((y as f64 - (gh / 2. + m)) / (gh / 2. + m));
                    let q = (clamp(1. - d).powi(2) * 4. + bay(x, y)).floor() / 4.;
                    if q > 0. { Some([c[0], c[1], c[2], jround(q * 150.) as u8]) } else { None }
                }),
                0.,
                0.,
                1,
            )
        };
        let mut sign = |rows: &[String], x: f64, y: f64, col: &str, dim: &str, every: f64| {
            let mut pts = vec![];
            for (j, row) in rows.iter().enumerate() {
                for (i, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        pts.push((i as f64 * sc, j as f64 * sc));
                    }
                }
            }
            let sw = rows[0].len() as f64 * sc;
            let sh = rows.len() as f64 * sc;
            let m = jround(22. * k);
            let g = glow(sw, sh, hex(col), m);
            (Sign { pts, z: sc, x: jround(x - sw / 2.), y: jround(y), m, col: hex(col), dim: hex(dim), g, on: true, burst: 0, tm: 0., every }, sw, sh)
        };
        let mut vert: Vec<String> = vec!["#########".into(), "#.......#".into()];
        for s in [0x5a3, 0x9c6, 0x3b5, 0xe49] {
            for row in glyph(s) {
                vert.push(format!("#.{row}.#"));
            }
            vert.push("#.......#".into());
        }
        vert.push("#########".into());
        let mut bar: Vec<String> = vec!["#################".into(), "#...............#".into()];
        let gs: Vec<Vec<String>> = [0x2d9, 0x7a1, 0x4c6].iter().map(|s| glyph(*s)).collect();
        for j in 0..5 {
            bar.push(format!("#.{}.#", gs.iter().map(|g| g[j].as_str()).collect::<String>()));
        }
        bar.push("#...............#".into());
        bar.push("#################".into());
        let octo: Vec<String> = OCTO.iter().map(|s| s.to_string()).collect();
        let (o, ow, _) = sign(&octo, v.x(0.08), v.y(0.15), "#ff4fb8", "#4a1438", 9.);
        let (vs, vw, vh) = sign(&vert, v.x(0.815), v.y(0.13), "#3ce6ff", "#103a48", 6.);
        let (ba, baw, _) = sign(&bar, v.x(0.085), v.y(0.34), "#ffb347", "#4a3014", 14.);
        // octopus sign hangs on a bracket; the vertical one on two arms
        let br = hex("#2a1e38");
        mid.rect(br, o.x - 2., o.y - 3., ow + 4., 1.);
        mid.rect(br, o.x + 2., o.y - 3., 1., 3.);
        mid.rect(br, o.x + ow - 3., o.y - 3., 1., 3.);
        mid.rect(br, vs.x + vw, vs.y + 3., jround(v.x(0.85)) - vs.x - vw, 1.);
        mid.rect(br, vs.x + vw, vs.y + vh - 4., jround(v.x(0.85)) - vs.x - vw, 1.);
        mid.rect(br, ba.x + 3., ba.y - jround(3. * k), 1., jround(3. * k));
        mid.rect(br, ba.x + baw - 4., ba.y - jround(3. * k), 1., jround(3. * k));
        self.signs = vec![o, vs, ba];
        self.mid = Some(b.img(mid));

        // Foreground rooftop: parapet, wet tar, puddles, water tower, AC unit, vent
        let roof_y = jround(v.y(0.84));
        let (tar, tar2, wall, wall_l, wall_d) = (hex("#181026"), hex("#1f1532"), hex("#241834"), hex("#9a4a9a"), hex("#140c1e"));
        self.puddles = [(0.33, 0.9, 0.09, 2.2), (0.58, 0.94, 0.12, 2.6), (0.47, 0.875, 0.05, 1.6), (0.8, 0.92, 0.07, 2.)]
            .iter()
            .enumerate()
            .map(|(i, &(a, bb, rx, ry))| Puddle { x: v.x(a), y: jround(v.y(bb)), rx: w * rx, ry: ry * k, c: hex(if i % 2 == 1 { "#3ce6ff" } else { "#ff4fb8" }) })
            .collect();
        let pud = &self.puddles;
        let in_p = |x: f64, y: f64| pud.iter().any(|p| ((x - p.x) / p.rx).powi(2) + ((y - p.y) / p.ry).powi(2) < 1.);
        let (p4, p14, p9) = (jround(4. * k), jround(14. * k), jround(9. * k));
        let mut fg = paint(fw, fh, |x, y| {
            let (xf, yf) = (x as f64, y as f64);
            if yf < roof_y {
                return None;
            }
            let bb = bay(x, y);
            if yf < roof_y + p4 {
                return op(if yf == roof_y || (yf == roof_y + 1. && bb > 0.5) {
                    wall_l
                } else if p14 > 0. && xf % p14 == 0. {
                    wall_d
                } else {
                    wall
                });
            }
            if in_p(xf, yf) {
                return op([34, 22, 60]);
            }
            let mut c = if fbm(n, xf / 6., yf / 3., 2) > 0.55 && bb > 0.3 { tar2 } else { tar };
            if p9 > 0. && (yf - roof_y) % p9 == 0. {
                c = wall_d;
            }
            op(c)
        });
        let f = &mut fg;
        let ink = hex("#0a0612");
        // water tower
        let tx = jround(v.x(0.1));
        let tw = jround(24. * k);
        let tt = jround(v.y(0.5));
        let th = jround(20. * k);
        let tl = jround(roof_y - tt - th);
        for lx in [tx - tw / 2. + 2., tx + tw / 2. - 3., tx - 2.] {
            f.rect(ink, lx, tt + th, 2., tl);
        }
        f.rect(ink, tx - tw / 2. + 2., tt + th + tl * 0.45, tw - 3., 1.);
        for x in 0..tw as i64 {
            let xf = x as f64;
            let c = if xf > tw - 3. {
                hex("#b04a9a")
            } else if x % 4 == 0 {
                hex("#120c1e")
            } else {
                hex("#1a1028")
            };
            f.rect(c, tx - tw / 2. + xf, tt, 1., th);
        }
        f.rect(ink, tx - tw / 2., tt + jround(th * 0.3), tw, 1.);
        f.rect(ink, tx - tw / 2., tt + jround(th * 0.7), tw, 1.);
        let rh = jround(9. * k);
        for i in 0..rh as i64 {
            let i = i as f64;
            let ww = jround(tw * (i / rh)) + 2.;
            f.rect(if i > rh - 2. { ink } else { hex("#120a1e") }, tx - ww / 2., tt - rh + i, ww, 1.);
            f.rect(hex("#b04a9a"), tx + ww / 2. - 1., tt - rh + i, 1., 1.);
        }
        // AC unit with a fan
        let ax = jround(v.x(0.86));
        let aw = jround(26. * k);
        let ah = jround(14. * k);
        let ay = roof_y - ah + jround(2. * k);
        f.rect(hex("#201832"), ax, ay, aw, ah);
        f.rect(hex("#3a2c56"), ax, ay, aw, 1.);
        f.rect(hex("#2a8aa8"), ax, ay, 1., ah);
        let mut y = ay + 3.;
        while y < ay + ah - 2. {
            f.rect(hex("#100a1a"), ax + 3., y, jround(aw * 0.45), 1.);
            y += 2.;
        }
        self.fan = (ax + jround(aw * 0.75), ay + jround(ah / 2.), jround(ah * 0.32));
        // vent pipe
        let vx = jround(v.x(0.69));
        let vh2 = jround(13. * k);
        let vy = roof_y - vh2 + 2.;
        f.rect(hex("#1a1228"), vx, vy, jround(4. * k), vh2);
        f.rect(hex("#2c2040"), vx - 1., vy - 1., jround(4. * k) + 2., 2.);
        self.vent = (vx + jround(2. * k), vy - 2.);
        self.fg = Some(b.img(fg));
        let mut spill = |cx: f64, c: &str| {
            let c = hex(c);
            b.img(paint(fw, fh, |x, y| {
                let yf = y as f64;
                if yf < roof_y {
                    return None;
                }
                let d = ((x as f64 - cx) / (w * 0.28)).hypot((yf - roof_y) / (h * 0.12));
                let q = (clamp(1. - d).powi(2) * 4. + bay(x, y)).floor() / 4.;
                if q > 0. { Some([c[0], c[1], c[2], jround(q * 70.) as u8]) } else { None }
            }))
        };
        self.spills = vec![spill(v.x(0.12), "#ff4fb8"), spill(v.x(0.82), "#3ce6ff")];

        // Rain, splashes, steam, ripples, blimp
        let mut drop = |near: bool| {
            let x = r.next() * fw as f64;
            let y = r.next() * fhf;
            let len = if near { (6. + r.next() * 5.) * k } else { (3. + r.next() * 3.) * k };
            let vv = h * if near { 1.5 + r.next() * 0.4 } else { 0.9 + r.next() * 0.3 };
            let stop = if near { roof_y + r.next() * (fhf - roof_y) } else { hz + r.next() * (fhf - hz) };
            Drop { x, y, near, len, v: vv, stop }
        };
        let mut rain: Vec<Drop> = (0..jround(150. * k) as usize).map(|_| drop(false)).collect();
        rain.extend((0..jround(40. * k) as usize).map(|_| drop(true)));
        self.rain = rain;
        let (bw, bh) = (jround(30. * k).max(2.), jround(9. * k).max(2.));
        self.blimp = (v.x(0.32), v.y(0.1), bw, bh);
        self.blimp_c = Some(b.img_raw(
            paint(bw as usize, bh as usize, |x, y| {
                let (xf, yf) = (x as f64, y as f64);
                let e = ((xf - bw / 2.) / (bw / 2.)).powi(2) + ((yf - bh / 2.) / (bh / 2.)).powi(2);
                if e >= 1. {
                    None
                } else if yf < bh * 0.3 && e < 0.8 {
                    Some([52, 38, 78, 255])
                } else if yf > bh * 0.75 {
                    Some([18, 12, 30, 255])
                } else {
                    Some([30, 22, 50, 255])
                }
            }),
            0.,
            0.,
            1,
        ));
    }

    fn update(&mut self, v: &Vp, t: f64, dt: f64) {
        for s in &mut self.signs {
            if s.burst > 0 {
                s.tm -= dt;
                if s.tm <= 0. {
                    s.on = !s.on;
                    s.burst -= 1;
                    s.tm = 0.03 + rand() * 0.12;
                }
            } else {
                s.on = true;
                if rand() < dt / s.every {
                    s.burst = 2 + (rand() * 3.) as i32 * 2;
                    s.tm = 0.04;
                }
            }
        }
        for d in &mut self.rain {
            d.y += d.v * dt;
            d.x -= d.v * dt * 0.18;
            if d.y > d.stop {
                if d.near && rand() < 0.6 {
                    self.splash.push(Fx { x: d.x, y: d.stop, age: 0., life: 0.2, ph: 0., c: [0; 3] });
                }
                if d.near {
                    if let Some(p) = self.puddles.iter().find(|p| ((d.x - p.x) / p.rx).powi(2) + ((d.stop - p.y) / p.ry).powi(2) < 1.) {
                        self.ripples.push(Fx { x: d.x, y: d.stop, age: 0., life: 0.6, ph: 0., c: p.c });
                    }
                }
                d.y = -d.len - rand() * v.h * 0.3;
                d.x = rand() * (v.fw as f64 + v.w * 0.3);
            }
        }
        self.splash.retain_mut(|s| {
            s.age += dt;
            s.age < 0.2
        });
        self.ripples.retain_mut(|s| {
            s.age += dt;
            s.age < 0.6
        });
        if rand() < dt * 4. {
            self.steam.push(Fx { x: self.vent.0, y: self.vent.1, age: 0., life: 1.6 + rand(), ph: rand() * 6., c: [0; 3] });
        }
        let k = self.k;
        self.steam.retain_mut(|s| {
            s.age += dt;
            s.y -= dt * 7. * k;
            s.x += (t * 2. + s.ph).sin() * dt * 3. - dt * 2. * k;
            s.age < s.life
        });
        self.blimp.0 += dt * v.w * 0.012;
        if self.blimp.0 > v.x(1.15) {
            self.blimp.0 = v.x(-0.15);
        }
    }

    fn draw(&self, v: &Vp, t: f64, f: &mut Frame) {
        let Some(sky) = &self.sky else { return };
        let k = self.k;
        f.fill_hex("#07040f");
        f.rect(0., 0., v.w, v.h);
        f.layer(v, sky, 0.02, 1.);
        // blimp with a scrolling screen and a blinking tail light
        let (bxl, byl, bw, bh) = self.blimp;
        let (bx, by) = v.at(bxl, byl, 0.06);
        f.image(self.blimp_c.as_ref().unwrap(), bx, by, 1.);
        let cols = ["#ff4fb8", "#3ce6ff", "#ffb347", "#9b87f5"];
        let sw = jround(bw * 0.5) as i64;
        let (sx0, sy0) = (bx + jround(bw * 0.25), by + jround(bh * 0.4));
        f.alpha = 0.85;
        for i in 0..sw {
            f.fill_hex(cols[(((i + (t * 6.).floor() as i64) >> 2) % 4) as usize]);
            f.rect(sx0 + i as f64, sy0, 1., jround(bh * 0.2).max(1.));
        }
        f.alpha = if (t * 4.).sin() > 0.6 { 1. } else { 0.15 };
        f.fill_hex("#ff3a4a");
        f.rect(bx, by + jround(bh / 2.), 1., 1.);
        f.alpha = 1.;
        f.layer(v, self.vfar.as_ref().unwrap(), 0.08, 1.);
        f.layer(v, self.far.as_ref().unwrap(), 0.14, 1.);
        for w in &self.blink {
            f.alpha = if (t * w.sp + w.ph).sin() > 0. { 1. } else { 0.1 };
            f.fill(w.c);
            f.px(v, w.x, w.y, 0.14, 1., 1.);
        }
        if let Some((ax, ay)) = self.ant {
            f.alpha = if (t * 2.4).sin() > 0.5 { 1. } else { 0.1 };
            f.fill_hex("#ff3a4a");
            f.px(v, ax, ay, 0.14, 1., 1.);
        }
        f.alpha = 1.;
        f.layer(v, self.haze.as_ref().unwrap(), 0.14, 1.);
        // far rain
        f.fill_hex("#8aa0d8");
        f.alpha = 0.22;
        for d in self.rain.iter().filter(|d| !d.near) {
            let (x, y) = v.at(d.x, d.y, 0.2);
            let mut i = 0.;
            while i < d.len {
                f.rect(x + i * 0.18, y - i, 1., 1.);
                i += 1.;
            }
        }
        f.alpha = 1.;
        f.layer(v, self.mid.as_ref().unwrap(), 0.3, 1.);
        for (i, s) in self.signs.iter().enumerate() {
            let (x, y) = v.at(s.x, s.y, 0.3);
            if s.on {
                f.additive = true;
                f.image(&s.g, x - s.m, y - s.m, if i == 0 { 0.9 } else { 0.75 });
                f.additive = false;
            }
            f.fill(if s.on { s.col } else { s.dim });
            for &(i, j) in &s.pts {
                f.rect(x + i, y + j, s.z, s.z);
            }
            if s.on {
                f.fill([255, 255, 255]);
                f.alpha = 0.55;
                for &(i, j) in s.pts.iter().step_by(3) {
                    f.rect(x + i, y + j, 1., 1.);
                }
                f.alpha = 1.;
            }
        }
        f.layer(v, self.fg.as_ref().unwrap(), 0.6, 1.);
        for (i, c) in self.spills.iter().enumerate() {
            f.additive = true;
            f.layer(v, c, 0.6, if self.signs[i].on { 1. } else { 0.35 });
            f.additive = false;
        }
        // puddle reflections, shimmering in the rain
        for p in &self.puddles {
            let (x0, y0) = v.at(p.x, p.y, 0.6);
            let ry = jround(p.ry).max(1.) as i64;
            f.fill(p.c);
            for j in (-ry + 1)..ry {
                let jf = j as f64;
                let hw = (p.rx * (1. - (jf / p.ry).powi(2)).max(0.).sqrt()).floor() as i64;
                let mut i = -hw;
                while i < hw {
                    let fi = i as f64;
                    let s = (fi * 0.45 + t * 3. + jf * 2.1).sin() + (fi * 0.13 - t * 1.3).sin();
                    if s > 0.4 {
                        f.alpha = 0.3 + (s - 0.4) * 0.2;
                        f.rect(x0 + fi, y0 + jf, 2., 1.);
                    }
                    i += 2;
                }
            }
        }
        for rp in &self.ripples {
            let (x, y) = v.at(rp.x, rp.y, 0.6);
            let rr = 1. + rp.age * 8. * k;
            f.alpha = 0.6 * (1. - rp.age / 0.6);
            f.fill(rp.c);
            f.rect(x - rr, y, 1., 1.);
            f.rect(x + rr, y, 1., 1.);
        }
        f.alpha = 1.;
        // AC fan spinning
        {
            let (fx, fy) = v.at(self.fan.0, self.fan.1, 0.6);
            let r = self.fan.2;
            f.fill_hex("#0a0612");
            f.rect(fx - r, fy - r, r * 2. + 1., r * 2. + 1.);
            f.fill_hex("#3a2c50");
            for a in 0..3 {
                let an = t * 9. + a as f64 * 2.094;
                let mut q = 1.;
                while q <= r {
                    f.rect(jround(fx + an.cos() * q), jround(fy + an.sin() * q), 1., 1.);
                    q += 1.;
                }
            }
        }
        // vent steam
        f.fill_hex("#9a8cb8");
        for s in &self.steam {
            let fa = s.age / s.life;
            f.alpha = 0.3 * (1. - fa);
            let z = if fa > 0.4 { 2. } else { 1. };
            f.px(v, s.x, s.y, 0.6, z, z);
        }
        // near rain + splashes
        f.fill_hex("#b4c4ee");
        f.alpha = 0.3;
        for d in self.rain.iter().filter(|d| d.near) {
            let (x, y) = v.at(d.x, d.y, 0.6);
            let mut i = 0.;
            while i < d.len {
                f.rect(x + i * 0.18, y - i, 1., 1.);
                i += 1.;
            }
        }
        for s in &self.splash {
            let (x, y) = v.at(s.x, s.y, 0.6);
            let u = s.age / 0.2;
            f.alpha = 0.7 * (1. - u);
            f.rect(x - 1. - u * 2., y - 1. - u * 2., 1., 1.);
            f.rect(x + 1. + u * 2., y - 1. - u * 2., 1., 1.);
        }
        f.alpha = 1.;
    }
}
