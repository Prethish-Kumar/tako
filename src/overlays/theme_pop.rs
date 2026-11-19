//! Theme picker popover (port of the mock's THEME ENGINE picker).
use crate::{
    app::TakoApp,
    theme::{ActiveTheme, PRESETS, Rgb, ThemeState, alpha, hex, hsl2rgb, rgb_a, tune_accent},
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{cell::Cell, collections::HashMap, rc::Rc, sync::Arc};

pub struct ThemePopState {
    pub open: bool,
    /// Window bounds of the titlebar palette button, when the titlebar records them
    /// (see GAPS.md). Falls back to the empty-layout position.
    pub anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    shown: bool,
    tween: Option<(bool, Tween)>,
}

impl ThemePopState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        ThemePopState { open: false, anchor: Rc::default(), shown: false, tween: None }
    }
}

/// Open/close fade shared by the popovers: returns the 0..1 visibility (eased)
/// or None when fully hidden. `shown` tracks the last seen `open` value.
pub fn pop_progress(open: bool, shown: &mut bool, tween: &mut Option<(bool, Tween)>, ms_in: f32, window: &mut Window) -> Option<f32> {
    // Offscreen snapshots draw a single frame after the actions: skip the fade there.
    static SNAP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *SNAP.get_or_init(|| std::env::var("TAKO_SNAP").is_ok()) {
        *shown = open;
        *tween = None;
        return open.then_some(1.);
    }
    if open != *shown {
        // reverse mid-flight from the current value
        let cur = tween.as_ref().map(|(o, tw)| if *o { ease_snappy(tw.raw()) } else { 1. - ease_snappy(tw.raw()) });
        let mut tw = Tween::new(if open { ms_in } else { 160. });
        if let Some(c) = cur {
            let p = if open { c } else { 1. - c };
            tw.start -= std::time::Duration::from_secs_f32(tw.ms * p / 1000.);
        }
        *tween = Some((open, tw));
        *shown = open;
    }
    let v = match tween {
        Some((o, tw)) => {
            let e = ease_snappy(tw.raw());
            let done = tw.done();
            let v = if *o { e } else { 1. - e };
            if done {
                *tween = None;
            } else {
                window.request_animation_frame();
            }
            v
        }
        None => {
            if open {
                1.
            } else {
                0.
            }
        }
    };
    if !open && v <= 0. { None } else { Some(v) }
}

/// Registers a window-level Escape listener for this frame (call from a canvas paint).
pub fn escape_closer(entity: WeakEntity<TakoApp>, close: fn(&mut TakoApp)) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            let entity = entity.clone();
            window.on_key_event::<KeyDownEvent>(move |ev, phase, _, cx| {
                if phase == DispatchPhase::Bubble && ev.keystroke.key == "escape" {
                    entity
                        .update(cx, |this, cx| {
                            close(this);
                            cx.notify();
                        })
                        .ok();
                }
            });
        },
    )
    .absolute()
    .size_0()
}

// ---------------------------------------------------------------------------
// Orbs: conic / hard-stop linear gradients rasterised on the CPU (GPUI only
// has two-stop linear gradients).

#[derive(Clone, Copy)]
pub enum Grad {
    /// conic-gradient(from `from`deg, stops… evenly spaced)
    Conic { from: f32 },
    /// linear-gradient(135deg, c0 0 s0, c1 s0 s1, c2 s1) — hard stops
    Bands { s0: f32, s1: f32 },
}

thread_local! {
    static ORBS: std::cell::RefCell<HashMap<String, Arc<RenderImage>>> = Default::default();
}

fn mix(a: Rgb, b: Rgb, f: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

/// Round swatch `size` px wide, with the mock's 1px inset white ring (`ring` alpha).
pub fn orb(g: Grad, cols: &[Rgb], size: f32, ring: f32) -> impl IntoElement {
    let q = |c: &Rgb| format!("{:02x}{:02x}{:02x}", c[0] as u8, c[1] as u8, c[2] as u8);
    let key = format!(
        "{}|{}|{}|{}",
        match g {
            Grad::Conic { from } => format!("c{from}"),
            Grad::Bands { s0, s1 } => format!("b{s0}-{s1}"),
        },
        cols.iter().map(q).collect::<Vec<_>>().join(","),
        size,
        ring
    );
    let img = ORBS.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 400 {
            m.clear();
        }
        m.entry(key).or_insert_with(|| raster(g, cols, size, ring)).clone()
    });
    img_el(img, size)
}

fn img_el(img: Arc<RenderImage>, size: f32) -> impl IntoElement {
    gpui_kit::img(ImageSource::Render(img)).size(px(size)).flex_none().rounded_full()
}

fn raster(g: Grad, cols: &[Rgb], size: f32, ring: f32) -> Arc<RenderImage> {
    let sc = 4.;
    let n = (size * sc).round() as u32;
    let r = n as f32 / 2.;
    let mut im = image::RgbaImage::new(n, n);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 + 0.5 - r, y as f32 + 0.5 - r);
            let d = (dx * dx + dy * dy).sqrt();
            let cov = (r - d + 0.5).clamp(0., 1.);
            if cov <= 0. {
                continue;
            }
            let mut c = match g {
                Grad::Conic { from } => {
                    let th = dx.atan2(-dy).to_degrees();
                    let t = ((th - from).rem_euclid(360.)) / 360.;
                    let segs = (cols.len() - 1) as f32;
                    let p = t * segs;
                    let i = (p.floor() as usize).min(cols.len() - 2);
                    mix(cols[i], cols[i + 1], p - i as f32)
                }
                Grad::Bands { s0, s1 } => {
                    // 135deg: gradient runs top-left → bottom-right, length = size·√2
                    let t = (dx + dy) * std::f32::consts::FRAC_1_SQRT_2 / (n as f32 * std::f32::consts::SQRT_2) + 0.5;
                    let edge = 0.6 / (n as f32 * std::f32::consts::SQRT_2);
                    let w0 = ((t - s0) / edge + 0.5).clamp(0., 1.);
                    let a = mix(cols[0], cols[1], w0);
                    if cols.len() > 2 {
                        let w1 = ((t - s1) / edge + 0.5).clamp(0., 1.);
                        mix(a, cols[2], w1)
                    } else {
                        a
                    }
                }
            };
            // inset 0 0 0 1px rgba(255,255,255,ring)
            let rw = (d - (r - sc)).clamp(0., 1.);
            if rw > 0. {
                c = mix(c, [255., 255., 255.], ring * rw);
            }
            let a = (cov * 255.).round() as u8;
            // BGRA
            im.put_pixel(x, y, image::Rgba([c[2].round() as u8, c[1].round() as u8, c[0].round() as u8, a]));
        }
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(im)]))
}

fn hsl_rgb(h: f32, s: f32, l: f32) -> Rgb {
    hsl2rgb([h, s.clamp(0., 1.), l.clamp(0., 1.)])
}

/// The colours the Auto orb uses: tuned accent, second, and the derived bg.
pub fn auto_orb_cols(ts: &ThemeState) -> ([Rgb; 3], bool) {
    let a = ts.auto.unwrap_or(crate::theme::Spec { accent: hex("#e2795a"), second: hex("#8a3a7a"), tint: (350., 0.1), light: false });
    let bg = hsl_rgb(a.tint.0, a.tint.1.clamp(0.02, 0.35) * if a.light { 0.5 } else { 0.55 }, if a.light { 0.955 } else { 0.055 });
    ([tune_accent(a.accent, a.light), a.second, bg], a.light)
}

/// The theme's current accent as an Rgb (for orbs that follow the live palette).
pub fn rgb_of(c: Hsla) -> Rgb {
    let r = Rgba::from(c);
    [r.r * 255., r.g * 255., r.b * 255.]
}

/// Set the palette / mode the way the mock's picker does, then animate.
pub fn set_pal(cx: &mut App, pal: &str) {
    let ts = cx.global_mut::<ThemeState>();
    ts.pal = pal.to_string();
    ts.apply();
}
pub fn set_mode(cx: &mut App, light: bool) {
    let ts = cx.global_mut::<ThemeState>();
    if ts.pal == "auto" {
        ts.pal = "ember".into();
    }
    ts.light = light;
    ts.apply();
}
/// Whether the effective appearance is light (Auto decides from the backdrop).
pub fn is_light(ts: &ThemeState) -> bool {
    if ts.pal == "auto" { ts.auto.map(|a| a.light).unwrap_or(false) } else { ts.light }
}

impl TakoApp {
    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_theme_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let st = &mut self.overlays.theme;
        let vis = pop_progress(st.open, &mut st.shown, &mut st.tween, 220., window)?;
        let t = cx.t();
        let vw = f32::from(window.viewport_size().width);
        // place(): top = btn.bottom + 8, right = innerWidth - btn.right - 4
        let anchor = st.anchor.get();
        let (top, right) = match anchor {
            Some(b) => (f32::from(b.bottom()) + 8., (vw - f32::from(b.right()) - 4.).max(8.)),
            None => (44., 70.),
        };
        let ts = cx.global::<ThemeState>();
        let pal = ts.pal.clone();
        let mode_light = ts.light;
        let ([oa, ob, oc], auto_light) = auto_orb_cols(ts);
        let auto_on = pal == "auto";
        let light_on = is_light(ts);

        let auto_row = div()
            .id("tp-auto")
            .w_full()
            .flex()
            .items_center()
            .gap(px(11.))
            .p(px(10.))
            .rounded(px(10.))
            .border_1()
            .when(auto_on, |d| d.border_color(alpha(t.accent, 0.6)).bg(alpha(t.accent, 0.09)))
            .when(!auto_on, |d| d.border_color(t.border).bg(t.surface).hover(|s| s.bg(t.surface_hover)))
            .child(
                div()
                    .size(px(34.))
                    .flex_none()
                    .rounded_full()
                    .shadow(vec![BoxShadow { color: rgb_a(oa, 0.45), offset: point(px(0.), px(0.)), blur_radius: px(18.), spread_radius: px(0.), inset: false }])
                    .child(orb(Grad::Conic { from: 210. }, &[oa, ob, oc, oa], 34., 0.15)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .min_w_0()
                    .flex_1()
                    .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(t.text).child("Auto"))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .line_height(px(15.5))
                            .text_color(t.text_3)
                            .child(format!("Picks colours from the backdrop behind the composer · {}", if auto_light { "Light" } else { "Dark" })),
                    ),
            )
            .child(div().when(!auto_on, |d| d.opacity(0.)).child(icon("check", 15., t.accent)))
            .on_click(|_, _, cx| set_pal(cx, "auto"));

        let grid = div().grid().grid_cols(3).gap(px(6.)).my(px(8.)).children(PRESETS.iter().map(|p| {
            let on = pal == p.id;
            let c = hsl_rgb(p.tint.0, p.tint.1 * 0.55, if mode_light { 0.95 } else { 0.07 });
            let id = p.id;
            div()
                .id(SharedString::from(format!("tp-sw-{}", p.id)))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .pt(px(9.))
                .pb(px(7.))
                .px(px(4.))
                .rounded(px(10.))
                .border_1()
                .text_size(px(11.5))
                .when(on, |d| d.border_color(alpha(t.accent, 0.55)).text_color(t.text).bg(t.surface))
                .when(!on, |d| d.border_color(transparent_black()).text_color(t.text_2).hover(|s| s.bg(t.surface_hover).text_color(t.text)))
                .child(orb(Grad::Bands { s0: 0.45, s1: 0.70 }, &[hex(p.accent), hex(p.second), c], 26., 0.14))
                .child(p.name)
                .on_click(move |_, _, cx| set_pal(cx, id))
        }));

        let seg_btn = |id: &'static str, ic: &'static str, label: &'static str, on: bool, light: bool| {
            div()
                .id(id)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .p(px(6.))
                .rounded(px(7.))
                .text_size(px(12.5))
                .when(auto_on && !on, |d| d.opacity(0.7))
                .when(on, |d| d.bg(t.surface_active).text_color(t.text))
                .when(!on, |d| d.text_color(t.text_3))
                .child(icon(ic, 14., if on { t.text } else { t.text_3 }))
                .child(label)
                .on_click(move |_, _, cx| set_mode(cx, light))
        };

        let palette_btn = anchor;
        let el = div()
            .id("theme-pop")
            .occlude()
            .absolute()
            .top(px(top - 6. * (1. - vis)))
            .right(px(right))
            // the mock's scale(.98) → 1, approximated by growing the box from its top-right anchor
            .w(px(296. * (0.98 + 0.02 * vis)))
            .p(px(10.))
            .rounded(px(14.))
            .bg(t.panel_solid) // flat style: solid popovers
            .border_1()
            .border_color(t.border)
            .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.18), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
            .opacity(vis)
            .on_mouse_down_out(cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                // Leave clicks on the palette button to its own toggle.
                let vw = f32::from(window.viewport_size().width);
                let btn = palette_btn.unwrap_or(Bounds::new(point(px(vw - 102.), px(8.)), size(px(28.), px(28.))));
                if !btn.contains(&ev.position) {
                    this.overlays.theme.open = false;
                    cx.notify();
                }
            }))
            .child(escape_closer(cx.entity().downgrade(), |a| a.overlays.theme.open = false))
            .child(div().text_size(px(11.5)).text_color(t.text_3).pt(px(2.)).px(px(4.)).pb(px(8.)).child("THEME"))
            .child(auto_row)
            .child(grid)
            .child(div().text_size(px(11.5)).text_color(t.text_3).pt(px(10.)).px(px(4.)).pb(px(6.)).child("Appearance"))
            .child(
                div()
                    .flex()
                    .gap(px(2.))
                    .p(px(3.))
                    .rounded(px(9.))
                    .bg(t.surface)
                    .child(seg_btn("tp-dark", "moon", "Dark", !light_on, false))
                    .child(seg_btn("tp-light", "sun", "Light", light_on, true)),
            );
        Some(el.into_any_element())
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// `pal:<id|auto>` picks a palette, `mode:dark|light` the appearance.
pub fn act(_app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "pal" => set_pal(cx, v),
        "mode" => set_mode(cx, v == "light"),
        _ => return false,
    }
    true
}
