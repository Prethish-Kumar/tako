//! Profile menu, sign-out overlay (port of the mock's PROFILE section).
use crate::{
    app::{TakoApp, View},
    pages::stats::{dur, fmt_int, mix, totals, ACCTS},
    theme::ActiveTheme,
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::time::{Duration, Instant};

/// Art-pixel size of the signed-out scene (the mock draws at 1/4 resolution).
const S: f32 = 4.;

#[derive(Clone, Copy)]
struct Bubble {
    x: f32,
    y: f32,
    a: f32,
    z: bool,
}

pub struct SignOut {
    t0: Instant,
    last: Instant,
    /// fade in (leaving = false) or out (leaving = true), 500 ms
    fade: Tween,
    leaving: bool,
    bubbles: Vec<Bubble>,
    seed: u32,
}

pub struct ProfileState {
    pub open: bool,
    /// bumps on every open so the entrance animation replays
    opened: Option<Instant>,
    was_open: bool,
    pub signed_out: Option<SignOut>,
    _keys: Subscription,
}

impl ProfileState {
    pub fn new(_window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        // Escape closes the menu wherever focus is.
        let keys = cx.observe_keystrokes(|this, ev, _, cx| {
            if ev.keystroke.key == "escape" && this.overlays.profile.open {
                this.overlays.profile.open = false;
                cx.notify();
            }
        });
        ProfileState { open: false, opened: None, was_open: false, signed_out: None, _keys: keys }
    }
}

impl TakoApp {
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.overlays.profile.open = false;
        self.overlays.profile.signed_out = Some(SignOut { t0: now, last: now, fade: Tween::new(500.), leaving: false, bubbles: vec![], seed: 0x9e3779b9 });
        cx.notify();
    }

    pub fn sign_in(&mut self, cx: &mut Context<Self>) {
        if let Some(so) = &mut self.overlays.profile.signed_out {
            if !so.leaving {
                // reverse from wherever the fade-in currently is
                let p = so.fade.raw();
                so.fade = Tween::new(500.);
                so.fade.start -= Duration::from_secs_f32((1. - p) * 0.5);
                so.leaving = true;
            }
        }
        cx.notify();
    }

    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pf = &mut self.overlays.profile;
        if pf.open && !pf.was_open {
            pf.opened = Some(Instant::now());
        }
        pf.was_open = pf.open;
        let pop = self.overlays.profile.open.then(|| self.render_pf_pop(window, cx).into_any_element());
        let so = self.render_signed_out(window, cx);
        if pop.is_none() && so.is_none() {
            return None;
        }
        Some(div().absolute().top_0().left_0().size_full().children(pop).children(so).into_any_element())
    }

    fn render_pf_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let tt = totals();
        let mini = |id: &'static str, b: String, small: &'static str| {
            div()
                .id(id)
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .px(px(9.))
                .py(px(8.))
                .rounded(px(9.))
                .bg(t.surface)
                .border_1()
                .border_color(t.border)
                .hover(|s| s.bg(t.surface_hover).border_color(mix(t.accent, 0.35, t.border)))
                .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).font_features(crate::pages::stats::tnum()).child(b))
                .child(div().mt(px(1.)).text_size(px(11.)).text_color(t.text_3).whitespace_nowrap().child(small))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.overlays.profile.open = false;
                    this.open_stats(cx);
                }))
        };
        let item = |id: &'static str, ic: &'static str, label: &'static str, out: bool| {
            let col = if out { t.red } else { t.text_2 };
            div()
                .id(id)
                .w_full()
                .h(px(32.))
                .px(px(9.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .gap(px(10.))
                .text_size(px(13.))
                .text_color(col)
                .when(!out, |d| d.hover(|s| s.bg(t.surface_hover).text_color(t.text)))
                .when(out, |d| d.hover(|s| s.bg(mix(t.red, 0.1, transparent_black()))))
                .child(icon(ic, 15., col))
                .child(label)
        };
        // .pf-pop: left = avatar.right + 10, bottom = avatar.bottom + 2
        // (avatar: 30px, centred in the 52px rail, 10px above the window bottom)
        let left = 52. / 2. + 15. + 10.;
        let bottom = 10. - 2.;        // entrance: opacity .16s ease, transform .22s cubic-bezier(.2,.8,.2,1)
        // (offscreen snapshots draw a single frame, so they skip it)
        let ms = self.overlays.profile.opened.map(|o| o.elapsed().as_secs_f32() * 1000.).unwrap_or(1e9);
        let p = if snap_mode() { 1. } else { (ms / 220.).min(1.) };
        if p < 1. {
            window.request_animation_frame();
        }
        let (op, e) = ((p * 220. / 160.).min(1.), ease_snappy(p));
        div()
            .id("pf-pop")
            .occlude()
            .absolute()
            .left(px(left - 6. * (1. - e)))
            .bottom(px(bottom))
            .opacity(op)
            .w(px(286.))
            .p(px(8.))
            .rounded(px(14.))
            .bg(alpha(t.panel_solid, 0.94))
            .border_1()
            .border_color(t.border)
            .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.35), offset: point(px(0.), px(24.)), blur_radius: px(60.), spread_radius: px(0.), inset: false }])
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(8.))
                    .pt(px(8.))
                    .pb(px(10.))
                    .child(
                        div()
                            .size(px(36.))
                            .flex_none()
                            .rounded_full()
                            .bg(t.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(white())
                            .child("P"),
                    )
                    .child(
                        div()
                            .child(div().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child("Your account"))
                            .child(div().mt(px(1.)).text_size(px(12.)).text_color(t.text_3).child(format!("{} AI accounts · 2 machines, 1 phone", ACCTS.len()))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .mx(px(2.))
                    .mb(px(6.))
                    .child(mini("pf-agents", fmt_int(tt.agents), "Agents"))
                    .child(mini("pf-time", dur(tt.hours), "Agent time"))
                    .child(mini("pf-prs", format!("{}", tt.prs), "PRs")),
            )
            .child(item("pf-stats", "chart", "Stats", false).on_click(cx.listener(|this, _, _, cx| {
                this.overlays.profile.open = false;
                this.open_stats(cx);
            })))
            .child(
                item("pf-settings", "gear", "Settings", false)
                    .child(div().ml_auto().font_family(MONO).text_size(px(11.)).text_color(t.text_3).child("⌘,"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overlays.profile.open = false;
                        this.go(View::Settings, cx);
                    })),
            )
            .child(item("pf-intro", "play", "Replay intro", false).on_click(cx.listener(|this, _, window, cx| {
                this.overlays.profile.open = false;
                this.open_onboarding(window, cx);
            })))
            .child(div().h(px(1.)).bg(t.border).mx(px(4.)).my(px(6.)))
            .child(item("pf-logout", "logout", "Log out", true).on_click(cx.listener(|this, _, _, cx| this.sign_out(cx))))
            .on_mouse_down_out(cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                // clicks on the avatar toggle the menu themselves
                let vh = f32::from(window.viewport_size().height);
                let (x, y) = (f32::from(ev.position.x), f32::from(ev.position.y));
                let on_avatar = (11. ..=41.).contains(&x) && (vh - 40. ..=vh - 10.).contains(&y);
                if !on_avatar {
                    this.overlays.profile.open = false;
                    cx.notify();
                }
            }))
    }

    fn render_signed_out(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let so = self.overlays.profile.signed_out.as_mut()?;
        let now = Instant::now();
        if so.leaving && so.fade.done() {
            self.overlays.profile.signed_out = None;
            return None;
        }
        let vp = window.viewport_size();
        let (w, h) = ((f32::from(vp.width) / S).ceil(), (f32::from(vp.height) / S).ceil());
        let t = so.t0.elapsed().as_secs_f32();
        // sleeping Tako's mantle centre (art px)
        let (ox, oy) = (w / 2., h * 0.5 - 52. + (t * 1.1).sin() * 2.);
        // Zz bubbles: the mock steps at 60 fps; step by elapsed frames
        // (capped: the mock pauses while hidden instead of jumping)
        let frames = (now.duration_since(so.last).as_secs_f32() * 60.).min(4.);
        so.last = now;
        let mut rnd = || {
            so.seed ^= so.seed << 13;
            so.seed ^= so.seed >> 17;
            so.seed ^= so.seed << 5;
            so.seed as f32 / u32::MAX as f32
        };
        let mut f = frames;
        let mut spawned = vec![];
        while f > 0. {
            let k = f.min(1.);
            if rnd() < 0.025 * k {
                spawned.push(Bubble { x: ox + 12., y: oy - 18., a: 0., z: rnd() < 0.5 });
            }
            f -= 1.;
        }
        so.bubbles.extend(spawned);
        for b in &mut so.bubbles {
            b.a += frames / 60.;
            b.y -= 0.25 * frames;
            b.x += (b.a * 3.).sin() * 0.3 * frames;
        }
        so.bubbles.retain(|b| b.a <= 4.);
        let bubbles = so.bubbles.clone();
        let fp = ease_out(so.fade.raw());
        let opacity = if so.leaving { 1. - fp } else { fp };
        window.request_animation_frame();

        let scene = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| paint_deep_sea(window, bounds, w, h, t, ox, oy, &bubbles),
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let gh = |id: &'static str| div().id(id).h(px(40.)).px(px(18.)).rounded(px(10.)).text_size(px(14.)).font_weight(FontWeight::MEDIUM).flex().items_center().gap(px(9.));
        let card = div()
            .relative()
            .mt(px(150.))
            .flex()
            .flex_col()
            .items_center()
            .text_color(rgb(0xe6efff))
            .child(div().text_size(px(30.)).font_weight(FontWeight::SEMIBOLD).child("See you soon."))
            .child(div().mt(px(8.)).mb(px(22.)).text_color(rgb(0x9db3e0)).child("You're signed out. Tako is taking a nap."))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .justify_center()
                    .child(
                        gh("so-github")
                            .bg(rgb(0x9cc8ff))
                            .text_color(rgb(0x06163a))
                            .hover(|s| s.bg(rgb(0xa8d8ff)))
                            .child(icon("github", 16., rgb(0x06163a).into()))
                            .child("Sign in with GitHub")
                            .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                    )
                    .child(
                        gh("so-email")
                            .border_1()
                            .border_color(rgba(0x9cc8ff4d))
                            .text_color(rgb(0xcfe0ff))
                            .hover(|s| s.bg(rgba(0x9cc8ff14)))
                            .child("Use email instead")
                            .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                    ),
            )
            .child(div().mt(px(18.)).text_size(px(12.)).text_color(rgb(0x6f86b8)).child("Your worktrees and chats stay on this Mac."));

        Some(
            div()
                .id("signed-out")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .opacity(opacity)
                .bg(rgb(0x040c26))
                .flex()
                .items_center()
                .justify_center()
                .child(scene)
                .child(card)
                .into_any_element(),
        )
    }
}

fn snap_mode() -> bool {
    std::env::var_os("TAKO_SNAP").is_some()
}

fn alpha(c: Hsla, a: f32) -> Hsla {
    hsla(c.h, c.s, c.l, c.a * a)
}

fn hexa(v: u32, a: f32) -> Hsla {
    let mut c: Hsla = rgb(v).into();
    c.a = a;
    c
}

/// The signed-out deep-sea scene, drawn in art pixels (S screen px each).
#[allow(clippy::too_many_arguments)]
fn paint_deep_sea(window: &mut Window, b: Bounds<Pixels>, w: f32, h: f32, t: f32, ox: f32, oy: f32, bubbles: &[Bubble]) {
    let o = b.origin;
    let at = |x: f32, y: f32| point(o.x + px(x * S), o.y + px(y * S));
    let rect = |window: &mut Window, x: f32, y: f32, rw: f32, rh: f32, c: Hsla| {
        window.paint_quad(fill(Bounds { origin: at(x, y), size: size(px(rw * S), px(rh * S)) }, c));
    };
    // water: #0a2a6e → #061a4a (55%) → #030a22
    let split = h * 0.55;
    window.paint_quad(fill(
        Bounds { origin: at(0., 0.), size: size(px(w * S), px(split * S)) },
        linear_gradient(180., linear_color_stop(rgb(0x0a2a6e), 0.), linear_color_stop(rgb(0x061a4a), 1.)),
    ));
    window.paint_quad(fill(
        Bounds { origin: at(0., split), size: size(px(w * S), px((h - split) * S)) },
        linear_gradient(180., linear_color_stop(rgb(0x061a4a), 0.), linear_color_stop(rgb(0x030a22), 1.)),
    ));
    // light rays
    let ray = hexa(0x9cc8ff, 0.07);
    for k in 0..5 {
        let kf = k as f32;
        let x = w * (0.15 + kf * 0.19) + (t * 0.3 + kf).sin() * 6.;
        let mut pb = PathBuilder::fill();
        pb.move_to(at(x, 0.));
        pb.line_to(at(x + 10., 0.));
        pb.line_to(at(x + 40. + kf * 4., h));
        pb.line_to(at(x + 18., h));
        pb.close();
        if let Ok(p) = pb.build() {
            window.paint_path(p, ray);
        }
    }
    // plankton
    for k in 0..60 {
        let kf = k as f32;
        let x = (kf * 97.3 + t * (1. + (k % 3) as f32)).rem_euclid(w);
        let y = (kf * 53.7 + (t * 0.5 + kf).sin() * 4.).rem_euclid(h);
        let c = if k % 4 != 0 { hexa(0x8cbeff, 0.35) } else { hexa(0xb4f0ff, 0.7) };
        rect(window, x.floor(), y.floor(), 1., 1., c);
    }
    // sand
    let sand: Hsla = rgb(0x0b2156).into();
    for x in 0..w as i32 {
        let xf = x as f32;
        let y = (h - 10. - (xf * 0.05).sin() * 3. - (xf * 0.13).sin() * 1.5).floor();
        rect(window, xf, y, 1., h - y, sand);
    }
    // TODO(octo): replace this placeholder with the shared renderer once
    // src/scene/octo.rs lands, i.e. the mock's
    //   TakoOcto.draw(g, { x: ox, y: oy, t: t * .6, lid: 1, pose: "float", ripple: .2 + .2 * sin(t * .8) })
    // → crate::scene::octo::draw_octo(window, origin, S, ox, oy, OctoPose { t: t * 0.6, lid: 1., float, ripple, .. })
    paint_octo_placeholder(window, &rect, ox, oy, t * 0.6);
    // Zz bubbles
    for bb in bubbles {
        let a = (1. - bb.a / 4.).max(0.);
        if bb.z {
            let c = hexa(0xcfe3ff, a);
            let (x, y) = (bb.x.floor(), bb.y.floor());
            rect(window, x, y, 4., 1., c);
            rect(window, x + 2., y + 1., 1., 1., c);
            rect(window, x + 1., y + 2., 1., 1., c);
            rect(window, x, y + 3., 4., 1., c);
        } else {
            let r = (1.6 + bb.a * 0.4) * S;
            let c = at(bb.x, bb.y);
            window.paint_quad(
                quad(Bounds { origin: point(c.x - px(r), c.y - px(r)), size: size(px(2. * r), px(2. * r)) }, px(r), transparent_black(), px(S), hexa(0x9cc8ff, a), BorderStyle::Solid),
            );
        }
    }
}

/// A simple sleeping pixel octopus (mantle ellipse, closed eyes, eight arms).
fn paint_octo_placeholder(window: &mut Window, rect: &dyn Fn(&mut Window, f32, f32, f32, f32, Hsla), ox: f32, oy: f32, t: f32) {
    let (out, d, m, l): (Hsla, Hsla, Hsla, Hsla) = (rgb(0x0b1a4f).into(), rgb(0x1c3b9a).into(), rgb(0x2f5fd6).into(), rgb(0x4f86f2).into());
    // arms
    for i in 0..8 {
        let k = i as f32 - 3.5;
        for s in 0..16 {
            let u = s as f32 / 15.;
            let x = ox + k * 3.1 + k * 6. * u + (t * 1.1 + i as f32 * 0.8 + u * 3.).sin() * 2. * u;
            let y = oy + 10. + u * (24. - k.abs() * 3.);
            let r = (2.2 * (1. - u) + 0.8).round();
            rect(window, (x - r).round(), y.round(), 2. * r, 2., if u > 0.8 { d } else { m });
        }
    }
    // mantle
    let (rx, ryt, ryb) = (14.5f32, 18.5f32, 13.5f32);
    for j in -19..15 {
        let jf = j as f32 + 0.5;
        let ry = if jf < 0. { ryt } else { ryb };
        let q = 1. - (jf / ry).powi(2);
        if q <= 0. {
            continue;
        }
        let hw = (rx * q.sqrt()).round();
        rect(window, ox - hw - 1., oy + j as f32, 2. * hw + 2., 1., out);
        rect(window, ox - hw, oy + j as f32, 2. * hw, 1., if jf < -8. { l } else { m });
    }
    // closed eyes
    for ex in [-6., 4.] {
        rect(window, ox + ex, oy + 2., 3., 1., out);
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
///   logout   sign out (deep-sea overlay)    login   sign back in
pub fn act(app: &mut TakoApp, k: &str, _v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "logout" => app.sign_out(cx),
        "login" => app.sign_in(cx),
        _ => return false,
    }
    true
}
