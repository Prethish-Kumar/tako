//! Usage rings in the rail + usage popover (port of the mock's USAGE section).
use crate::{
    app::TakoApp,
    overlays::theme_pop::{escape_closer, pop_progress},
    pages::settings::tgl,
    theme::{ActiveTheme, Theme, alpha},
    ui::*,
};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{cell::Cell, collections::HashSet, rc::Rc, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prov {
    Claude,
    Codex,
}
pub const PROVS: [Prov; 2] = [Prov::Claude, Prov::Codex];

impl Prov {
    pub fn name(self) -> &'static str {
        match self {
            Prov::Claude => "Claude",
            Prov::Codex => "Codex",
        }
    }
    pub fn cli(self) -> &'static str {
        match self {
            Prov::Claude => "Claude Code",
            Prov::Codex => "Codex CLI",
        }
    }
    pub fn mark(self) -> &'static str {
        match self {
            Prov::Claude => "spark",
            Prov::Codex => "code",
        }
    }
    pub fn color(self, t: &Theme) -> Hsla {
        match self {
            Prov::Claude => t.accent,
            Prov::Codex => t.blue,
        }
    }
    pub fn plans(self) -> &'static [&'static str] {
        match self {
            Prov::Claude => &["Pro", "Max 5×", "Max 20×", "Team"],
            Prov::Codex => &["Plus", "Pro", "Business"],
        }
    }
    fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
pub struct Acct {
    pub id: String,
    pub p: Prov,
    pub label: String,
    pub plan: String,
    pub synced: String,
    pub win: Vec<(&'static str, u32, String)>,
    pub note: Option<&'static str>,
}

impl Acct {
    pub fn week(&self) -> u32 {
        self.win.iter().find(|w| w.0 == "Week").or(self.win.first()).map(|w| w.1).unwrap_or(0)
    }
    pub fn capped(&self) -> bool {
        self.win.iter().any(|w| w.1 >= 100)
    }
}

pub struct UsageState {
    pub open: bool,
    pub accounts: Vec<Acct>,
    pub active: [String; 2],
    pub auto: bool,
    expanded: HashSet<String>,
    adding: Option<Prov>,
    plan_pick: usize,
    name: Entity<InputState>,
    refreshing: bool,
    shown: bool,
    tween: Option<(bool, Tween)>,
    /// bars grow from 0 each time the popover opens
    bars: Option<Tween>,
    rings: Tween,
    btn: Rc<Cell<Option<Bounds<Pixels>>>>,
    hover_btn: bool,
    hover_pop: bool,
    next_id: u32,
}

fn acct(id: &str, p: Prov, label: &str, plan: &str, synced: &str, win: &[(&'static str, u32, &str)], note: Option<&'static str>) -> Acct {
    Acct {
        id: id.into(),
        p,
        label: label.into(),
        plan: plan.into(),
        synced: synced.into(),
        win: win.iter().map(|(a, b, c)| (*a, *b, c.to_string())).collect(),
        note,
    }
}

impl UsageState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        use Prov::*;
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Account name, e.g. Side project"));
        cx.subscribe_in(&name, window, |this: &mut TakoApp, _, ev: &InputEvent, _, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                if let Some(p) = this.overlays.usage.adding {
                    this.usage_sign_in(p, cx);
                }
            }
        })
        .detach();
        UsageState {
            open: false,
            accounts: vec![
                acct("c1", Claude, "Personal", "Max 20×", "just now", &[("5-hour", 100, "resets 15:26"), ("Week", 60, "resets Mon 09:00"), ("Opus · week", 41, "resets Mon 09:00")], None),
                acct("c2", Claude, "Work", "Team", "2 min ago", &[("5-hour", 18, "resets 17:02"), ("Week", 34, "resets Sat 22:00")], None),
                acct("x1", Codex, "Personal", "Plus", "just now", &[("5-hour", 42, "resets 15:26"), ("Week", 23, "resets Thu 14:00")], Some("2 free full resets · next expires Thu 29 Oct")),
                acct("x2", Codex, "Acme client", "Pro", "18 h ago", &[("5-hour", 0, "resets 19:40"), ("Week", 8, "resets Mon 10:26")], None),
            ],
            active: ["c2".into(), "x1".into()],
            auto: true,
            expanded: HashSet::new(),
            adding: None,
            plan_pick: 0,
            name,
            refreshing: false,
            shown: false,
            tween: None,
            bars: None,
            rings: if std::env::var("TAKO_SNAP").is_ok() { Tween::delayed(0., 0.) } else { Tween::delayed(1200., 300.) },
            btn: Rc::default(),
            hover_btn: false,
            hover_pop: false,
            next_id: 0,
        }
    }
    pub fn accts(&self, p: Prov) -> Vec<&Acct> {
        self.accounts.iter().filter(|a| a.p == p).collect()
    }
    pub fn active(&self, p: Prov) -> Option<&Acct> {
        self.accounts.iter().find(|a| a.id == self.active[p.idx()]).or_else(|| self.accounts.iter().find(|a| a.p == p))
    }
    pub fn is_active(&self, a: &Acct) -> bool {
        self.active[a.p.idx()] == a.id
    }
    pub fn use_acct(&mut self, id: &str) {
        if let Some(a) = self.accounts.iter().find(|a| a.id == id) {
            self.active[a.p.idx()] = a.id.clone();
        }
        self.expanded.clear();
    }
    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
        for p in PROVS {
            if self.active[p.idx()] == id {
                self.active[p.idx()] = self.accounts.iter().find(|a| a.p == p).map(|a| a.id.clone()).unwrap_or_default();
            }
        }
    }
    pub fn add(&mut self, p: Prov, label: String, plan: String) {
        self.next_id += 1;
        let id = format!("{}n{}", if p == Prov::Claude { 'c' } else { 'x' }, self.next_id);
        self.accounts.push(Acct { id, p, label, plan, synced: "just now".into(), win: vec![("5-hour", 0, "resets in 5h".into()), ("Week", 0, "resets in 7d".into())], note: None });
    }
    pub fn set_auto(&mut self, v: bool) {
        self.auto = v;
        if v {
            for p in PROVS {
                if self.active(p).is_some_and(|a| a.capped()) {
                    if let Some(ok) = self.accounts.iter().find(|a| a.p == p && !a.capped()) {
                        self.active[p.idx()] = ok.id.clone();
                    }
                }
            }
        }
    }
}

impl TakoApp {
    fn usage_sign_in(&mut self, p: Prov, cx: &mut Context<Self>) {
        let u = &mut self.overlays.usage;
        let v = u.name.read(cx).value().trim().to_string();
        let plan = p.plans()[u.plan_pick % p.plans().len()].to_string();
        u.add(p, if v.is_empty() { "New account".into() } else { v }, plan);
        u.adding = None;
        cx.notify();
    }

    fn usage_hover_out(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(160)).await;
            this.update(cx, |this, cx| {
                let u = &mut this.overlays.usage;
                if !u.hover_btn && !u.hover_pop && u.adding.is_none() {
                    u.open = false;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_usage_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = cx.t();
        let u = &mut self.overlays.usage;
        let was = u.shown;
        let vis = pop_progress(u.open, &mut u.shown, &mut u.tween, 220., window)?;
        if u.open && !was && std::env::var("TAKO_SNAP").is_err() {
            u.bars = Some(Tween::delayed(900., 30.));
        }
        let bar_p = match &u.bars {
            Some(tw) => {
                if !tw.done() {
                    window.request_animation_frame();
                }
                ease_snappy(tw.raw())
            }
            None => 1.,
        };
        let vh = f32::from(window.viewport_size().height);
        let btn = u.btn.get().unwrap_or(Bounds::new(point(px(8.), px(vh - 82.)), size(px(36.), px(36.))));
        // place(): left = btn.right + 10, top = btn.bottom - popH + 4 (clamped to 8px)
        let left = f32::from(btn.right()) + 10. - 6. * (1. - vis);
        let bottom = (vh - f32::from(btn.bottom()) - 4.).max(8.);

        let bar = |v: u32, c: Hsla| {
            let w = v.min(100) as f32 / 100. * bar_p;
            div()
                .h(px(5.))
                .w_full()
                .rounded(px(5.))
                .bg(t.surface_active)
                .overflow_hidden()
                .child(div().h_full().w(relative(w)).rounded(px(5.)).bg(if v >= 100 { t.yellow } else { c }))
        };
        let pct = |v: u32| div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_right().text_color(if v >= 100 { t.yellow } else { t.text }).child(format!("{v}%"));

        let mut provs: Vec<AnyElement> = vec![];
        for p in PROVS {
            let m = p.color(&t);
            let list: Vec<Acct> = u.accts(p).into_iter().cloned().collect();
            let sw = if u.auto && list.len() > 1 { list.iter().find(|a| !u.is_active(a) && a.capped()).cloned() } else { None };
            let act_label = u.active(p).map(|a| a.label.clone()).unwrap_or_default();
            let mut sec = div()
                .pt(px(10.))
                .pb(px(4.))
                .border_t_1()
                .border_color(t.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .px(px(2.))
                        .pb(px(6.))
                        .child(div().size(px(20.)).rounded(px(6.)).bg(m).flex().items_center().justify_center().child(icon(p.mark(), 12., white())))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(p.name()))
                        .child(div().text_size(px(11.5)).text_color(t.text_3).child(format!("{} account{}", list.len(), if list.len() == 1 { "" } else { "s" })))
                        .child(div().flex_1())
                        .child(
                            div()
                                .id(SharedString::from(format!("up-add-{}", p.name())))
                                .size(px(22.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .hover(|s| s.bg(t.surface_hover))
                                .child(icon("plus", 13., t.text_3))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let u = &mut this.overlays.usage;
                                    u.adding = if u.adding == Some(p) { None } else { Some(p) };
                                    u.plan_pick = 0;
                                    if u.adding.is_some() {
                                        u.name.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                            s.focus(window, cx);
                                        });
                                    }
                                    cx.notify();
                                })),
                        ),
                );
            if let Some(sw) = sw {
                sec = sec.child(
                    div()
                        .mx(px(2.))
                        .mb(px(6.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(7.))
                        .bg(alpha(t.yellow, 0.12))
                        .text_size(px(11.5))
                        .text_color(t.text_2)
                        .child(format!("{} hit its limit, so new sessions use {}.", sw.label, act_label)),
                );
            }
            for a in &list {
                let on = u.is_active(a);
                let full = a.capped();
                let open = on || u.expanded.contains(&a.id);
                let wk = a.week();
                let aid = a.id.clone();
                let pick = {
                    let aid = aid.clone();
                    div()
                        .id(SharedString::from(format!("ua-pick-{}", a.id)))
                        .size(px(14.))
                        .flex_none()
                        .rounded_full()
                        .border(px(1.5))
                        .border_color(if on { m } else { t.text_3 })
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(on, |d| d.child(div().size(px(6.)).rounded_full().bg(m)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.overlays.usage.use_acct(&aid);
                            cx.notify();
                        }))
                };
                let toggle = {
                    let aid = aid.clone();
                    cx.listener(move |this: &mut TakoApp, _: &ClickEvent, _, cx: &mut Context<TakoApp>| {
                        let e = &mut this.overlays.usage.expanded;
                        if !e.remove(&aid) {
                            e.insert(aid.clone());
                        }
                        cx.notify();
                    })
                };
                let titles = |sub: String| {
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(div().text_size(px(12.5)).font_weight(FontWeight::SEMIBOLD).child(a.label.clone()))
                        .child(div().text_size(px(11.)).text_color(t.text_3).child(sub))
                };
                if !open {
                    sec = sec.child(
                        div()
                            .id(SharedString::from(format!("ua-{}", a.id)))
                            .mx(px(-6.))
                            .px(px(8.))
                            .py(px(7.))
                            .rounded(px(9.))
                            .mt(px(2.))
                            .hover(|s| s.bg(t.surface_hover))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(pick)
                            .child(titles(format!("{} · {}", a.plan, if full { "limit reached".into() } else { format!("updated {}", a.synced) })))
                            .child(div().flex_1())
                            .child(div().flex().items_center().gap(px(8.)).child(div().w(px(70.)).child(bar(wk, alpha(m, 0.7)))).child(div().w(px(34.)).child(pct(wk))))
                            .on_click(toggle),
                    );
                    continue;
                }
                let bar_c = if on { m } else { alpha(m, 0.7) };
                let status = if full { "limit reached".to_string() } else if on { "in use".into() } else { format!("updated {}", a.synced) };
                let mut card = div()
                    .mx(px(-6.))
                    .pt(px(8.))
                    .px(px(8.))
                    .pb(px(6.))
                    .mt(px(2.))
                    .rounded(px(9.))
                    .when(on, |d| d.bg(alpha(m, 0.08)))
                    .child(
                        div()
                            .id(SharedString::from(format!("ua-h-{}", a.id)))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .mb(px(6.))
                            .child(pick)
                            .child(titles(format!("{} · {}", p.cli(), a.plan)))
                            .child(div().flex_1())
                            .child(div().text_size(px(11.)).text_color(if full { t.yellow } else if on { m } else { t.text_3 }).child(status))
                            .when(!on, |d| d.on_click(toggle)),
                    );
                for (l, v, r) in &a.win {
                    card = card.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .py(px(3.))
                            .pl(px(23.))
                            .child(
                                div()
                                    .w(px(104.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .min_w_0()
                                    .child(div().text_size(px(12.)).text_color(if on { t.text_2 } else { t.text_3 }).child(*l))
                                    .child(div().text_size(px(10.5)).text_color(t.text_3).truncate().child(r.clone())),
                            )
                            .child(div().flex_1().child(bar(*v, bar_c)))
                            .child(div().w(px(38.)).flex_none().child(pct(*v))),
                    );
                }
                if let Some(n) = a.note {
                    card = card.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .mt(px(4.))
                            .ml(px(23.))
                            .text_size(px(11.))
                            .text_color(t.text_2)
                            .child(icon("spark", 11., m))
                            .child(n),
                    );
                }
                sec = sec.child(card);
            }
            if u.adding == Some(p) {
                let plan = p.plans()[u.plan_pick % p.plans().len()];
                sec = sec.child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .px(px(2.))
                        .pt(px(6.))
                        .pb(px(4.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .h(px(28.))
                                .px(px(9.))
                                .rounded(px(7.))
                                .border_1()
                                .border_color(t.border)
                                .bg(t.surface)
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .child(Input::new(&u.name).appearance(false).bordered(false).text_size(px(12.)).p_0()),
                        )
                        .child(
                            div()
                                .id("up-plan")
                                .h(px(28.))
                                .px(px(6.))
                                .rounded(px(7.))
                                .border_1()
                                .border_color(t.border)
                                .bg(t.surface)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child(plan)
                                .child(icon("chev-d", 11., t.text_3))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.overlays.usage.plan_pick += 1;
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .id("up-sign")
                                .h(px(28.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(m)
                                .text_color(white())
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .flex()
                                .items_center()
                                .child("Sign in")
                                .on_click(cx.listener(move |this, _, _, cx| this.usage_sign_in(p, cx))),
                        ),
                );
            }
            provs.push(sec.into_any_element());
        }

        let refreshing = u.refreshing;
        let auto = u.auto;
        let btn_b = u.btn.clone();
        let el = div()
            .id("usage-pop")
            .occlude()
            .absolute()
            .left(px(left))
            .bottom(px(bottom))
            .w(px(340.))
            .max_h(px(vh - 16.))
            .overflow_y_scroll()
            .pt(px(14.))
            .px(px(14.))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(t.panel_solid) // flat style: solid popovers
            .border_1()
            .border_color(t.border)
            .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.18), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
            .opacity(vis)
            .text_color(t.text)
            .on_hover(cx.listener(|this, h: &bool, _, cx| {
                this.overlays.usage.hover_pop = *h;
                if !*h {
                    this.usage_hover_out(cx);
                }
            }))
            .on_mouse_down_out(cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                if btn_b.get().is_some_and(|b| b.contains(&ev.position)) {
                    return;
                }
                this.overlays.usage.open = false;
                this.overlays.usage.adding = None;
                cx.notify();
            }))
            .child(escape_closer(cx.entity().downgrade(), |a| {
                a.overlays.usage.open = false;
                a.overlays.usage.adding = None;
            }))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .px(px(2.))
                    .pb(px(8.))
                    .child("Usage")
                    .child(
                        div()
                            .id("up-ref")
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .px(px(4.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(t.text_3)
                            .hover(|s| s.text_color(t.text))
                            .child(if refreshing { spinner("up-spin", 12., t.text_3).into_any_element() } else { icon("refresh", 12., t.text_3).into_any_element() })
                            .child(div().text_size(px(11.)).child("synced just now"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.overlays.usage.refreshing = true;
                                cx.notify();
                                cx.spawn(async move |this, cx| {
                                    cx.background_executor().timer(Duration::from_millis(700)).await;
                                    this.update(cx, |this, cx| {
                                        let u = &mut this.overlays.usage;
                                        u.refreshing = false;
                                        u.accounts.iter_mut().for_each(|a| a.synced = "just now".into());
                                        cx.notify();
                                    })
                                    .ok();
                                })
                                .detach();
                            })),
                    ),
            )
            .children(provs)
            .child(
                div()
                    .id("up-auto")
                    .border_t_1()
                    .border_color(t.border)
                    .pt(px(10.))
                    .px(px(2.))
                    .pb(px(2.))
                    .text_size(px(12.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .text_color(t.text_2)
                    .child("Switch accounts when a limit is hit")
                    .child(tgl(auto, 0.85, &t))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let u = &mut this.overlays.usage;
                        u.set_auto(!u.auto);
                        cx.notify();
                    })),
            );
        Some(el.into_any_element())
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// `usage` opens the popover, `usage:<account id>` also expands that account.
pub fn act(app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k != "usage" {
        return false;
    }
    let u = &mut app.overlays.usage;
    u.open = true;
    if !v.is_empty() {
        u.expanded.insert(v.to_string());
    }
    cx.notify();
    true
}

/// The rail button: Claude + Codex rings. Clicking toggles the popover.
pub fn usage_ring(app: &mut TakoApp, cx: &mut Context<TakoApp>) -> impl IntoElement {
    let t = cx.t();
    let u = &app.overlays.usage;
    let e = ease_snappy(u.rings.raw());
    let animating = !u.rings.done();
    // dashoffset transitions: Codex starts .1s later
    let e2 = ease_snappy(((u.rings.start.elapsed().as_secs_f32() * 1000. - 400.) / 1200.).clamp(0., 1.));
    let vals: Vec<(f32, Hsla)> = PROVS
        .iter()
        .enumerate()
        .map(|(i, p)| (u.active(*p).map(|a| a.week()).unwrap_or(0) as f32 / 100. * if i == 0 { e } else { e2 }, p.color(&t)))
        .collect();
    let hot = PROVS.iter().any(|p| u.active(*p).is_some_and(|a| a.capped()));
    let trk = t.surface_active;
    let btn_cell = u.btn.clone();
    let open = u.open;
    div()
        .id("usage")
        .relative()
        .size(px(36.))
        .rounded(px(9.))
        .flex()
        .items_center()
        .justify_center()
        .when(open, |d| d.bg(t.surface_hover))
        .hover(|s| s.bg(t.surface_hover))
        .child(
            canvas(
                move |b, _, _| btn_cell.set(Some(b)),
                move |b, _, window, _| {
                    if animating {
                        window.request_animation_frame();
                    }
                    // 26px svg with a 36-unit viewBox
                    let k = 26. / 36.;
                    let c = b.center();
                    for (i, r) in [13f32, 8.5].iter().enumerate() {
                        let r = r * k;
                        let w = 3. * k;
                        ring_arc(window, c, r, w, 1., trk);
                        let (v, col) = vals[i];
                        if v > 0.001 {
                            ring_arc(window, c, r, w, v, col);
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .when(hot, |d| {
            d.child(div().absolute().top(px(3.)).right(px(3.)).rounded_full().bg(t.yellow).border(px(2.)).border_color(t.bg).size(px(10.)))
        })
        .on_hover(cx.listener(|this, h: &bool, _, cx| {
            let u = &mut this.overlays.usage;
            u.hover_btn = *h;
            if *h {
                u.open = true;
                cx.notify();
            } else {
                this.usage_hover_out(cx);
            }
        }))
        .on_click(cx.listener(|this, _, _, cx| {
            let u = &mut this.overlays.usage;
            u.open = !u.open;
            cx.notify();
        }))
}

/// A round-capped arc starting at 12 o'clock, clockwise, covering `frac` of the circle.
fn ring_arc(window: &mut Window, c: Point<Pixels>, r: f32, w: f32, frac: f32, col: Hsla) {
    let full = frac >= 0.999;
    let n = ((64. * frac).ceil() as usize).max(2);
    let pt = |a: f32| point(c.x + px(r * a.sin()), c.y - px(r * a.cos()));
    let mut pb = PathBuilder::stroke(px(w));
    let tau = std::f32::consts::TAU;
    let end = if full { tau } else { tau * frac };
    pb.move_to(pt(0.));
    for i in 1..=n {
        pb.line_to(pt(end * i as f32 / n as f32));
    }
    if full {
        pb.close();
    }
    if let Ok(p) = pb.build() {
        window.paint_path(p, col);
    }
    if !full {
        // round line caps
        for a in [0., end] {
            let p = pt(a);
            let h = w / 2.;
            window.paint_quad(fill(Bounds::new(point(p.x - px(h), p.y - px(h)), size(px(w), px(w))), col).corner_radii(px(h)));
        }
    }
}
