//! Settings page (port of the mock's SETTINGS section).
use crate::{
    app::{TakoApp, View},
    overlays::{
        theme_pop::{Grad, auto_orb_cols, is_light, orb, rgb_of, set_mode, set_pal},
        usage::PROVS,
    },
    pix::pix,
    theme::{ActiveTheme, PRESETS, Theme, ThemeState, alpha, hex},
    ui::*,
};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

const SECTIONS: &[(&str, &str, &str)] = &[
    ("general", "General", "gear"),
    ("appearance", "Appearance", "palette"),
    ("machines", "Machines & relay", "server"),
    ("worktrees", "Worktrees", "worktree"),
    ("models", "Models & usage", "spark"),
    ("labels", "Labels", "tag"),
    ("shortcuts", "Keyboard shortcuts", "term"),
    ("account", "Account", "lock"),
];

pub const LABEL_COLORS: [u32; 8] = [0x7aa7e8, 0x5fc48a, 0xe2b75a, 0xc77dff, 0xec7aa0, 0x4fb3c8, 0xe5675f, 0x9aa3b0];

#[derive(Clone, Debug)]
pub struct Lbl {
    pub id: String,
    pub name: String,
    pub c: u32,
}

/// Per-worktree size (MB) and uncommitted changes, from the mock's SIZE / META tables.
fn wt_size(id: usize) -> u32 {
    match id {
        1 => 412,
        2 => 388,
        3 => 640,
        4 => 296,
        5 => 351,
        6 => 820,
        7 => 274,
        8 => 905,
        9 => 330,
        10 => 186,
        11 => 512,
        _ => 300,
    }
}
fn wt_dirty(id: usize) -> bool {
    matches!(id, 1 | 2 | 4 | 5 | 7 | 9 | 10)
}
fn gb(mb: u32) -> String {
    if mb >= 1000 { format!("{:.1} GB", mb as f32 / 1000.) } else { format!("{mb} MB") }
}

pub struct SettingsState {
    pub sec: &'static str,
    pub startup: &'static str,
    pub bools: HashMap<&'static str, bool>,
    pub sels: HashMap<&'static str, String>,
    pub effort: &'static str,
    pub branch_prefix: Entity<InputState>,
    pub wt_setup: Entity<InputState>,
    pub wt_pick: Option<HashSet<usize>>,
    pub wt_freed: u32,
    pub labels: Vec<Lbl>,
    edit_label: Option<String>,
    adding_label: bool,
    new_label_c: usize,
    label_input: Entity<InputState>,
    del_sure: Option<String>,
    reset_armed: bool,
    open_sel: Option<&'static str>,
    toast: Option<(String, u32)>,
    toast_seq: u32,
}

impl SettingsState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        let branch_prefix = cx.new(|cx| InputState::new(window, cx).default_value("tako/"));
        let wt_setup = cx.new(|cx| InputState::new(window, cx).default_value("bun install"));
        let label_input = cx.new(|cx| InputState::new(window, cx).placeholder("Label name"));
        cx.subscribe_in(&label_input, window, |this: &mut TakoApp, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.label_commit(window, cx);
            }
        })
        .detach();
        let bools = HashMap::from([
            ("notifyDone", true),
            ("notifyAsk", true),
            ("notifyFail", true),
            ("sounds", false),
            ("vmAutoDelete", true),
            ("wtMerged", true),
            ("wtDirty", true),
            ("wtEnv", true),
        ]);
        let sels = HashMap::from([
            ("vmSize", "Medium".to_string()),
            ("vmRegion", "eu-west".into()),
            ("wtClosed", "After 3 days".into()),
            ("wtIdle", "After 30 days".into()),
            ("wtMax", "10 per project".into()),
            ("model", "Opus 5.5".into()),
            ("access", "Full access".into()),
        ]);
        let mut labels: Vec<Lbl> = crate::data::LABELS.iter().map(|l| Lbl { id: l.id.into(), name: l.name.into(), c: l.color }).collect();
        for (id, name, c) in [("client", "Client", 0xc77dff), ("maint", "Maintenance", 0x9aa3b0)] {
            if !labels.iter().any(|l| l.id == id) {
                labels.push(Lbl { id: id.into(), name: name.into(), c });
            }
        }
        SettingsState {
            sec: "general",
            startup: "home",
            bools,
            sels,
            effort: "High",
            branch_prefix,
            wt_setup,
            wt_pick: None,
            wt_freed: 0,
            labels,
            edit_label: None,
            adding_label: false,
            new_label_c: 0,
            label_input,
            del_sure: None,
            reset_armed: false,
            open_sel: None,
            toast: None,
            toast_seq: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared controls (also used by the popovers).

/// The mock's `.tgl` switch, optionally scaled (the usage popover uses .85).
pub fn tgl(on: bool, scale: f32, t: &Theme) -> Div {
    let k = scale;
    div()
        .relative()
        .flex_none()
        .w(px(36. * k))
        .h(px(20. * k))
        .rounded_full()
        .bg(if on { t.accent } else { t.surface_active })
        .child(
            div()
                .absolute()
                .top(px(2. * k))
                .left(px((2. + if on { 16. } else { 0. }) * k))
                .size(px(16. * k))
                .rounded_full()
                .bg(white())
                .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.3), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false }]),
        )
}

/// `.btn.sm`
pub fn btn_sm(id: impl Into<ElementId>, t: &Theme) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.))
        .h(px(28.))
        .px(px(11.))
        .rounded(px(8.))
        .border_1()
        .border_color(t.border)
        .bg(t.surface)
        .text_color(t.text)
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .hover(|s| s.bg(t.surface_hover))
}

type Pick = fn(&mut TakoApp, &'static str, &mut Window, &mut Context<TakoApp>);

fn row(t: &Theme, first: bool, title: impl Into<SharedString>, sub: Option<SharedString>, control: impl IntoElement) -> Div {
    row_lead(t, first, None, title, sub, control)
}

fn row_lead(t: &Theme, first: bool, lead: Option<AnyElement>, title: impl Into<SharedString>, sub: Option<SharedString>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(14.))
        .w_full()
        .min_h(px(60.))
        .py(px(12.))
        .px(px(18.))
        .when(!first, |d| d.border_t_1().border_color(t.border))
        .children(lead)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).text_color(t.text).child(title.into()))
                .when_some(sub, |d, s| d.child(div().text_size(px(12.5)).text_color(t.text_3).child(s))),
        )
        .child(div().flex().items_center().gap(px(6.)).flex_none().child(control))
}

fn card(t: &Theme) -> Div {
    div().w_full().border_1().border_color(t.border).rounded(px(10.)).bg(t.panel_solid).flex().flex_col()
}

/// Stack rows into a card, drawing the hairlines between them.
fn rows_card(t: &Theme, rows: Vec<(SharedString, Option<SharedString>, AnyElement)>) -> Div {
    card(t).children(rows.into_iter().enumerate().map(|(i, (a, b, c))| row(t, i == 0, a, b, c)))
}

fn h2(t: &Theme, s: &str) -> Div {
    div().mb(px(18.)).text_size(px(24.)).font_weight(FontWeight::SEMIBOLD).text_color(t.text).child(s.to_string())
}
fn h3(t: &Theme, s: &str) -> Div {
    div().mt(px(26.)).mb(px(10.)).mx(px(2.)).text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(t.text_2).child(s.to_string())
}
fn note(t: &Theme, s: &str) -> Div {
    div().mx(px(2.)).mb(px(14.)).text_size(px(13.)).line_height(px(19.5)).text_color(t.text_3).child(s.to_string())
}

impl TakoApp {
    fn st_toast(&mut self, msg: &str, cx: &mut Context<Self>) {
        let st = &mut self.pages.settings;
        st.toast_seq += 1;
        let seq = st.toast_seq;
        st.toast = Some((msg.to_string(), seq));
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(1800)).await;
            this.update(cx, |this, cx| {
                if this.pages.settings.toast.as_ref().is_some_and(|t| t.1 == seq) {
                    this.pages.settings.toast = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn label_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let st = &mut self.pages.settings;
        let v = st.label_input.read(cx).value().trim().to_string();
        if let Some(id) = st.edit_label.take() {
            if !v.is_empty() {
                if let Some(l) = st.labels.iter_mut().find(|l| l.id == id) {
                    l.name = v;
                }
            }
        } else if st.adding_label {
            if v.is_empty() {
                st.label_input.update(cx, |s, cx| s.focus(window, cx));
                return;
            }
            let id = format!("l{}", st.labels.len() + 1);
            st.labels.push(Lbl { id, name: v.clone(), c: LABEL_COLORS[st.new_label_c % 8] });
            st.adding_label = false;
            self.st_toast(&format!("Added label {v}"), cx);
        }
        cx.notify();
    }

    fn seg(&self, t: &Theme, id: &str, val: &str, opts: &[(&'static str, &'static str)], pick: Pick, cx: &mut Context<Self>) -> Div {
        div()
            .min_w(px(200.))
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(9.))
            .bg(t.surface)
            .children(opts.iter().map(|(v, n)| {
                let on = *v == val;
                let v = *v;
                div()
                    .id(SharedString::from(format!("{id}-{v}")))
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py(px(6.))
                    .px(px(12.))
                    .rounded(px(7.))
                    .text_size(px(12.5))
                    .whitespace_nowrap()
                    .when(on, |d| d.bg(t.surface_active).text_color(t.text))
                    .when(!on, |d| d.text_color(t.text_3))
                    .child(*n)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        pick(this, v, window, cx);
                        cx.notify();
                    }))
            }))
    }

    fn tgl_pref(&self, t: &Theme, key: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let on = self.pages.settings.bools.get(key).copied().unwrap_or(false);
        tgl(on, 1., t)
            .id(SharedString::from(format!("tgl-{key}")))
            .on_click(cx.listener(move |this, _, _, cx| {
                let b = this.pages.settings.bools.entry(key).or_default();
                *b = !*b;
                cx.notify();
            }))
            .into_any_element()
    }

    /// `.au-sel` — a value with a chevron that opens a small menu of options.
    fn sel(&self, t: &Theme, key: &'static str, opts: &'static [&'static str], cx: &mut Context<Self>) -> AnyElement {
        let st = &self.pages.settings;
        let val = st.sels.get(key).cloned().unwrap_or_default();
        let open = st.open_sel == Some(key);
        div()
            .id(SharedString::from(format!("sel-{key}")))
            .relative()
            .flex()
            .items_center()
            .gap(px(6.))
            .h(px(30.))
            .pl(px(12.))
            .pr(px(10.))
            .rounded(px(7.))
            .bg(t.surface_active)
            .hover(|s| s.bg(alpha(t.text, 0.12)))
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(t.text)
            .whitespace_nowrap()
            .flex_none()
            .child(val.clone())
            .child(icon("chev-d", 12., t.text_3))
            .on_click(cx.listener(move |this, _, _, cx| {
                let s = &mut this.pages.settings;
                s.open_sel = if s.open_sel == Some(key) { None } else { Some(key) };
                cx.notify();
            }))
            .when(open, |d| {
                d.child(
                    deferred(
                            div()
                                .id(SharedString::from(format!("menu-{key}")))
                                .occlude()
                                .absolute()
                                .top(px(34.))
                                .right_0()
                                .min_w(px(160.))
                                .p(px(4.))
                                .rounded(px(9.))
                                .bg(t.panel_solid)
                                .border_1()
                                .border_color(t.border)
                                .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.18), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
                                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                    this.pages.settings.open_sel = None;
                                    cx.notify();
                                }))
                                .children(opts.iter().map(|o| {
                                    let on = *o == val;
                                    div()
                                        .id(SharedString::from(format!("opt-{key}-{o}")))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .h(px(30.))
                                        .px(px(10.))
                                        .rounded(px(6.))
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::NORMAL)
                                        .text_color(t.text)
                                        .hover(|s| s.bg(t.surface_hover))
                                        .child(div().flex_1().child(*o))
                                        .when(on, |d| d.child(icon("check", 13., t.accent)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            let s = &mut this.pages.settings;
                                            s.sels.insert(key, o.to_string());
                                            s.open_sel = None;
                                            cx.notify();
                                        }))
                                })),
                    )
                    .with_priority(1),
                )
            })
            .into_any_element()
    }

    fn text_input(&self, t: &Theme, st: &Entity<InputState>, w: f32, mono: bool, radius: f32, fs: f32) -> AnyElement {
        div()
            .w(px(w))
            .h(px(30.))
            .px(px(10.))
            .rounded(px(radius))
            .border_1()
            .border_color(t.border)
            .bg(t.surface)
            .flex()
            .items_center()
            .text_size(px(fs))
            .text_color(t.text)
            .when(mono, |d| d.font_family(MONO))
            .child(Input::new(st).appearance(false).bordered(false).text_size(px(fs)).p_0())
            .into_any_element()
    }

    pub fn render_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let sec = self.pages.settings.sec;
        let nav = div()
            .w(px(300.))
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(t.border)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(48.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .pl(px(16.))
                    .pr(px(10.))
                    .border_b_1()
                    .border_color(t.border)
                    .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(t.text).child("Settings")),
            )
            .child(div().p(px(8.)).flex().flex_col().gap(px(2.)).children(SECTIONS.iter().map(|(k, n, i)| {
                let on = *k == sec;
                let k = *k;
                div()
                    .id(SharedString::from(format!("st-{k}")))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .w_full()
                    .py(px(9.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .text_size(px(13.5))
                    .when(on, |d| d.bg(t.surface_active).text_color(t.text))
                    .when(!on, |d| d.text_color(t.text_2).hover(|s| s.bg(t.surface_hover)))
                    .child(icon(i, 16., if on { t.accent } else { t.text_3 }))
                    .child(*n)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let s = &mut this.pages.settings;
                        s.sec = k;
                        s.edit_label = None;
                        s.adding_label = false;
                        s.reset_armed = false;
                        s.open_sel = None;
                        s.del_sure = None;
                        cx.notify();
                    }))
            })));

        let body: Vec<AnyElement> = match sec {
            "general" => self.sec_general(&t, cx),
            "appearance" => self.sec_appearance(&t, cx),
            "machines" => self.sec_machines(&t, cx),
            "worktrees" => self.sec_worktrees(&t, cx),
            "models" => self.sec_models(&t, cx),
            "labels" => self.sec_labels(&t, window, cx),
            "shortcuts" => self.sec_shortcuts(&t),
            _ => self.sec_account(&t, cx),
        };

        let toast = self.pages.settings.toast.clone();
        div()
            .size_full()
            .pt(px(44.))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .border_t_1()
                    .border_l_1()
                    .border_color(t.border)
                    .rounded_tl(px(12.))
                    .bg(t.bg)
                    .overflow_hidden()
                    .child(nav)
                    .child(
                        div()
                            .id("st-main")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_y_scroll()
                            .child(div().max_w(px(760.)).pt(px(30.)).px(px(40.)).pb(px(80.)).flex().flex_col().children(body)),
                    )
                    .when_some(toast, |d, (msg, _)| {
                        d.child(
                            div().absolute().bottom(px(28.)).left_0().right_0().flex().justify_center().child(
                                div()
                                    .px(px(14.))
                                    .py(px(9.))
                                    .rounded(px(9.))
                                    .bg(t.panel_solid)
                                    .border_1()
                                    .border_color(t.border)
                                    .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.2), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
                                    .text_size(px(13.))
                                    .child(msg),
                            ),
                        )
                    }),
            )
    }

    fn sec_general(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = &self.pages.settings;
        let startup = self.seg(t, "startup", st.startup, &[("home", "Home"), ("chats", "Last chat")], |a, v, _, _| a.pages.settings.startup = v, cx);
        let prefix = self.text_input(t, &self.pages.settings.branch_prefix, 180., false, 7., 13.);
        let replay = btn_sm("st-replay", t).child("Replay").on_click(cx.listener(|this, _, window, cx| this.replay_intro(window, cx)));
        vec![
            h2(t, "General").into_any_element(),
            rows_card(
                t,
                vec![
                    ("Open on startup".into(), Some("What you see when Tako launches".into()), startup.into_any_element()),
                    ("Branch prefix".into(), Some("Used for new worktree branches".into()), prefix),
                    ("Introduction".into(), Some("Watch the Tako story again".into()), replay.into_any_element()),
                ],
            )
            .into_any_element(),
            h3(t, "Notifications").into_any_element(),
            rows_card(
                t,
                vec![
                    ("An agent finishes".into(), Some("When a session is ready for review".into()), self.tgl_pref(t, "notifyDone", cx)),
                    ("An agent has a question".into(), Some("When a session is waiting on you".into()), self.tgl_pref(t, "notifyAsk", cx)),
                    ("Checks fail".into(), Some("When CI fails on one of your PRs".into()), self.tgl_pref(t, "notifyFail", cx)),
                    ("Sounds".into(), Some("Play a short sound with notifications".into()), self.tgl_pref(t, "sounds", cx)),
                ],
            )
            .into_any_element(),
        ]
    }

    /// Hook for "Replay intro": opens the onboarding story.
    fn replay_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_onboarding(window, cx);
    }

    fn sec_appearance(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let ts = cx.global::<ThemeState>();
        let pal = ts.pal.clone();
        let auto_on = pal == "auto";
        let light = is_light(ts);
        let _ = auto_orb_cols;
        let auto_row = div()
            .id("st-auto")
            .flex()
            .items_center()
            .gap(px(14.))
            .min_h(px(60.))
            .py(px(12.))
            .px(px(18.))
            .when(auto_on, |d| d.bg(alpha(t.accent, 0.06)))
            .when(!auto_on, |d| d.hover(|s| s.bg(t.surface_hover)))
            .child(orb(Grad::Conic { from: 210. }, &[rgb_of(t.accent), rgb_of(t.accent_2), rgb_of(t.bg), rgb_of(t.accent)], 30., 0.15))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child("Auto"))
                    .child(div().text_size(px(12.5)).text_color(t.text_3).child("Picks colours from the backdrop behind the composer")),
            )
            .when(auto_on, |d| d.child(icon("check", 16., t.accent)))
            .on_click(|_, _, cx| set_pal(cx, "auto"));
        let presets = div()
            .grid()
            .grid_cols(6)
            .gap(px(8.))
            .py(px(14.))
            .px(px(18.))
            .border_t_1()
            .border_color(t.border)
            .children(PRESETS.iter().map(|p| {
                let on = pal == p.id;
                let id = p.id;
                div()
                    .id(SharedString::from(format!("st-pal-{id}")))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .border_1()
                    .text_size(px(12.))
                    .when(on, |d| d.border_color(alpha(t.accent, 0.55)).text_color(t.text))
                    .when(!on, |d| d.border_color(transparent_black()).text_color(t.text_2).hover(|s| s.bg(t.surface_hover)))
                    .child(orb(Grad::Bands { s0: 0.55, s1: 2. }, &[hex(p.accent), hex(p.second)], 26., 0.14))
                    .child(p.name)
                    .on_click(move |_, _, cx| set_pal(cx, id))
            }));
        let mode = self.seg(t, "mode", if light { "light" } else { "dark" }, &[("dark", "Dark"), ("light", "Light")], |_, v, _, cx| set_mode(cx, v == "light"), cx);
        let scene = self.scene.id;
        let scenes = self.seg(
            t,
            "scene",
            scene,
            &[("tako", "Tako"), ("aurora", "Aurora"), ("neon", "Neon Rain"), ("peaks", "Peaks"), ("glade", "Glade"), ("valley", "Valley")],
            |a, v, w, cx| {
                crate::scene::act(a, "scene", v, w, cx);
            },
            cx,
        );
        vec![
            h2(t, "Appearance").into_any_element(),
            h3(t, "Theme").into_any_element(),
            card(t)
                .overflow_hidden()
                .child(auto_row)
                .child(presets)
                .child(row(t, false, "Appearance", if auto_on { Some("Auto chooses from the backdrop".into()) } else { None }, mode))
                .into_any_element(),
            h3(t, "Home scene").into_any_element(),
            card(t).child(row(t, true, "Backdrop", Some("The pixel scene behind the composer".into()), scenes)).into_any_element(),
        ]
    }

    fn sec_machines(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let flow = |id: &'static str| {
            let a = t.accent;
            div().flex_1().h(px(2.)).relative().overflow_hidden().child(
                div()
                    .absolute()
                    .top_0()
                    .h(px(2.))
                    .flex()
                    .children((0..80).map(move |_| div().w(px(11.)).h(px(2.)).flex_none().child(div().w(px(5.)).h_full().bg(a))))
                    .with_animation(id, Animation::new(Duration::from_millis(900)).repeat(), |d, p| d.left(px(-11. + 11. * p))),
            )
        };
        let node = |ic: &'static str, n: &'static str| {
            div().flex().flex_col().items_center().gap(px(6.)).whitespace_nowrap().child(icon(ic, 22., t.text)).child(n)
        };
        let relay = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .py(px(18.))
            .px(px(20.))
            .mb(px(10.))
            .rounded(px(10.))
            .border_1()
            .border_color(t.border)
            .bg(t.panel_solid)
            .text_size(px(12.))
            .text_color(t.text_3)
            .child(node("phone", "iPhone"))
            .child(flow("flow1"))
            .child(node("relay", "Relay · eu-west"))
            .child(flow("flow2"))
            .child(node("laptop", "My MacBook"));
        let machines: [(&str, &str, &str, &str, &str); 4] = [
            ("My MacBook", "This computer · macOS · 12 cores", "laptop", "ok", "28 ms"),
            ("devbox-01", "SSH · us-east-1 · 32 GB", "server", "ok", "61 ms"),
            ("gpu-runner", "SSH · eu-west · 1× A100", "server", "idle", "idle"),
            ("staging-api", "SSH · offline since 2h", "server", "off", "offline"),
        ];
        let mut mc = card(t).overflow_hidden();
        for (i, (n, s, ic, stt, lat)) in machines.iter().enumerate() {
            let dot = match *stt {
                "ok" => t.green,
                "idle" => t.yellow,
                _ => t.text_3,
            };
            let lead = div().size(px(30.)).rounded(px(8.)).flex_none().bg(t.surface_active).flex().items_center().justify_center().child(icon(ic, 16., t.text_2));
            mc = mc.child(
                row_lead(t, i == 0, Some(lead.into_any_element()), *n, Some((*s).into()), div().flex().items_center().gap(px(7.)).text_size(px(12.5)).text_color(t.text_3).child(div().size(px(6.)).rounded_full().bg(dot)).child(*lat)),
            );
        }
        for (id, ic, label, msg) in [("st-ssh", "plus", "Add SSH server", "SSH setup would open here"), ("st-pair", "phone", "Pair a phone", "A pairing code would appear here")] {
            mc = mc.child(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .min_h(px(48.))
                    .px(px(18.))
                    .border_t_1()
                    .border_color(t.border)
                    .text_size(px(13.5))
                    .text_color(t.accent)
                    .hover(|s| s.bg(t.surface_hover))
                    .child(icon(ic, 15., t.accent))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| this.st_toast(msg, cx))),
            );
        }
        vec![
            h2(t, "Machines & relay").into_any_element(),
            relay.into_any_element(),
            note(t, "Your phone and other devices reach your machines through the Tako relay. Traffic is end-to-end encrypted; the relay can't read it.").into_any_element(),
            h3(t, "Machines").into_any_element(),
            mc.into_any_element(),
            h3(t, "Ephemeral VMs").into_any_element(),
            rows_card(
                t,
                vec![
                    ("Default size".into(), Some("Used when you pick “Ephemeral VM”".into()), self.sel(t, "vmSize", &["Small", "Medium", "Large"], cx)),
                    ("Region".into(), Some("Closest to your repositories is fastest".into()), self.sel(t, "vmRegion", &["eu-west", "us-east", "us-west", "ap-south"], cx)),
                    ("Delete when the session ends".into(), Some("Turn off to keep the VM for follow-ups".into()), self.tgl_pref(t, "vmAutoDelete", cx)),
                ],
            )
            .into_any_element(),
        ]
    }

    fn sec_worktrees(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        use crate::data::PrState;
        let mut th = self.threads.clone();
        th.sort_by_key(|x| x.id);
        let days = |x: &crate::data::Thread| x.ago_min / 1440;
        let why = |x: &crate::data::Thread| -> Option<(String, &'static str)> {
            match x.pr_state {
                PrState::Merged => Some(("PR merged".into(), "ok")),
                PrState::Closed => Some(("PR closed".into(), "muted")),
                _ if days(x) >= 5 => Some((format!("Idle {} days", days(x)), "warn")),
                _ => None,
            }
        };
        let dirty = |x: &crate::data::Thread| wt_dirty(x.id) && x.pr_state != PrState::Merged;
        let cand: Vec<_> = th.iter().filter(|x| why(x).is_some() && !x.running).cloned().collect();
        let st = &mut self.pages.settings;
        if st.wt_pick.is_none() {
            st.wt_pick = Some(cand.iter().filter(|x| !dirty(x)).map(|x| x.id).collect());
        }
        let pick = st.wt_pick.clone().unwrap_or_default();
        let freed = st.wt_freed;
        // per-project totals, in first-seen order
        let mut by_p: Vec<(String, u32)> = vec![];
        for x in &th {
            match by_p.iter_mut().find(|(k, _)| *k == x.repo) {
                Some(e) => e.1 += wt_size(x.id),
                None => by_p.push((x.repo.clone(), wt_size(x.id))),
            }
        }
        let total: u32 = by_p.iter().map(|e| e.1).sum();
        let pick_mb: u32 = cand.iter().filter(|x| pick.contains(&x.id)).map(|x| wt_size(x.id)).sum();
        let cols = [t.accent, t.blue, hsla_hex(0x5fc48a), hsla_hex(0xe2b75a), hsla_hex(0xc77dff), hsla_hex(0xec7aa0)];
        let proj = |id: &str| self.projects.iter().find(|p| p.id == id).map(|p| (p.name, p.icon)).unwrap_or(("?", "star"));

        let sum = card(t)
            .py(px(16.))
            .px(px(18.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .mb(px(12.))
                    .child(div().text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).child(gb(total)))
                    .child(div().text_size(px(12.5)).text_color(t.text_3).child(format!("across {} worktree{} on My MacBook", th.len(), if th.len() == 1 { "" } else { "s" })))
                    .when(freed > 0, |d| {
                        d.child(div().flex_1()).child(div().flex().items_center().gap(px(5.)).text_size(px(12.)).text_color(t.green).child(icon("check", 13., t.green)).child(format!("Freed {}", gb(freed))))
                    }),
            )
            .child(div().flex().gap(px(2.)).h(px(8.)).rounded(px(4.)).overflow_hidden().children(by_p.iter().enumerate().map(|(i, (_, v))| div().flex_grow(*v as f32).flex_basis(px(0.)).min_w(px(3.)).h_full().bg(cols[i % 6]))))
            .child(div().flex().flex_wrap().gap_x(px(16.)).gap_y(px(6.)).mt(px(12.)).text_size(px(12.)).text_color(t.text_2).children(by_p.iter().enumerate().map(|(i, (k, v))| {
                let (n, ic) = proj(k);
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(8.)).rounded(px(2.)).bg(cols[i % 6]))
                    .child(pix(ic, 12.))
                    .child(n)
                    .child(div().text_color(t.text_3).child(gb(*v)))
            })));

        let mut clean = card(t).overflow_hidden();
        if cand.is_empty() {
            clean = clean.child(div().flex().items_center().gap(px(8.)).py(px(16.)).px(px(18.)).text_size(px(13.)).text_color(t.text_2).child(icon("check", 15., t.green)).child("Nothing to clean up. Every worktree is in use."));
        } else {
            for (i, x) in cand.iter().enumerate() {
                let (w, kind) = why(x).unwrap();
                let on = pick.contains(&x.id);
                let id = x.id;
                let (wc, wb) = match kind {
                    "ok" => (hsla_hex(0xb48ae8), alpha(hsla_hex(0xb48ae8), 0.14)),
                    "warn" => (t.yellow, alpha(t.yellow, 0.13)),
                    _ => (t.text_2, t.surface_active),
                };
                let (_, ic) = proj(&x.repo);
                clean = clean.child(
                    div()
                        .id(SharedString::from(format!("wt-{id}")))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .py(px(11.))
                        .px(px(18.))
                        .when(i > 0, |d| d.border_t_1().border_color(t.border))
                        .hover(|s| s.bg(t.surface_hover))
                        .child(
                            div()
                                .size(px(16.))
                                .flex_none()
                                .rounded(px(5.))
                                .border(px(1.5))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(on, |d| d.bg(t.accent).border_color(t.accent).child(icon("check", 11., white())))
                                .when(!on, |d| d.border_color(t.text_3)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(3.))
                                .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child(x.title.clone()))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_size(px(11.5))
                                        .font_family(MONO)
                                        .text_color(t.text_3)
                                        .child(pix(ic, 11.))
                                        .child(x.branch.clone())
                                        .when(dirty(x), |d| d.child("·").child(div().font_family(FONT).text_color(t.yellow).child("uncommitted changes"))),
                                ),
                        )
                        .child(div().text_size(px(11.)).px(px(8.)).py(px(2.)).rounded_full().whitespace_nowrap().text_color(wc).bg(wb).child(w))
                        .child(div().w(px(64.)).flex_none().text_right().text_size(px(12.)).text_color(t.text_2).child(gb(wt_size(id))))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let p = this.pages.settings.wt_pick.get_or_insert_default();
                            if !p.remove(&id) {
                                p.insert(id);
                            }
                            cx.notify();
                        })),
                );
            }
            let n = pick.len();
            clean = clean.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .py(px(12.))
                    .px(px(18.))
                    .border_t_1()
                    .border_color(t.border)
                    .child(div().flex_1().text_size(px(11.5)).text_color(t.text_3).child("Removes the worktree folder. Chats are archived; branches and PRs stay on the remote."))
                    .child(
                        btn_sm("wt-clean", t)
                            .bg(t.accent)
                            .border_color(transparent_black())
                            .text_color(white())
                            .when(n == 0, |d| d.opacity(0.45))
                            .child(format!("Clean up {n} · {}", gb(pick_mb)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let ids: Vec<usize> = this.pages.settings.wt_pick.take().unwrap_or_default().into_iter().collect();
                                if ids.is_empty() {
                                    this.pages.settings.wt_pick = Some(HashSet::new());
                                    return;
                                }
                                this.pages.settings.wt_freed += ids.iter().map(|i| wt_size(*i)).sum::<u32>();
                                // archive: drop the chats + their worktrees
                                this.threads.retain(|x| !ids.contains(&x.id));
                                if ids.contains(&this.active) {
                                    if let Some(x) = this.threads.first() {
                                        this.active = x.id;
                                    }
                                    if this.view == View::Chats {
                                        this.view = View::Home;
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            );
        }

        vec![
            h2(t, "Worktrees").into_any_element(),
            note(t, "Every chat gets its own worktree, so they add up. Clean up the ones you're done with, or let Tako do it for you.").into_any_element(),
            sum.into_any_element(),
            h3(t, "Ready to clean up").into_any_element(),
            clean.into_any_element(),
            h3(t, "Automatic cleanup").into_any_element(),
            rows_card(
                t,
                vec![
                    ("When a PR is merged".into(), Some("Remove its worktree and archive the chat".into()), self.tgl_pref(t, "wtMerged", cx)),
                    ("When a PR is closed".into(), Some("Remove the worktree after a grace period".into()), self.sel(t, "wtClosed", &["Right away", "After 1 day", "After 3 days", "After 7 days", "Never"], cx)),
                    ("Inactive worktrees".into(), Some("No activity and no open PR".into()), self.sel(t, "wtIdle", &["After 7 days", "After 14 days", "After 30 days", "After 60 days", "Never"], cx)),
                    ("Keep at most".into(), Some("Oldest inactive worktrees go first".into()), self.sel(t, "wtMax", &["5 per project", "10 per project", "20 per project", "No limit"], cx)),
                    ("Protect uncommitted work".into(), Some("Never remove a worktree with uncommitted changes".into()), self.tgl_pref(t, "wtDirty", cx)),
                ],
            )
            .into_any_element(),
            h3(t, "New worktrees").into_any_element(),
            rows_card(
                t,
                vec![
                    (
                        "Location".into(),
                        Some("Where worktrees are created".into()),
                        div().font_family(MONO).text_size(px(12.)).px(px(8.)).py(px(4.)).rounded(px(6.)).bg(t.surface_active).text_color(t.text_2).child("~/.tako/worktrees/<project>").into_any_element(),
                    ),
                    ("Copy ignored files".into(), Some("Bring .env and other gitignored config into each new worktree".into()), self.tgl_pref(t, "wtEnv", cx)),
                    ("Setup command".into(), Some("Runs once after a worktree is created".into()), self.text_input(t, &self.pages.settings.wt_setup, 200., true, 8., 12.5)),
                ],
            )
            .into_any_element(),
        ]
    }

    fn sec_models(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let effort = self.seg(t, "effort", self.pages.settings.effort, &[("Low", "Low"), ("Medium", "Medium"), ("High", "High"), ("Max", "Max")], |a, v, _, _| a.pages.settings.effort = v, cx);
        let mut out = vec![
            h2(t, "Models & usage").into_any_element(),
            h3(t, "Defaults for new sessions").into_any_element(),
            rows_card(
                t,
                vec![
                    ("Model".into(), None, self.sel(t, "model", &["Opus 5.5", "Sonnet 5.5", "Haiku 5.5", "Codex"], cx)),
                    ("Effort".into(), None, effort.into_any_element()),
                    ("Access".into(), None, self.sel(t, "access", &["Full access", "Ask before edits", "Read-only"], cx)),
                ],
            )
            .into_any_element(),
            h3(t, "Accounts & usage").into_any_element(),
        ];
        let u = &self.overlays.usage;
        for p in PROVS {
            let m = p.color(t);
            let list: Vec<_> = u.accts(p).into_iter().cloned().collect();
            let mut c = card(t).py(px(4.)).mb(px(10.)).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .pt(px(10.))
                    .px(px(18.))
                    .pb(px(8.))
                    .text_size(px(13.5))
                    .child(div().size(px(22.)).rounded(px(6.)).bg(m).flex().items_center().justify_center().child(icon(p.mark(), 13., white())))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(p.name()))
                    .child(div().text_size(px(12.)).text_color(t.text_3).child(format!("{} connected", list.len())))
                    .child(div().flex_1())
                    .child(btn_sm(SharedString::from(format!("uadd-{}", p.name())), t).child(icon("plus", 14., t.text)).child("Add account").on_click(cx.listener(move |this, _, _, cx| {
                        let u = &mut this.overlays.usage;
                        let n = u.accts(p).len() + 1;
                        u.add(p, format!("{} {n}", p.name()), p.plans()[0].to_string());
                        cx.notify();
                    }))),
            );
            for a in list {
                let on = u.is_active(&a);
                let wk = a.week();
                let full = a.capped();
                let (id1, id2) = (a.id.clone(), a.id.clone());
                c = c.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .py(px(10.))
                        .px(px(18.))
                        .border_t_1()
                        .border_color(t.border)
                        .child(
                            div()
                                .id(SharedString::from(format!("uuse-{}", a.id)))
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
                                    this.overlays.usage.use_acct(&id1);
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(3.))
                                .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child(a.label.clone()))
                                .child(div().text_size(px(12.5)).text_color(t.text_3).truncate().child(format!(
                                    "{} · {}",
                                    a.plan,
                                    if full { "limit reached".to_string() } else if on { "in use".into() } else { format!("updated {}", a.synced) }
                                ))),
                        )
                        .child(
                            div()
                                .w(px(150.))
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(div().h(px(5.)).rounded(px(5.)).bg(t.surface_active).overflow_hidden().child(div().h_full().rounded(px(5.)).w(relative(wk.min(100) as f32 / 100.)).bg(if wk >= 100 { t.yellow } else { m })))
                                .child(div().text_size(px(11.5)).text_color(t.text_3).text_right().child(format!("{wk}% this week"))),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("urm-{}", a.id)))
                                .size(px(24.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .hover(|s| s.bg(t.surface_hover))
                                .child(icon("x", 13., t.text_3))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.overlays.usage.remove(&id2);
                                    cx.notify();
                                })),
                        ),
                );
            }
            out.push(c.into_any_element());
        }
        let auto = u.auto;
        out.push(
            card(t)
                .child(row(
                    t,
                    true,
                    "Switch accounts automatically",
                    Some("When one account hits a limit, new sessions move to the next one".into()),
                    tgl(auto, 1., t).id("uauto").on_click(cx.listener(|this, _, _, cx| {
                        let u = &mut this.overlays.usage;
                        u.set_auto(!u.auto);
                        cx.notify();
                    })),
                ))
                .into_any_element(),
        );
        out
    }

    fn sec_labels(&mut self, t: &Theme, _window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = &self.pages.settings;
        let mut c = card(t).overflow_hidden();
        let swatches = |sel_c: u32, editing: bool, cx: &mut Context<Self>| {
            let pc = t.panel_solid;
            div().flex().gap(px(6.)).flex_wrap().children(LABEL_COLORS.iter().enumerate().map(|(i, col)| {
                let on = *col == sel_c;
                let col = *col;
                div()
                    .id(SharedString::from(format!("lc-{i}")))
                    .size(px(16.))
                    .rounded(px(4.))
                    .bg(hsla_hex(col))
                    .when(on, |d| {
                        d.shadow(vec![
                            BoxShadow { color: pc, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false },
                            BoxShadow { color: hsla_hex(col), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(3.5), inset: false },
                        ])
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let s = &mut this.pages.settings;
                        if editing {
                            if let Some(id) = s.edit_label.clone() {
                                if let Some(l) = s.labels.iter_mut().find(|l| l.id == id) {
                                    l.c = col;
                                }
                            }
                        } else {
                            s.new_label_c = i;
                        }
                        cx.notify();
                    }))
            }))
        };
        let labels = st.labels.clone();
        let edit = st.edit_label.clone();
        let adding = st.adding_label;
        let new_c = LABEL_COLORS[st.new_label_c % 8];
        let del_sure = st.del_sure.clone();
        let input = self.text_input(t, &st.label_input, 160., false, 7., 13.);
        let mut input = Some(input);
        for (i, l) in labels.iter().enumerate() {
            let base = div().flex().items_center().gap(px(14.)).w_full().min_h(px(60.)).py(px(12.)).px(px(18.)).when(i > 0, |d| d.border_t_1().border_color(t.border));
            if edit.as_deref() == Some(&l.id) {
                c = c.child(
                    base.flex_wrap()
                        .children(input.take())
                        .child(swatches(l.c, true, cx))
                        .child(div().flex_1())
                        .child(btn_sm("ldone", t).child("Done").on_click(cx.listener(|this, _, window, cx| this.label_commit(window, cx)))),
                );
                continue;
            }
            let uses = self.projects.iter().filter(|p| p.label == Some(l.id.as_str())).count();
            let (id1, id2) = (l.id.clone(), l.id.clone());
            let sure = del_sure.as_deref() == Some(&l.id);
            c = c.child(
                base.child(div().size(px(12.)).rounded(px(3.)).bg(hsla_hex(l.c)).flex_none())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).child(l.name.clone()))
                            .child(div().text_size(px(12.5)).text_color(t.text_3).child(if uses > 0 { format!("{uses} project{}", if uses > 1 { "s" } else { "" }) } else { "Not used yet".into() })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(btn_sm(SharedString::from(format!("ledit-{}", l.id)), t).child("Edit").on_click(cx.listener(move |this, _, window, cx| {
                                let s = &mut this.pages.settings;
                                let name = s.labels.iter().find(|l| l.id == id1).map(|l| l.name.clone()).unwrap_or_default();
                                s.edit_label = Some(id1.clone());
                                s.adding_label = false;
                                s.label_input.update(cx, |s, cx| {
                                    s.set_value(name, window, cx);
                                    s.focus(window, cx);
                                    s.select_all(window, cx);
                                });
                                cx.notify();
                            })))
                            .child(
                                btn_sm(SharedString::from(format!("ldel-{}", l.id)), t)
                                    .px(px(9.))
                                    .when(sure, |d| d.text_color(t.red).border_color(alpha(t.red, 0.4)).child("Delete?"))
                                    .when(!sure, |d| d.child(icon("x", 14., t.text_3)))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let s = &mut this.pages.settings;
                                        if s.del_sure.as_deref() != Some(&id2) {
                                            s.del_sure = Some(id2.clone());
                                        } else {
                                            s.labels.retain(|l| l.id != id2);
                                            s.del_sure = None;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    ),
            );
        }
        let first = labels.is_empty();
        if adding {
            c = c.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(14.))
                    .min_h(px(60.))
                    .py(px(12.))
                    .px(px(18.))
                    .when(!first, |d| d.border_t_1().border_color(t.border))
                    .children(input.take())
                    .child(swatches(new_c, false, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(btn_sm("lcancel", t).child("Cancel").on_click(cx.listener(|this, _, _, cx| {
                                this.pages.settings.adding_label = false;
                                cx.notify();
                            })))
                            .child(btn_sm("lsave", t).bg(t.accent).border_color(transparent_black()).text_color(white()).child("Add").on_click(cx.listener(|this, _, window, cx| this.label_commit(window, cx)))),
                    ),
            );
        } else {
            c = c.child(
                div()
                    .id("ladd")
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .min_h(px(48.))
                    .px(px(18.))
                    .when(!first, |d| d.border_t_1().border_color(t.border))
                    .text_size(px(13.5))
                    .text_color(t.accent)
                    .hover(|s| s.bg(t.surface_hover))
                    .child(icon("plus", 15., t.accent))
                    .child("New label")
                    .on_click(cx.listener(|this, _, window, cx| {
                        let s = &mut this.pages.settings;
                        s.adding_label = true;
                        s.edit_label = None;
                        s.new_label_c = s.labels.len() % 8;
                        s.label_input.update(cx, |s, cx| {
                            s.set_value("", window, cx);
                            s.focus(window, cx);
                        });
                        cx.notify();
                    })),
            );
        }
        vec![
            h2(t, "Labels").into_any_element(),
            note(t, "Labels tag projects and automations. The sidebar can group projects by label.").into_any_element(),
            c.into_any_element(),
        ]
    }

    fn sec_shortcuts(&mut self, t: &Theme) -> Vec<AnyElement> {
        let k = [
            ("New tab", "⌘ T"),
            ("Close tab", "⌘ W"),
            ("Jump to tab 1–9", "⌘ 1 … 9"),
            ("Next / previous tab", "⌘ ⇧ ] / ["),
            ("Home", "⌘ N"),
            ("Search worktrees", "⌘ K"),
            ("Toggle sidebar", "⌘ B"),
            ("Toggle right panel", "⌘ ."),
            ("Commit staged files", "⌘ ↵"),
            ("Close a menu", "Esc"),
        ];
        let kbd = |x: &'static str| {
            div()
                .min_w(px(24.))
                .h(px(24.))
                .px(px(7.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(t.border)
                .bg(t.surface)
                .text_size(px(12.))
                .text_color(t.text_2)
                .child(x)
        };
        vec![
            h2(t, "Keyboard shortcuts").into_any_element(),
            card(t).children(k.iter().enumerate().map(|(i, (n, keys))| row(t, i == 0, *n, None, div().flex().gap(px(6.)).children(keys.split(' ').map(kbd))))).into_any_element(),
        ]
    }

    fn sec_account(&mut self, t: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let armed = self.pages.settings.reset_armed;
        let avatar = div()
            .size(px(40.))
            .flex_none()
            .rounded_full()
            .bg(t.accent)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(15.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(white())
            .child("P");
        vec![
            h2(t, "Account").into_any_element(),
            card(t)
                .child(
                    row_lead(t, true, Some(avatar.into_any_element()), "Your account", Some("Max plan · 2 machines and 1 phone paired".into()), btn_sm("plan", t).child("Manage plan").on_click(cx.listener(|this, _, _, cx| this.st_toast("Plan management would open here", cx)))),
                )
                .into_any_element(),
            h3(t, "Data").into_any_element(),
            card(t)
                .child(row(
                    t,
                    true,
                    "Reset this mockup",
                    Some("Clears projects, labels, tabs, automations and preferences saved in this browser".into()),
                    btn_sm("reset", t).text_color(t.red).border_color(alpha(t.red, 0.4)).child(if armed { "Click again to reset" } else { "Reset" }).on_click(cx.listener(|this, _, window, cx| {
                        if !this.pages.settings.reset_armed {
                            this.pages.settings.reset_armed = true;
                        } else {
                            // Reset settings + usage to their seeds (the mock clears localStorage and reloads).
                            this.pages.settings = SettingsState::new(window, cx);
                            this.pages.settings.sec = "account";
                            this.overlays.usage = crate::overlays::usage::UsageState::new(window, cx);
                            let ts = cx.global_mut::<ThemeState>();
                            ts.pal = "auto".into();
                            ts.light = false;
                            ts.apply();
                        }
                        cx.notify();
                    })),
                ))
                .into_any_element(),
        ]
    }
}

fn hsla_hex(c: u32) -> Hsla {
    rgb(c).into()
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// `settings:<section>` opens the Settings page on that section; `stsel:<key>` opens a select menu.
pub fn act(app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "settings" => {
            app.view = View::Settings;
            if let Some((s, ..)) = SECTIONS.iter().find(|s| s.0 == v) {
                app.pages.settings.sec = s;
            }
        }
        "stsel" => {
            app.pages.settings.open_sel = ["vmSize", "vmRegion", "wtClosed", "wtIdle", "wtMax", "model", "access"].into_iter().find(|x| *x == v);
        }
        _ => return false,
    }
    cx.notify();
    true
}
