//! Theme "Auto": derive a palette from the backdrop behind the composer
//! (port of `extract()` in the mock's THEME ENGINE).
use crate::theme::{Spec, hsl2rgb, rgb2hsl};

/// `rgb` is a W×H art-resolution frame (0..1), resampled to the mock's 64×40 probe first.
pub fn extract(rgb: &[[f32; 3]], w: usize, h: usize) -> Option<Spec> {
    const PW: usize = 64;
    const PH: usize = 40;
    if w == 0 || h == 0 {
        return None;
    }
    // Like the browser's (low-quality, bilinear) drawImage downscale: sample each probe
    // cell's centre, so bright dither pixels survive instead of being box-averaged away.
    let mut probe = [[0f32; 3]; PW * PH];
    let at = |x: usize, y: usize| rgb[y.min(h - 1) * w + x.min(w - 1)];
    for py in 0..PH {
        let sy = ((py as f32 + 0.5) * h as f32 / PH as f32 - 0.5).max(0.);
        let (y0, fy) = (sy.floor() as usize, sy.fract());
        for px in 0..PW {
            let sx = ((px as f32 + 0.5) * w as f32 / PW as f32 - 0.5).max(0.);
            let (x0, fx) = (sx.floor() as usize, sx.fract());
            let (a, b, c, d) = (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1));
            let mut o = [0f32; 3];
            for i in 0..3 {
                let top = a[i] + (b[i] - a[i]) * fx;
                let bot = c[i] + (d[i] - c[i]) * fx;
                o[i] = (top + (bot - top) * fy) * 255.;
            }
            probe[py * PW + px] = o;
        }
    }
    let mut bins = [[0f64; 4]; 24];
    let (mut wsum, mut l_sum, mut tr, mut tg, mut tb, mut energy) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    for y in 0..34 {
        // skip the bottom fade into the dashboard
        for x in 0..PW {
            let c = probe[y * PW + x].map(|v| v.round());
            let wc = 1. + 1.5 * (-((x as f64 - 32.) / 18.).powi(2) - ((y as f64 - 17.) / 10.).powi(2)).exp();
            let [hh, s, l] = rgb2hsl(c);
            let (hh, s, l) = (hh as f64, s as f64, l as f64);
            let chroma = s * (1. - (2. * l - 1.).abs());
            let v = chroma * chroma * wc;
            let cd = [c[0] as f64, c[1] as f64, c[2] as f64];
            wsum += wc;
            l_sum += (0.2126 * cd[0] + 0.7152 * cd[1] + 0.0722 * cd[2]) / 255. * wc;
            tr += cd[0] * wc;
            tg += cd[1] * wc;
            tb += cd[2] * wc;
            energy += l;
            let b = &mut bins[((hh / 15.).floor() as usize) % 24];
            b[0] += v;
            b[1] += cd[0] * v;
            b[2] += cd[1] * v;
            b[3] += cd[2] * v;
        }
    }
    if wsum == 0. || energy < 20. {
        return None;
    }
    let score = |i: usize| bins[i][0] + 0.5 * (bins[(i + 23) % 24][0] + bins[(i + 1) % 24][0]);
    let mut p = 0;
    for i in 1..24 {
        if score(i) > score(p) {
            p = i;
        }
    }
    let mut q: Option<usize> = None;
    for i in 0..24 {
        let d = (i as i32 - p as i32).abs();
        let dist = d.min(24 - d);
        if dist >= 3 && q.is_none_or(|q| score(i) > score(q)) {
            q = Some(i);
        }
    }
    let col = |i: usize| -> Option<[f32; 3]> {
        let b = bins[i];
        if b[0] == 0. { None } else { Some([(b[1] / b[0]) as f32, (b[2] / b[0]) as f32, (b[3] / b[0]) as f32]) }
    };
    let avg = [(tr / wsum) as f32, (tg / wsum) as f32, (tb / wsum) as f32];
    let light = l_sum / wsum > 0.5;
    let accent = col(p).unwrap_or(avg);
    let mut second = match q {
        Some(q) if score(q) > score(p) * 0.12 => col(q),
        _ => None,
    };
    if second.is_none() {
        let [h, s, l] = rgb2hsl(accent);
        second = Some(hsl2rgb([(h + 40.) % 360., s, l * 0.7]));
    }
    let [th, ts, _] = rgb2hsl(avg);
    Some(Spec {
        accent: accent.map(|v| v.round()),
        second: second.unwrap().map(|v| v.round()),
        tint: (th.round(), (ts * 1000.).round() / 1000.),
        light,
    })
}
