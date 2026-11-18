//! Browser tab: URL bar with history, a loading bar, and the session's app
//! preview. GPUI has no web view, so the mock's srcdoc page is drawn natively.

use super::{hex, ib, mix, seg, st};
use crate::{app::TakoApp, chat::Tab, theme::{ActiveTheme, Theme}, ui::*};
use gpui_kit::{
    component::input::{Input, InputEvent, InputState},
    prelude::FluentBuilder as _,
    *,
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub struct BrowserState {
    pub input: Entity<InputState>,
    pub hist: Vec<String>,
    pub at: usize,
    /// What the stage shows; swaps to `hist[at]` 220ms into a load.
    pub page: String,
    pub load: Option<Tween>,
    pub mobile: bool,
    /// width transition (.35s): from-px + tween
    pub width_anim: Option<(f32, Tween)>,
    pub stage_w: Rc<Cell<f32>>,
}

impl TakoApp {
    pub(super) fn ensure_browser(&mut self, id: usize, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if st(|s| s.browser.contains_key(&id)) {
            return;
        }
        let url = if url.is_empty() || url == "Browser" { "localhost:3000" } else { url }.to_string();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |s, cx| s.set_value(url.clone(), window, cx));
        cx.subscribe_in(&input, window, move |this: &mut TakoApp, inp, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                let v = inp.read(cx).value().to_string();
                this.browser_go(id, v, true, window, cx);
                window.blur(cx);
            }
        })
        .detach();
        st(|s| {
            s.browser.insert(
                id,
                BrowserState { input, hist: vec![url.clone()], at: 0, page: url, load: Some(Tween::new(700.)), mobile: false, width_anim: None, stage_w: Rc::default() },
            )
        });
    }

    pub(super) fn browser_go(&mut self, id: usize, url: String, push: bool, window: &mut Window, cx: &mut Context<Self>) {
        let url = if url.trim().is_empty() { "localhost:3000".to_string() } else { url.trim().to_string() };
        let input = st(|s| {
            let b = s.browser.get_mut(&id)?;
            if push {
                b.hist.truncate(b.at + 1);
                b.hist.push(url.clone());
                b.at = b.hist.len() - 1;
            }
            b.load = Some(Tween::new(700.));
            Some(b.input.clone())
        });
        if let Some(input) = input {
            input.update(cx, |s, cx| s.set_value(url.clone(), window, cx));
        }
        self.rename_tab(id, url);
        cx.notify();
    }

    pub(super) fn render_browser(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        self.ensure_browser(id, &tab.name, window, cx);
        // advance the load: page swap at 220ms, bar fades by 700ms
        let (input, page, load_p, mobile, width_anim, stage_w) = st(|s| {
            let b = s.browser.get_mut(&id).unwrap();
            let mut lp = None;
            if let Some(l) = &b.load {
                let ms = l.raw() * 700.;
                if ms >= 220. {
                    b.page = b.hist[b.at].clone();
                }
                if l.done() {
                    b.load = None;
                } else {
                    lp = Some(l.raw());
                }
            }
            let wa = b.width_anim.as_ref().map(|(f, tw)| (*f, tw.raw()));
            if b.width_anim.as_ref().is_some_and(|(_, tw)| tw.done()) {
                b.width_anim = None;
            }
            (b.input.clone(), b.page.clone(), lp, b.mobile, wa, b.stage_w.clone())
        });
        if load_p.is_some() || width_anim.is_some() {
            window.request_animation_frame();
        }
        let nav = move |this: &mut TakoApp, d: isize, window: &mut Window, cx: &mut Context<TakoApp>| {
            let url = st(|s| {
                let b = s.browser.get_mut(&id)?;
                let to = b.at as isize + d;
                if d != 0 && (to < 0 || to >= b.hist.len() as isize) {
                    return None;
                }
                b.at = to as usize;
                Some(b.hist[b.at].clone())
            });
            if let Some(u) = url {
                this.browser_go(id, u, false, window, cx);
            }
        };

        // stage width: 100% or 420px, eased over .35s
        let full = stage_w.get();
        let target = if mobile { 420f32.min(full.max(1.)) } else { full };
        let w = match width_anim {
            Some((from, p)) if full > 0. => from + (target - from) * ease_snappy(p),
            _ => target,
        };
        let set_mobile = move |m: bool, sw: Rc<Cell<f32>>| {
            st(|s| {
                let b = s.browser.get_mut(&id).unwrap();
                if b.mobile != m {
                    let full = sw.get();
                    let cur = if b.mobile { 420f32.min(full) } else { full };
                    b.mobile = m;
                    b.width_anim = Some((cur, Tween::new(350.)));
                }
            })
        };

        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.))
            .px(px(10.))
            .py(px(7.))
            .border_b_1()
            .border_color(t.border)
            .child(ib("bz-back", "left", t).on_click(cx.listener(move |this, _, w, cx| nav(this, -1, w, cx))))
            .child(ib("bz-fwd", "right", t).on_click(cx.listener(move |this, _, w, cx| nav(this, 1, w, cx))))
            .child(ib("bz-reload", "reload", t).on_click(cx.listener(move |this, _, w, cx| nav(this, 0, w, cx))))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(30.))
                    .px(px(12.))
                    .mx(px(6.))
                    .rounded(px(8.))
                    .bg(t.surface)
                    .child(icon("lock", 13., t.text_3))
                    .child(div().flex_1().text_size(px(13.)).text_color(t.text).child(Input::new(&input).appearance(false).bordered(false).p_0().h(px(20.)))),
            )
            .child({
                let sw = stage_w.clone();
                seg("bz-w", &[("", Some("globe")), ("", Some("phone"))], mobile as usize, t, move |i, b| {
                    let sw = sw.clone();
                    b.on_click(move |_, window, _| {
                        set_mobile(i == 1, sw.clone());
                        window.refresh();
                    })
                })
            })
            .child(ib("bz-ext", "ext", t));

        // loading bar: scaleX 0→1 over 80%, then fades
        let load = div().h(px(2.)).w_full().flex_none().relative().when_some(load_p, |d, p| {
            let e = ease_snappy((p / 0.8).min(1.));
            let op = if p < 0.8 { 1. } else { 1. - (p - 0.8) / 0.2 };
            d.child(div().absolute().top_0().left_0().h_full().w(relative(e)).bg(t.accent).opacity(op))
        });

        let sw = stage_w.clone();
        let stage = div()
            .flex_1()
            .min_h_0()
            .relative()
            .flex()
            .justify_center()
            .bg(mix(t.text, t.bg, 0.04))
            .child(canvas(move |b, _, _| sw.set(f32::from(b.size.width)), |_, _, _, _| {}).absolute().size_full())
            .child(
                div()
                    .id(("bz-page", id))
                    .h_full()
                    .when(full > 0., |d| d.w(px(w)))
                    .when(full <= 0., |d| d.w_full())
                    .overflow_y_scroll()
                    .bg(white())
                    .child(preview_page(&page, w, t)),
            );

        div().flex_1().min_w_0().flex().flex_col().child(bar).child(load).child(stage)
    }
}

/// The mock's srcdoc page, drawn with divs.
fn preview_page(url: &str, width: f32, t: Theme) -> AnyElement {
    let local = url.starts_with("localhost") || url.starts_with("127.");
    let ink = hex(0x1d1b1a);
    let base = div().size_full().min_h_full().bg(hex(0xfbfaf8)).text_color(ink).font_family(FONT).text_size(px(14.)).line_height(relative(1.5));
    if !local {
        return base
            .flex()
            .items_center()
            .justify_center()
            .text_color(hex(0x888888))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(div().text_size(px(18.)).font_weight(FontWeight::BOLD).text_color(hex(0x333333)).mb(px(6.)).child(url.to_string()))
                    .child("Live pages load here in the real app.")
                    .child("This mock previews localhost only."),
            )
            .into_any_element();
    }
    let a = t.accent;
    let cards = [("Background agents panel", 0usize), ("Flaky auth test on CI", 1), ("Retry on transient 403", 2)];
    // auto-fill minmax(220px, 1fr) with a 14px gap
    let inner = (width.min(880.) - 56.).max(220.);
    let cols = (((inner + 14.) / (220. + 14.)).floor() as usize).max(1);
    let card_w = (inner - 14. * (cols as f32 - 1.)) / cols as f32;
    base.child(
        div()
            .flex()
            .items_center()
            .gap(px(22.))
            .px(px(28.))
            .py(px(14.))
            .border_b_1()
            .border_color(hex(0xeeeeee))
            .bg(white())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(15.))
                    .font_weight(FontWeight::BOLD)
                    .child(div().size(px(18.)).rounded(px(5.)).bg(a))
                    .child("Tako web"),
            )
            .children(["Agents", "Pull requests", "Settings"].map(|s| div().text_size(px(13.)).text_color(hex(0x777777)).child(s))),
    )
    .child(
        div()
            .max_w(px(880.))
            .mx_auto()
            .px(px(28.))
            .py(px(44.))
            .child(div().text_size(px(30.)).font_weight(FontWeight::BOLD).line_height(relative(1.2)).mb(px(6.)).child("Background agents"))
            .child(div().text_color(hex(0x666666)).mb(px(26.)).child("Three agents are working in their own worktrees."))
            .child(div().flex().flex_wrap().gap(px(14.)).children(cards.map(|(title, i)| {
                div()
                    .w(px(card_w))
                    .bg(white())
                    .border_1()
                    .border_color(hex(0xeeeeee))
                    .rounded(px(12.))
                    .p(px(16.))
                    .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.03), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false }])
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .mb(px(4.))
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .child(div().size(px(7.)).rounded_full().bg(hex(0x3cb371)).mr(px(6.)))
                            .child(title),
                    )
                    .child(div().text_size(px(11.5)).text_color(hex(0x888888)).child(format!("step {} of 7 · {}m", 3 + i, 4 + i * 3)))
                    .child(
                        div().mt(px(14.)).h(px(5.)).rounded(px(5.)).bg(hex(0xf0eeec)).overflow_hidden().child(
                            div().h_full().rounded(px(5.)).bg(a).with_animation(
                                ("bz-bar", i),
                                Animation::new(Duration::from_secs(6)).repeat(),
                                move |d, p| {
                                    // 3s ease-in-out, alternate, delay -i s
                                    let q = (p + i as f32 / 6.) % 1.;
                                    let tri = if q < 0.5 { q * 2. } else { 2. - q * 2. };
                                    let e = 0.5 - 0.5 * (std::f32::consts::PI * tri).cos();
                                    d.w(relative(0.2 + 0.65 * e))
                                },
                            ),
                        ),
                    )
            }))),
    )
    .into_any_element()
}

