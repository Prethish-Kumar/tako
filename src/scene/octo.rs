//! TAKO — the blue pixel octopus. One renderer shared by the Tako backdrop and the
//! onboarding (port of the mock's `TakoOcto.draw(g, o)`).
//!
//! ```ignore
//! let mut r = Raster::new(ox - 64, oy - 30, 128, 96);          // any PixelTarget
//! draw_octo(&mut r, &Octo { x: ox, y: oy, t, pose: Pose::Sit, floor: Some(fy), ..Default::default() });
//! r.emit(&mut frame, 0., 0.);                                   // → quads
//! ```
//! Coordinates are art pixels; the octopus snaps to whole art pixels like the mock.
use super::px::{C3, PixelTarget, hex};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pose {
    #[default]
    Float,
    Sit,
}

/// Everything `draw` takes (the mock's `o`).
#[derive(Clone, Debug, Default)]
pub struct Octo {
    /// mantle centre
    pub x: f64,
    pub y: f64,
    /// time in seconds
    pub t: f64,
    /// 0..1 per arm: how far each arm reaches toward `tips[i]`
    pub reach: Option<[f64; 8]>,
    pub tips: Option<[[f64; 2]; 8]>,
    /// 0..1 — waves arm 6
    pub wave: f64,
    /// 0 open … 1 closed
    pub lid: f64,
    /// gaze, each -1..1
    pub look: [f64; 2],
    /// 0..1 colour-change shimmer
    pub ripple: f64,
    pub pose: Pose,
    /// ground y when sitting (defaults to y + 24)
    pub floor: Option<f64>,
}

pub struct Pal {
    pub out: C3,
    pub d: C3,
    pub m: C3,
    pub l: C3,
    pub h: C3,
    pub s: C3,
    pub rim: C3,
    pub suck: C3,
    pub eye: C3,
    pub pupil: C3,
}
pub fn pal() -> &'static Pal {
    static P: OnceLock<Pal> = OnceLock::new();
    P.get_or_init(|| Pal {
        out: hex("#0b1a4f"),
        d: hex("#1c3b9a"),
        m: hex("#2f5fd6"),
        l: hex("#4f86f2"),
        h: hex("#86b2ff"),
        s: hex("#d6e6ff"),
        rim: hex("#5fd2ff"),
        suck: hex("#c4d8ff"),
        eye: hex("#f6f9ff"),
        pupil: hex("#06102a"),
    })
}

const BAY: [[f64; 4]; 4] = {
    const B: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let mut o = [[0.; 4]; 4];
    let mut j = 0;
    while j < 4 {
        let mut i = 0;
        while i < 4 {
            o[j][i] = (B[j][i] as f64 + 0.5) / 16.;
            i += 1;
        }
        j += 1;
    }
    o
};
fn clamp(v: f64) -> f64 {
    v.clamp(0., 1.)
}
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}
fn ease(x: f64) -> f64 {
    if x < 0.5 { 4. * x * x * x } else { 1. - (-2. * x + 2.).powi(3) / 2. }
}
fn jr(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

// ---- mantle sprite (static shading, built once) ----
const MW: i32 = 34;
const MH: i32 = 38;
const CX: f64 = 17.;
const CY: f64 = 19.;
const RX: f64 = 14.5;
const RYT: f64 = 18.5;
const RYB: f64 = 13.5;

struct Mantle {
    px: Vec<Option<C3>>,
    mask: Vec<(i32, i32)>,
}
fn mantle() -> &'static Mantle {
    static M: OnceLock<Mantle> = OnceLock::new();
    M.get_or_init(|| {
        let c = pal();
        let inside = |i: i32, j: i32| {
            let dx = (i as f64 + 0.5 - CX) / RX;
            let dy = (j as f64 + 0.5 - CY) / if (j as f64 + 0.5) < CY { RYT } else { RYB };
            dx * dx + dy * dy <= 1.
        };
        let l0 = [-0.55f64, -0.78, 0.42];
        let ln = (l0[0] * l0[0] + l0[1] * l0[1] + l0[2] * l0[2]).sqrt();
        let lv = [l0[0] / ln, l0[1] / ln, l0[2] / ln];
        let ramp = [c.d, c.m, c.l, c.h, c.s];
        let mut px = vec![None; (MW * MH) as usize];
        let mut mask = vec![];
        for j in 0..MH {
            for i in 0..MW {
                if !inside(i, j) {
                    if inside(i + 1, j) || inside(i - 1, j) || inside(i, j + 1) || inside(i, j - 1) {
                        px[(j * MW + i) as usize] = Some(c.out);
                    }
                    continue;
                }
                let dx = (i as f64 + 0.5 - CX) / RX;
                let dy = (j as f64 + 0.5 - CY) / if (j as f64 + 0.5) < CY { RYT } else { RYB };
                let r2 = dx * dx + dy * dy;
                let nz = (1. - r2).max(0.).sqrt();
                let l = (dx * lv[0] + dy * lv[1] + nz * lv[2]) * 0.5 + 0.5;
                let spot = (i as f64 * 1.7 + j as f64 * 0.9).sin() * (i as f64 * 0.63 - j as f64 * 1.27).sin();
                let b = BAY[(j & 3) as usize][(i & 3) as usize];
                let mut v = l * 4.3 - 0.45 + (b - 0.5) * 0.3;
                if spot > 0.86 && (j as f64) < CY + 2. && v > 1.2 {
                    v -= 1.;
                }
                let mut col = ramp[(v.floor().clamp(0., 4.)) as usize];
                if r2 > 0.72 && dx * 0.8 + dy > 0.55 {
                    col = if b > 0.35 { c.rim } else { c.l }; // cool bounce light from below-right
                }
                px[(j * MW + i) as usize] = Some(col);
                mask.push((i, j));
            }
        }
        for (x, y, w, h) in [(9, 6, 3, 1), (8, 7, 2, 2), (12, 5, 1, 1)] {
            for yy in y..y + h {
                for xx in x..x + w {
                    px[(yy * MW + xx) as usize] = Some(c.s);
                }
            }
        }
        Mantle { px, mask }
    })
}

fn disc(g: &mut impl PixelTarget, x: f64, y: f64, r: f64, c: C3, a: f32) {
    let (x, y) = (jr(x), jr(y));
    let r2 = r * r;
    let ri = r.ceil() as i32;
    for dy in -ri..=ri {
        let q = r2 - (dy * dy) as f64;
        if q < 0. {
            continue;
        }
        let w = (q.sqrt() + 0.25).floor() as i32;
        g.fill(x - w, y + dy, 2 * w + 1, 1, c, a);
    }
}

const ORDER: [usize; 8] = [0, 7, 1, 6, 2, 5, 3, 4];

struct P {
    x: f64,
    y: f64,
    r: f64,
    nx: f64,
    ny: f64,
}

fn arm_pts(o: &Octo, i: usize) -> Vec<P> {
    let k = i as f64 - 3.5;
    let t = o.t;
    let fi = i as f64;
    let (bx, by) = (o.x + k * 3.1, o.y + 10.);
    let (rtx, rty);
    if o.pose == Pose::Sit {
        let fl = o.floor.unwrap_or(o.y + 24.);
        let side = if k < 0. { -1. } else { 1. };
        rtx = o.x + k * 11. + side * 6. + (t * 0.9 + fi * 0.8).sin() * 2.5;
        rty = fl - 1. + (t * 0.7 + fi).sin() * 1.2 - if k.abs() < 1. { 2. } else { 0. };
    } else {
        rtx = o.x + k * 9.5 + (t * 1.1 + fi * 0.8).sin() * 3.;
        rty = o.y + 37. - k.abs() * 3.5 + (t * 1.3 + fi).sin() * 2.;
    }
    let rr = o.reach.map(|r| ease(clamp(r[i]))).unwrap_or(0.);
    let (mut tx, mut ty) = (rtx, rty);
    if rr > 0. {
        if let Some(tips) = o.tips {
            tx = lerp(rtx, tips[i][0], rr);
            ty = lerp(rty, tips[i][1], rr);
        }
    }
    if o.wave > 0. && i == 6 {
        let wv = ease(clamp(o.wave));
        tx = lerp(tx, o.x + 30., wv);
        ty = lerp(ty, o.y - 16. + (t * 9.).sin() * 5., wv);
    }
    let sitting = o.pose == Pose::Sit && rr < 0.5;
    let cx = bx + k * 6. + (tx - bx) * if sitting { 0.55 } else { 0.35 };
    let cy = by + if sitting { 10. } else { 16. } + (ty - by) * if sitting { 0.1 } else { 0.25 };
    let len = (tx - bx).hypot(ty - by) + 12.;
    let n = ((len / 1.1 + 0.5).floor() as i32).max(14);
    let curl = if k < 0. { -1. } else { 1. };
    let calm = 1. - rr * 0.7;
    let mut out = Vec::with_capacity(n as usize + 1);
    for s in 0..=n {
        let u = s as f64 / n as f64;
        let (a, b, c) = ((1. - u) * (1. - u), 2. * (1. - u) * u, u * u);
        let mut x = a * bx + b * cx + c * tx;
        let mut y = a * by + b * cy + c * ty;
        let dx = 2. * (1. - u) * (cx - bx) + 2. * u * (tx - cx);
        let dy = 2. * (1. - u) * (cy - by) + 2. * u * (ty - cy);
        let dl = dx.hypot(dy);
        let dl = if dl == 0. { 1. } else { dl };
        let (nx, ny) = (-dy / dl, dx / dl);
        let mut w = (u * std::f64::consts::PI * 1.6 - t * 2.2 + fi * 1.3).sin() * 2.2 * u * calm;
        if u > 0.78 {
            w += ((u - 0.78) * 14.).sin() * 2.2 * calm * curl;
        }
        x += nx * w;
        y += ny * w;
        out.push(P { x, y, r: lerp(3.5, 0.7, u.powf(0.85)), nx, ny });
    }
    out
}

fn draw_arm(g: &mut impl PixelTarget, pts: &[P]) {
    let c = pal();
    for p in pts {
        disc(g, p.x, p.y, p.r + 1., c.out, 1.);
    }
    for p in pts {
        disc(g, p.x, p.y, p.r, c.m, 1.);
    }
    for p in pts {
        if p.r > 1.2 {
            disc(g, p.x - p.r * 0.3, p.y - p.r * 0.45, p.r * 0.45, c.l, 1.);
        }
    }
    for (j, p) in pts.iter().enumerate() {
        if j % 3 != 0 || p.r < 1.3 {
            continue;
        }
        let (sx, sy) = if p.ny > 0. { (p.nx, p.ny) } else { (-p.nx, -p.ny) };
        g.fill(jr(p.x + sx * p.r * 0.62), jr(p.y + sy * p.r * 0.62), if p.r > 2.6 { 2 } else { 1 }, 1, c.suck, 1.);
    }
}

/// Paint Tako into `g` (art pixels). See [`Octo`] for the knobs.
pub fn draw_octo(g: &mut impl PixelTarget, o: &Octo) {
    let c = pal();
    let ox = jr(o.x);
    let oy = jr(o.y);
    let t = o.t;
    let q = Octo { x: ox as f64, y: oy as f64, ..o.clone() };
    for i in ORDER {
        draw_arm(g, &arm_pts(&q, i));
    }
    let m = mantle();
    let (mx0, my0) = (ox - CX as i32, oy - CY as i32);
    for j in 0..MH {
        let mut i = 0;
        while i < MW {
            let Some(col) = m.px[(j * MW + i) as usize] else {
                i += 1;
                continue;
            };
            let mut e = i + 1;
            while e < MW && m.px[(j * MW + e) as usize] == Some(col) {
                e += 1;
            }
            g.fill(mx0 + i, my0 + j, e - i, 1, col, 1.);
            i = e;
        }
    }
    if o.ripple > 0. {
        let a = (0.45 * clamp(o.ripple)) as f32;
        for &(i, j) in &m.mask {
            let d = (i as f64 - CX).hypot(j as f64 - CY + 6.);
            if (d * 0.75 - t * 5.).sin() > 0.82 {
                g.fill(mx0 + i, my0 + j, 1, 1, c.h, a);
            }
        }
    }
    // eyes: big, kind, with horizontal octopus pupils
    let lid = clamp(o.lid);
    let lx = jr(o.look[0].clamp(-1., 1.));
    let ly = jr(o.look[1].clamp(-1., 1.) * 0.6);
    for dx in [-6, 6] {
        let (x, y) = (ox + dx, oy + 5);
        disc(g, x as f64, y as f64, 3.4, c.d, 1.);
        disc(g, x as f64, y as f64, 2.6, c.eye, 1.);
        g.fill(x - 1 + lx, y - 1 + ly, 3, 2, c.pupil, 1.);
        g.fill(x + lx, y + 1 + ly, 1, 1, c.pupil, 1.);
        g.fill(x - 1 + lx, y - 1 + ly, 1, 1, [255, 255, 255], 1.);
        if lid > 0. {
            let rows = (7. * lid).ceil() as i32;
            g.fill(x - 3, y - 3, 7, rows, c.m, 1.);
            if lid > 0.95 {
                g.fill(x - 2, y + 1, 5, 1, c.out, 1.);
            }
        }
    }
    let ch = hex("#6a8cff"); // cheeks
    g.fill(ox - 10, oy + 9, 2, 1, ch, 0.55);
    g.fill(ox + 9, oy + 9, 2, 1, ch, 0.55);
}

/// The 10×10 brand mark (rows of palette keys), for anyone who needs it.
#[allow(dead_code)]
pub const MARK: [&str; 10] = ["...bbbb...", "..bhbbbb..", ".bhbbbbbb.", ".bbbbbbbb.", ".bwkbbwkb.", ".bbbbbbbb.", "..bbbbbb..", ".b.bb.bb.b", "b..b..b..b", ".........."];
