//! Emulator tab: a generic device frame running the session's app
//! (the mock's `VIEWS.emulator`).

use super::{hex, ib, seg, st, tr};
use crate::{app::TakoApp, chat::Tab, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::time::{Duration, Instant};

pub struct EmuState {
    pub tablet: bool,
    pub land: bool,
    /// size/radius transition (.5s): from (w, h, r)
    pub anim: Option<((f32, f32, f32), Tween)>,
    pub boot: Instant,
}
impl Default for EmuState {
    fn default() -> Self {
        EmuState { tablet: false, land: false, anim: None, boot: Instant::now() }
    }
}
impl EmuState {
    fn dims(&self) -> (f32, f32, f32) {
        let (w, h, r) = if self.tablet { (470., 620., 28.) } else { (270., 560., 42.) };
        if self.land { (h, w, r) } else { (w, h, r) }
    }
    fn current(&self) -> (f32, f32, f32) {
        let to = self.dims();
        match &self.anim {
            Some((from, tw)) => {
                let e = ease_snappy(tw.raw());
                (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e, from.2 + (to.2 - from.2) * e)
            }
            None => to,
        }
    }
    fn change(&mut self, f: impl FnOnce(&mut Self)) {
        let from = self.current();
        f(self);
        self.anim = Some((from, Tween::new(500.)));
    }
}

pub fn emu_act(id: usize, v: &str) {
    st(|s| {
        let e = s.emu.entry(id).or_default();
        match v {
            "phone" => e.change(|e| e.tablet = false),
            "tablet" => e.change(|e| e.tablet = true),
            "rotate" => e.change(|e| e.land = !e.land),
            "reload" => e.boot = Instant::now(),
            _ => {}
        }
    })
}

impl TakoApp {
    pub(super) fn render_emulator(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        let (tablet, (w, h, r), boot_ms, animating) = st(|s| {
            let e = s.emu.entry(id).or_default();
            if e.anim.as_ref().is_some_and(|(_, tw)| tw.done()) {
                e.anim = None;
            }
            (e.tablet, e.current(), e.boot.elapsed().as_secs_f32() * 1000., e.anim.is_some())
        });
        if animating || boot_ms < 1200. {
            window.request_animation_frame();
        }
        // `.emu-dev.boot .emu-screen > *` / `.app-card` rise animations
        let rise = |delay: f32, dur: f32| {
            let p = ease_snappy(((boot_ms - delay) / dur).clamp(0., 1.));
            (p, 16. * (1. - p))
        };

        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .px(px(12.))
            .py(px(7.))
            .border_b_1()
            .border_color(t.border)
            .child(seg("emu-dev", &[("Phone", None), ("Tablet", None)], tablet as usize, t, move |i, b| {
                b.on_click(move |_, window, _| {
                    emu_act(id, if i == 0 { "phone" } else { "tablet" });
                    window.refresh();
                })
            }))
            .child(ib("emu-rot", "rotate", t).on_click(move |_, window, _| {
                emu_act(id, "rotate");
                window.refresh();
            }))
            .child(ib("emu-reload", "reload", t).on_click(move |_, window, _| {
                emu_act(id, "reload");
                window.refresh();
            }))
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(t.text_3)
                    .child(div().size(px(6.)).rounded_full().bg(t.green).shadow(vec![BoxShadow {
                        color: t.green,
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(8.),
                        spread_radius: px(0.),
                        inset: false,
                    }]))
                    .child("Hot reload on · Tako (debug)"),
            );

        let ink = hex(0x1d1b1a);
        let titles = ["Background agents panel", "Flaky auth test on CI", "Retry on transient 403", "Inline images"];
        let wrap = |el: Div, delay: f32, dur: f32| {
            let (o, dy) = rise(delay, dur);
            el.opacity(o).relative().top(px(dy))
        };
        let screen = div()
            .size_full()
            .rounded(px(if tablet { 20. } else { (r - 9.).max(20.) }))
            .overflow_hidden()
            .bg(hex(0xf7f5f2))
            .text_color(ink)
            .text_size(px(12.5))
            .line_height(relative(1.25))
            .flex()
            .flex_col()
            .child(wrap(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .pt(px(12.))
                    .px(px(22.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("9:41")
                    .child(div().size(px(9.)).rounded_full().bg(hex(0x111111)))
                    .child("●●● ▮"),
                0.,
                600.,
            ))
            .child(wrap(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .pt(px(14.))
                    .px(px(18.))
                    .pb(px(10.))
                    .child(div().text_size(px(22.)).font_weight(FontWeight::BOLD).child("Agents"))
                    .child(div().text_size(px(11.5)).text_color(hex(0x888888)).child("3 running")),
                0.,
                600.,
            ))
            .children(titles.iter().enumerate().map(|(i, title)| {
                let card = div()
                    .mx(px(12.))
                    .mb(px(8.))
                    .px(px(14.))
                    .py(px(12.))
                    .rounded(px(14.))
                    .bg(white())
                    .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.06), offset: point(px(0.), px(1.)), blur_radius: px(3.), spread_radius: px(0.), inset: false }])
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().font_weight(FontWeight::BOLD).child(*title))
                                    .child(div().text_size(px(11.)).text_color(hex(0x999999)).child(if i < 3 { format!("step {} of 7", 3 + i) } else { "Ready for review".into() })),
                            )
                            .when(i == 3, |d| d.child(div().text_color(hex(0x3a9a5a)).font_weight(FontWeight::SEMIBOLD).child("✓"))),
                    )
                    .when(i < 3, |d| {
                        d.child(div().h(px(4.)).rounded(px(4.)).bg(hex(0xeeeeee)).overflow_hidden().child(div().h_full().rounded(px(4.)).bg(t.accent).with_animation(
                            ("emu-bar", i),
                            Animation::new(Duration::from_millis(5200)).repeat(),
                            move |d, p| {
                                // 2.6s ease-in-out alternate, delay -(i*.8)s
                                let q = (p + i as f32 * 0.8 / 5.2) % 1.;
                                let tri = if q < 0.5 { q * 2. } else { 2. - q * 2. };
                                let e = 0.5 - 0.5 * (std::f32::consts::PI * tri).cos();
                                d.w(relative(0.18 + 0.7 * e))
                            },
                        )))
                    });
                wrap(card, 100. + 60. * i as f32, 500.)
            }))
            .child(div().flex_1())
            .child(wrap(
                div()
                    .flex()
                    .justify_around()
                    .pt(px(10.))
                    .pb(px(16.))
                    .border_t_1()
                    .border_color(hex(0xeeeeee))
                    .text_color(hex(0xbbbbbb))
                    .child(div().text_color(t.accent).child("●"))
                    .child("○")
                    .child("○"),
                0.,
                600.,
            ));

        let device = div()
            .w(px(w))
            .h(px(h))
            .flex_none()
            .p(px(10.))
            .rounded(px(r))
            .bg(linear_gradient(145., linear_color_stop(hex(0x3a3639), 0.), linear_color_stop(hex(0x121013), 1.)))
            .shadow(vec![
                BoxShadow { color: hsla(0., 0., 1., 0.08), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(1.), inset: false },
                BoxShadow { color: hsla(0., 0., 0., 0.45), offset: point(px(0.), px(30.)), blur_radius: px(70.), spread_radius: px(0.), inset: false },
            ])
            .child(screen);

        // radial-gradient(600px 400px at 50% 40%, accent 9%, transparent 70%), drawn as soft rings
        let glow = canvas(
            |_, _, _| (),
            move |b, _, window, _| {
                let c = b.center();
                let cy = b.origin.y + b.size.height * 0.4;
                let n = 24;
                for k in 0..n {
                    let f = 1. - k as f32 / n as f32; // 1 → small
                    let (rw, rh) = (600. * 0.7 * f, 400. * 0.7 * f);
                    let col = tr(t.accent, 0.09 / n as f32 * 1.6);
                    window.paint_quad(
                        fill(Bounds::new(point(c.x - px(rw), cy - px(rh)), size(px(rw * 2.), px(rh * 2.))), col).corner_radii(px(rw.min(rh))),
                    );
                }
            },
        )
        .absolute()
        .size_full();

        div().flex_1().min_w_0().flex().flex_col().child(bar).child(
            div()
                .id(("emu-stage", id))
                .flex_1()
                .min_h_0()
                .relative()
                .overflow_scroll()
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .child(glow)
                .child(device),
        )
    }
}
