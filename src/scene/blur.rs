//! Pre-blurred backdrops for the chat pane ("bleed"). GPUI has no backdrop-filter,
//! so instead of blurring live we ship one heavily blurred still per scene
//! (`assets/blur/<id>.png`, made by `TAKO_EXPORT_BLUR=assets/blur`), drawn behind the
//! chat with the theme's bg over it at (1 - bleed) opacity. A user-supplied image
//! would go through the same `blur_rgb` once on upload.

use super::{SCENES, info, px::{self, Builder, Frame, Vp}, valley};
use gpui_kit::RenderImage;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

/// Art size the scene is composed at for the still (≈ a 1680×1050 window at 3 px/art px).
const ART: (usize, usize) = (560, 350);
/// Output is the art downsampled by this factor, then blurred.
const DOWN: usize = 2;
const RADIUS: usize = 10;

fn key(id: &str, light: bool) -> String {
    if light && id == "valley" { "valley-light".into() } else { id.to_string() }
}

/// Compose one still of the scene and composite it to RGB (0..1).
fn still(id: &str, light: bool) -> (Vec<[f32; 3]>, usize, usize) {
    let inf = info(id);
    if inf.id == "valley" {
        valley::set_light(light);
    }
    let (w, h) = ART;
    let s = inf.px * 2.;
    let vp = Vp::new(w, h, s, inf.shift);
    let mut scene = (inf.make)();
    let mut bld = Builder::new(s as f32);
    scene.build(&mut bld, &vp, 0.);
    // let particles / creatures settle into place
    let mut t = 0.;
    for _ in 0..60 {
        t += 1. / 30.;
        scene.update(&vp, t, 1. / 30.);
    }
    let mut f = Frame::new(vp.s);
    scene.draw(&vp, t, &mut f);
    let passes = f.finish();
    (px::composite(&passes, w, h), w, h)
}

/// Downsample + 3-pass box blur (≈ gaussian). Returns RGB bytes, w, h.
pub fn blur_rgb(src: &[[f32; 3]], w: usize, h: usize, down: usize, radius: usize) -> (Vec<u8>, usize, usize) {
    let (ow, oh) = (w / down, h / down);
    let mut a = vec![[0f32; 3]; ow * oh];
    for y in 0..oh {
        for x in 0..ow {
            let mut c = [0f32; 3];
            for dy in 0..down {
                for dx in 0..down {
                    let p = src[(y * down + dy) * w + x * down + dx];
                    for i in 0..3 {
                        c[i] += p[i];
                    }
                }
            }
            a[y * ow + x] = c.map(|v| v / (down * down) as f32);
        }
    }
    let mut b = a.clone();
    for _ in 0..3 {
        box_pass(&a, &mut b, ow, oh, radius, true);
        box_pass(&b, &mut a, ow, oh, radius, false);
    }
    // ordered dither so dark gradients don't band in 8 bits
    const BAYER: [f32; 16] = [0., 8., 2., 10., 12., 4., 14., 6., 3., 11., 1., 9., 15., 7., 13., 5.];
    let mut out = Vec::with_capacity(ow * oh * 3);
    for y in 0..oh {
        for x in 0..ow {
            let d = (BAYER[(y % 4) * 4 + x % 4] / 16. - 0.5) / 255.;
            for i in 0..3 {
                out.push(((a[y * ow + x][i] + d).clamp(0., 1.) * 255.).round() as u8);
            }
        }
    }
    (out, ow, oh)
}

fn box_pass(src: &[[f32; 3]], dst: &mut [[f32; 3]], w: usize, h: usize, r: usize, horiz: bool) {
    let (n, m) = if horiz { (h, w) } else { (w, h) };
    let at = |line: usize, i: usize| if horiz { line * w + i } else { i * w + line };
    let k = 1. / (2 * r + 1) as f32;
    for line in 0..n {
        let mut acc = [0f32; 3];
        for j in 0..=2 * r {
            let i = (j as i64 - r as i64).clamp(0, m as i64 - 1) as usize;
            for c in 0..3 {
                acc[c] += src[at(line, i)][c];
            }
        }
        for i in 0..m {
            dst[at(line, i)] = acc.map(|v| v * k);
            let add = (i + r + 1).min(m - 1);
            let sub = (i as i64 - r as i64).max(0) as usize;
            for c in 0..3 {
                acc[c] += src[at(line, add)][c] - src[at(line, sub)][c];
            }
        }
    }
}

fn to_render(rgb: &[u8], w: usize, h: usize) -> Arc<RenderImage> {
    let mut im = image::RgbaImage::new(w as u32, h as u32);
    for (i, p) in im.pixels_mut().enumerate() {
        // BGRA
        *p = image::Rgba([rgb[i * 3 + 2], rgb[i * 3 + 1], rgb[i * 3], 255]);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(im)]))
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Arc<RenderImage>>> = Default::default();
}

/// The blurred still for a scene: the bundled PNG, or computed once if it's missing.
pub fn blurred(id: &str, light: bool) -> Arc<RenderImage> {
    let k = key(id, light);
    if let Some(i) = CACHE.with(|c| c.borrow().get(&k).cloned()) {
        return i;
    }
    let img = crate::assets::get(&format!("blur/{k}.png"))
        .and_then(|b| image::load_from_memory(&b).ok())
        .map(|d| {
            let d = d.to_rgb8();
            to_render(d.as_raw(), d.width() as usize, d.height() as usize)
        })
        .unwrap_or_else(|| {
            let (src, w, h) = still(id, light);
            let (rgb, w, h) = blur_rgb(&src, w, h, DOWN, RADIUS);
            to_render(&rgb, w, h)
        });
    CACHE.with(|c| c.borrow_mut().insert(k, img.clone()));
    img
}

/// `TAKO_EXPORT_BLUR=dir`: write every scene's blurred still as PNG and return.
pub fn export(dir: &str) {
    std::fs::create_dir_all(dir).ok();
    let mut jobs: Vec<(&str, bool)> = SCENES.iter().map(|s| (s.id, false)).collect();
    jobs.push(("valley", true));
    for (id, light) in jobs {
        let t0 = std::time::Instant::now();
        let (src, w, h) = still(id, light);
        let (rgb, w, h) = blur_rgb(&src, w, h, DOWN, RADIUS);
        let path = format!("{dir}/{}.png", key(id, light));
        image::save_buffer(&path, &rgb, w as u32, h as u32, image::ColorType::Rgb8).expect("write blur png");
        eprintln!("{path} {w}x{h} in {:?}", t0.elapsed());
    }
    valley::set_light(false);
}
