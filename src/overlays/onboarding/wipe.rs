//! The pixel-dissolve wipe (port of the mock's `startWipe` / `drawWipe`).
//!
//! The mock snapshots the old canvas, then every frame builds a cols×rows mask
//! (7 px blocks, bayer threshold vs. the wobbly distance from the origin) and a glow
//! image for freshly flipped cells. GPUI has no image masks, so instead:
//! * the old frame is uploaded **once** as a nearest-upscaled image;
//! * each frame, every row of still-covered blocks is merged into runs and each run
//!   is drawn as a cropped sprite of that image (`paint_image` with clip bounds; GPUI
//!   batches all of them into one instanced draw);
//! * glowing cells are quads (source-over at the mock's alpha; "lighter" isn't
//!   available), merged into runs of equal alpha.
//!
//! The per-cell distance is static for a wipe, so it's computed once at start.
use super::story::ease;
use crate::scene::px::{Builder, Layer, bay, clamp};
use gpui_kit::*;
use std::{rc::Rc, sync::Arc};

pub const BLOCK: f32 = 7.;
pub const DUR: f64 = 1.15;

#[derive(Clone)]
pub struct Wipe {
    pub img: Arc<RenderImage>,
    /// logical size of the snapshot image (placed at the window origin)
    iw: f32,
    ih: f32,
    cols: usize,
    rows: usize,
    d: Rc<Vec<f32>>,
    pub p: f64,
    col: [u8; 3],
    pub freeze: bool,
}

impl Wipe {
    /// `src` is the old frame at art resolution (1 art px = `unit` logical px).
    /// (`ox`, `oy`) is the origin in logical px; `col` the scene's GLOW colour.
    pub fn new(src: Layer, unit: f32, sf: f32, vw: f32, vh: f32, ox: f32, oy: f32, col: [u8; 3]) -> Self {
        let (iw, ih) = (src.w as f32 * unit, src.h as f32 * unit);
        let mut b = Builder::new(unit * sf);
        let img = b.img_raw(src, 0., 0., 1).img.clone();
        let (cols, rows) = ((vw / BLOCK).ceil() as usize, (vh / BLOCK).ceil() as usize);
        let (cx, cy) = (ox as f64 / BLOCK as f64, oy as f64 / BLOCK as f64);
        let (cf, rf) = (cols as f64, rows as f64);
        let max_d = cx.hypot(cy).max((cf - cx).hypot(cy)).max(cx.hypot(rf - cy)).max((cf - cx).hypot(rf - cy));
        let seed = crate::scene::px::rand() * 10.;
        let mut d = Vec::with_capacity(cols * rows);
        for y in 0..rows {
            for x in 0..cols {
                let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                let ang = dy.atan2(dx);
                d.push((dx.hypot(dy) / max_d + (ang * 5. + seed).sin() * 0.025 + (ang * 11. - seed).sin() * 0.012) as f32);
            }
        }
        Wipe { img, iw, ih, cols, rows, d: Rc::new(d), p: 0., col, freeze: false }
    }

    /// Advance; returns true when finished.
    pub fn advance(&mut self, dt: f64) -> bool {
        if !self.freeze {
            self.p += dt / DUR;
        }
        self.p >= 1.
    }

    /// Paint the still-covered blocks of the old frame plus the glowing edge.
    pub fn paint(&self, b: Bounds<Pixels>, window: &mut Window) {
        let pp = (ease(clamp(self.p)) * 1.35) as f32;
        let o = b.origin;
        let ib = Bounds { origin: o, size: size(px(self.iw), px(self.ih)) };
        let [cr, cg, cb] = self.col;
        let glow = |a: f32| -> Hsla { Rgba { r: cr as f32 / 255., g: cg as f32 / 255., b: cb as f32 / 255., a }.into() };
        let cell = |x0: usize, x1: usize, y: usize| Bounds {
            origin: point(o.x + px(x0 as f32 * BLOCK), o.y + px(y as f32 * BLOCK)),
            size: size(px((x1 - x0) as f32 * BLOCK), px(BLOCK)),
        };
        for y in 0..self.rows {
            let row = &self.d[y * self.cols..(y + 1) * self.cols];
            // state per cell: None = covered (old frame), Some(alpha) = gone (+ glow)
            let st = |x: usize| -> Option<u8> {
                let local = ((pp - row[x]) / 0.22).clamp(0., 1.);
                let th = bay(x as i64, y as i64) as f32;
                if local > th {
                    let edge = (1. - (local - th) * 6.).clamp(0., 1.);
                    // quantize the glow alpha so neighbouring cells merge into one quad
                    Some(((edge * edge * 200.) / 255. * 32.).round() as u8)
                } else {
                    None
                }
            };
            let mut x = 0;
            while x < self.cols {
                let s = st(x);
                let mut e = x + 1;
                while e < self.cols && st(e) == s {
                    e += 1;
                }
                match s {
                    None => {
                        window.paint_image(cell(x, e, y), ib, Corners::default(), self.img.clone(), 0, false).ok();
                    }
                    Some(a) if a > 0 => window.paint_quad(fill(cell(x, e, y), glow(a as f32 / 32.))),
                    _ => {}
                }
                x = e;
            }
        }
    }
}

/// Recorded passes composited on the CPU into an opaque art-resolution layer.
pub fn snapshot(passes: &[crate::scene::px::Pass], w: usize, h: usize, bg: [u8; 3]) -> Layer {
    let rgb = crate::scene::px::composite(passes, w, h);
    let mut l = Layer::new(w, h);
    for (i, c) in rgb.iter().enumerate() {
        let q = |v: f32, b: u8| if v.is_finite() { (v.clamp(0., 1.) * 255.).round() as u8 } else { b };
        l.d[i] = [q(c[0], bg[0]), q(c[1], bg[1]), q(c[2], bg[2]), 255];
    }
    l
}
