//! Automations page (port of the mock's AUTOMATIONS section).
//!
//! Two columns inside the flat `.page` frame: the list (`.au-list`) and the
//! editor (`.au-main`). Every `<select>` of the mock is an `.au-sel` chip that
//! opens a small menu; the time input is a chip with a half-hour menu.
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashSet;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------- helpers

/// CSS `color-mix(in srgb, a f, b)` (premultiplied, like the browser).
pub(crate) fn mix(a: Hsla, b: Hsla, f: f32) -> Hsla {
    let (ra, rb) = (Rgba::from(a), Rgba::from(b));
    let al = ra.a * f + rb.a * (1. - f);
    if al <= 0. {
        return hsla(0., 0., 0., 0.);
    }
    let ch = |x: f32, y: f32| (x * ra.a * f + y * rb.a * (1. - f)) / al;
    Rgba { r: ch(ra.r, rb.r), g: ch(ra.g, rb.g), b: ch(ra.b, rb.b), a: al }.into()
}

pub(crate) fn hexc(v: u32) -> Hsla {
    rgb(v).into()
}

pub(crate) fn transparent() -> Hsla {
    hsla(0., 0., 0., 0.)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Btn {
    Plain,
    Primary,
    Ghost,
    Danger,
    On,
}

/// The mock's `.btn` (+ `.sm`, `.primary`, `.ghost`, `.danger`, `.on`).
pub(crate) fn btn(id: impl Into<ElementId>, kind: Btn, sm: bool, disabled: bool, cx: &App) -> Stateful<Div> {
    let t = cx.t();
    let (bg, border, fg) = match kind {
        Btn::Plain => (t.surface, t.border, t.text),
        Btn::Primary => (t.accent, transparent(), white()),
        Btn::Ghost => (t.surface, t.border, t.text_3),
        Btn::Danger => (t.surface, mix(t.red, transparent(), 0.4), t.red),
        Btn::On => (mix(t.green, transparent(), 0.08), mix(t.green, transparent(), 0.35), t.green),
    };
    let hover_bg = match kind {
        Btn::Primary => hsla(bg.h, bg.s, (bg.l * 1.08).min(1.), bg.a),
        Btn::On => bg,
        _ => t.surface_hover,
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(7.))
        .h(px(if sm { 28. } else { 32. }))
        .px(px(match (kind, sm) {
            (Btn::Ghost, _) => 9.,
            (_, true) => 11.,
            _ => 14.,
        }))
        .rounded(px(8.))
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_color(fg)
        .text_size(px(if sm { 12.5 } else { 13. }))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .when(disabled, |d| d.opacity(0.45))
        .when(!disabled, |d| d.cursor_pointer().hover(move |s| s.bg(hover_bg)))
}

/// `.ib` — a square icon button.
pub(crate) fn ib(id: impl Into<ElementId>, ic: &'static str, size: f32, icon_size: f32, cx: &App) -> Stateful<Div> {
    let t = cx.t();
    div()
        .id(id)
        .size(px(size))
        .flex_none()
        .rounded(px(7.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(t.surface_hover))
        .child(icon(ic, icon_size, t.text_3))
}

/// The floating menu surface (`.au-menu`).
pub(crate) fn menu_panel(cx: &App) -> Div {
    let t = cx.t();
    div()
        .flex()
        .flex_col()
        .p(px(4.))
        .rounded(px(10.))
        .border_1()
        .border_color(t.border)
        .bg(t.panel_solid)
        .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.2), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
        .occlude()
}

pub(crate) fn menu_item(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    let t = cx.t();
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(10.))
        .py(px(8.))
        .rounded(px(7.))
        .text_size(px(13.))
        .whitespace_nowrap()
        .cursor_pointer()
        .hover(|s| s.bg(t.surface_hover))
}

// ---------------------------------------------------------------- data

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum TrigKind {
    Schedule,
    Pr,
    Ci,
    Issue,
    Webhook,
}
const TRIG_KINDS: [TrigKind; 5] = [TrigKind::Schedule, TrigKind::Pr, TrigKind::Ci, TrigKind::Issue, TrigKind::Webhook];

impl TrigKind {
    fn icon(self) -> &'static str {
        match self {
            TrigKind::Schedule => "clock",
            TrigKind::Pr => "pr",
            TrigKind::Ci => "x",
            TrigKind::Issue => "list",
            TrigKind::Webhook => "globe",
        }
    }
    fn label(self) -> &'static str {
        match self {
            TrigKind::Schedule => "Schedule",
            TrigKind::Pr => "Pull request opened",
            TrigKind::Ci => "CI failed",
            TrigKind::Issue => "New issue",
            TrigKind::Webhook => "Webhook",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Trigger {
    kind: TrigKind,
    every: Option<String>,
    at: Option<String>,
    branch: Option<String>,
    label: Option<String>,
}

impl Trigger {
    fn new(kind: TrigKind) -> Self {
        Trigger { kind, every: None, at: None, branch: None, label: None }
    }
    fn every(&self) -> String {
        self.every.clone().unwrap_or("Every weekday".into())
    }
    fn at(&self) -> String {
        self.at.clone().unwrap_or("09:00".into())
    }
    fn branch(&self) -> String {
        self.branch.clone().unwrap_or("main".into())
    }
    fn label(&self) -> String {
        self.label.clone().unwrap_or("bug".into())
    }
    fn sum(&self) -> String {
        match self.kind {
            TrigKind::Schedule => format!("{} at {}", self.every(), self.at()),
            TrigKind::Pr => "PR opened in tako".into(),
            TrigKind::Ci => format!("CI fails on {}", self.branch()),
            TrigKind::Issue => format!("New issue labelled {}", self.label()),
            TrigKind::Webhook => "POST /hooks/a7f3c2".into(),
        }
    }
    fn get(&self, k: &str) -> String {
        match k {
            "every" => self.every(),
            "at" => self.at(),
            "branch" => self.branch(),
            _ => self.label(),
        }
    }
    fn set(&mut self, k: &str, v: String) {
        match k {
            "every" => self.every = Some(v),
            "at" => self.at = Some(v),
            "branch" => self.branch = Some(v),
            _ => self.label = Some(v),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Run {
    ok: bool,
    when: String,
    out: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Automation {
    id: String,
    name: String,
    active: bool,
    triggers: Vec<Trigger>,
    instructions: String,
    model: String,
    effort: String,
    mode: String,
    working: String,
    conversation: String,
    label: String,
    repo: String,
    catchup: String,
    notify: String,
    timeout: String,
    runs: Vec<Run>,
}

impl Automation {
    fn blank(id: String) -> Self {
        Automation {
            id,
            name: "Untitled".into(),
            active: true,
            triggers: vec![],
            instructions: String::new(),
            model: "Opus 5.5".into(),
            effort: "High".into(),
            mode: "Auto".into(),
            working: "Fresh worktree".into(),
            conversation: "Start fresh".into(),
            label: String::new(),
            repo: "tako".into(),
            catchup: "Don't catch up".into(),
            notify: "On failure".into(),
            timeout: "30 minutes".into(),
            runs: vec![],
        }
    }
    fn field(&mut self, k: &str) -> &mut String {
        match k {
            "model" => &mut self.model,
            "effort" => &mut self.effort,
            "mode" => &mut self.mode,
            "working" => &mut self.working,
            "conversation" => &mut self.conversation,
            "label" => &mut self.label,
            "repo" => &mut self.repo,
            "catchup" => &mut self.catchup,
            "notify" => &mut self.notify,
            _ => &mut self.timeout,
        }
    }
    fn get(&self, k: &str) -> String {
        self.clone().field(k).clone()
    }
}

fn options(key: &str) -> &'static [&'static str] {
    match key {
        "working" => &["Fresh worktree", "This repo"],
        "conversation" => &["Start fresh", "Continue last run"],
        "repo" => &["tako", "api", "web"],
        "model" => &["Opus 5.5", "Sonnet 5.5", "Haiku 5.5", "Codex"],
        "effort" => &["Low", "Medium", "High"],
        "mode" => &["Auto", "Ask first", "Read-only"],
        "catchup" => &["Don't catch up", "Within 1 hour", "Within 6 hours", "Within 24 hours"],
        "notify" => &["On failure", "Always", "Never"],
        "timeout" => &["10 minutes", "30 minutes", "2 hours"],
        "every" => &["Every weekday", "Every day", "Every hour", "Every Monday"],
        "branch" => &["main", "release/*", "any branch"],
        "label" => &["bug", "good first issue", "any label"],
        _ => &[],
    }
}

/// App-wide labels (the mock's `window.forgeLabels` defaults).
#[derive(Clone)]
pub(crate) struct Lbl {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) c: u32,
}
const LABEL_COLORS: [u32; 8] = [0x7aa7e8, 0x5fc48a, 0xe2b75a, 0xc77dff, 0xec7aa0, 0x4fb3c8, 0xe5675f, 0x9aa3b0];

fn seed() -> Vec<Automation> {
    let r = |ok: bool, when: &str, out: &str| Run { ok, when: when.into(), out: out.into() };
    let mut a1 = Automation::blank("a1".into());
    a1.name = "Nightly dependency bumps".into();
    a1.triggers = vec![Trigger { every: Some("Every day".into()), at: Some("02:00".into()), ..Trigger::new(TrigKind::Schedule) }];
    a1.instructions = "Update minor and patch dependencies, run `bun run check`, and open one PR with a summary of what changed. Skip anything that needs a major bump.".into();
    a1.label = "maint".into();
    a1.runs = vec![r(true, "Today 02:00", "Opened PR #2161"), r(true, "Yesterday 02:00", "No updates"), r(false, "Mon 02:00", "Typecheck failed")];
    let mut a2 = Automation::blank("a2".into());
    a2.name = "Triage new bugs".into();
    a2.triggers = vec![Trigger { label: Some("bug".into()), ..Trigger::new(TrigKind::Issue) }];
    a2.instructions = "Reproduce the issue in a fresh worktree. If you can, add a failing test and a fix; otherwise comment with what you found and label it `needs-info`.".into();
    a2.model = "Sonnet 5.5".into();
    a2.effort = "Medium".into();
    a2.runs = vec![r(true, "Today 11:42", "ENG-482 → PR #2158")];
    let mut a3 = Automation::blank("a3".into());
    a3.name = "Fix red CI on main".into();
    a3.active = false;
    a3.triggers = vec![Trigger { branch: Some("main".into()), ..Trigger::new(TrigKind::Ci) }];
    a3.instructions = "Find the failing job, identify the commit that broke it, and open a PR that fixes it. Tag the commit author for review.".into();
    a3.conversation = "Continue last run".into();
    vec![a1, a2, a3]
}

/// Which chip menu is open.
#[derive(Clone, PartialEq, Debug)]
enum Menu {
    Field(&'static str),
    Trig(usize, &'static str),
    Time(usize),
    AddTrig,
}

pub struct AutoState {
    pub(crate) autos: Vec<Automation>,
    sel: Option<String>,
    draft: Option<Automation>,
    pub(crate) next_id: usize,
    menu: Option<Menu>,
    menu_closed: Option<(Menu, Instant)>,
    adv_open: bool,
    sure_delete: bool,
    running: HashSet<String>,
    toggle_tween: Option<Tween>,
    pub(crate) labels: Vec<Lbl>,
    adding_label: bool,
    lb_color: usize,
    filter: Entity<InputState>,
    name: Entity<InputState>,
    ins: Entity<TextareaState>,
    lb_name: Entity<InputState>,
    syncing: bool,
}

impl AutoState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter automations"));
        let name = cx.new(|cx| InputState::new(window, cx));
        let ins = cx.new(|cx| TextareaState::new(window, cx).auto_grow(5, 30).placeholder("Tell the agent what to do when this automation runs…"));
        let lb_name = cx.new(|cx| InputState::new(window, cx).placeholder("Label name"));
        cx.subscribe_in(&filter, window, |_: &mut TakoApp, _, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&name, window, |this: &mut TakoApp, st, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                if this.pages.auto.syncing {
                    return;
                }
                let v = st.read(cx).value().to_string();
                if let Some(a) = this.pages.auto.cur_mut() {
                    a.name = v;
                }
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&ins, window, |this: &mut TakoApp, st, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                if this.pages.auto.syncing {
                    return;
                }
                let v = st.read(cx).value().to_string();
                if let Some(a) = this.pages.auto.cur_mut() {
                    a.instructions = v;
                }
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&lb_name, window, |this: &mut TakoApp, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.add_label(window, cx);
            }
        })
        .detach();
        let lbl = |id: &str, name: &str, c: u32| Lbl { id: id.into(), name: name.into(), c };
        let mut s = AutoState {
            autos: seed(),
            sel: Some("a1".into()),
            draft: None,
            next_id: 4,
            menu: None,
            menu_closed: None,
            adv_open: false,
            sure_delete: false,
            running: HashSet::new(),
            toggle_tween: None,
            labels: vec![
                lbl("work", "Work", 0x7aa7e8),
                lbl("personal", "Personal", 0x5fc48a),
                lbl("oss", "Open source", 0xe2b75a),
                lbl("client", "Client", 0xc77dff),
                lbl("maint", "Maintenance", 0x9aa3b0),
            ],
            adding_label: false,
            lb_color: 0,
            filter,
            name,
            ins,
            lb_name,
            syncing: false,
        };
        s.sync_inputs(window, cx);
        s
    }

    fn cur(&self) -> Option<&Automation> {
        self.draft.as_ref().or_else(|| self.autos.iter().find(|a| Some(&a.id) == self.sel.as_ref()))
    }
    fn cur_mut(&mut self) -> Option<&mut Automation> {
        if self.draft.is_some() {
            return self.draft.as_mut();
        }
        let sel = self.sel.clone();
        self.autos.iter_mut().find(|a| Some(&a.id) == sel.as_ref())
    }

    /// Push the selected automation's name/instructions into the inputs.
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<TakoApp>) {
        let (n, i) = self.cur().map(|a| (a.name.clone(), a.instructions.clone())).unwrap_or_default();
        self.syncing = true;
        self.name.update(cx, |s, cx| s.set_value(n, window, cx));
        self.ins.update(cx, |s, cx| s.set_value(i, window, cx));
        self.syncing = false;
    }

    fn label(&self, id: &str) -> Option<&Lbl> {
        self.labels.iter().find(|l| l.id == id)
    }
}

// ---------------------------------------------------------------- behaviour

impl TakoApp {
    fn au_pick(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        s.draft = None;
        s.sel = Some(id);
        s.menu = None;
        s.sure_delete = false;
        s.adding_label = false;
        s.toggle_tween = None;
        s.sync_inputs(window, cx);
        cx.notify();
    }

    fn au_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        s.draft = Some(Automation::blank(format!("a{}", s.next_id)));
        s.next_id += 1;
        s.menu = None;
        s.sure_delete = false;
        s.adding_label = false;
        s.toggle_tween = None;
        s.sync_inputs(window, cx);
        let name = s.name.clone();
        name.update(cx, |st, cx| {
            st.focus(window, cx);
            st.select_all(window, cx);
        });
        cx.notify();
    }

    fn au_set(&mut self, m: &Menu, v: String, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        s.menu = None;
        if *m == Menu::Field("label") && v == "__new" {
            s.adding_label = true;
            s.lb_color = s.labels.len() % LABEL_COLORS.len();
            let st = s.lb_name.clone();
            st.update(cx, |st, cx| {
                st.set_value("", window, cx);
                st.focus(window, cx);
            });
            cx.notify();
            return;
        }
        if let Some(a) = s.cur_mut() {
            match m {
                Menu::Field(k) => *a.field(k) = v,
                Menu::Trig(i, k) => {
                    if let Some(tr) = a.triggers.get_mut(*i) {
                        tr.set(k, v)
                    }
                }
                Menu::Time(i) => {
                    if let Some(tr) = a.triggers.get_mut(*i) {
                        tr.at = Some(v)
                    }
                }
                Menu::AddTrig => {}
            }
        }
        cx.notify();
    }

    fn add_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        let name = s.lb_name.read(cx).value().trim().to_string();
        if name.is_empty() {
            let st = s.lb_name.clone();
            st.update(cx, |st, cx| st.focus(window, cx));
            return;
        }
        let id = format!("l{}", s.labels.len());
        s.labels.push(Lbl { id: id.clone(), name: name.chars().take(24).collect(), c: LABEL_COLORS[s.lb_color] });
        if let Some(a) = s.cur_mut() {
            a.label = id;
        }
        s.adding_label = false;
        cx.notify();
    }

    fn au_toggle_menu(&mut self, m: Menu, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        // the click that closed it from outside shouldn't reopen it
        if let Some((c, at)) = &s.menu_closed {
            if *c == m && at.elapsed() < Duration::from_millis(250) {
                s.menu_closed = None;
                return;
            }
        }
        s.menu = if s.menu.as_ref() == Some(&m) { None } else { Some(m) };
        cx.notify();
    }

    fn au_run(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.pages.auto.sel.clone() else { return };
        if !self.pages.auto.running.insert(id.clone()) {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(1800)).await;
            this.update(cx, |this, cx| {
                let s = &mut this.pages.auto;
                s.running.remove(&id);
                if let Some(a) = s.autos.iter_mut().find(|a| a.id == id) {
                    a.runs.insert(0, Run { ok: true, when: "Just now".into(), out: "Run finished · opened a worktree".into() });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn au_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.auto;
        if !s.sure_delete {
            s.sure_delete = true;
            cx.notify();
            return;
        }
        let sel = s.sel.clone();
        s.autos.retain(|a| Some(&a.id) != sel.as_ref());
        s.sel = s.autos.first().map(|a| a.id.clone());
        s.sure_delete = false;
        s.sync_inputs(window, cx);
        cx.notify();
    }

    // ------------------------------------------------------------ render

    pub fn render_automations(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        // toggle knob animation
        if let Some(tw) = &self.pages.auto.toggle_tween {
            if tw.done() {
                self.pages.auto.toggle_tween = None;
            } else {
                window.request_animation_frame();
            }
        }
        div().size_full().flex().flex_col().child(div().h(px(44.)).flex_none()).child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .border_t_1()
                .border_l_1()
                .border_color(t.border)
                .rounded_tl(px(12.))
                .bg(t.bg)
                .overflow_hidden()
                .child(self.au_list(cx))
                .child(self.au_main(window, cx)),
        )
    }

    fn au_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let s = &self.pages.auto;
        let q = s.filter.read(cx).value().to_lowercase();
        let on_id = if s.draft.is_some() { None } else { s.sel.clone() };
        let mut rows: Vec<AnyElement> = vec![];
        for (ix, x) in s.autos.iter().filter(|x| q.is_empty() || x.name.to_lowercase().contains(&q)).enumerate() {
            let on = Some(&x.id) == on_id.as_ref();
            let id = x.id.clone();
            let lbl = s.label(&x.label).cloned();
            let sum = x.triggers.first().map(|tr| tr.sum()).unwrap_or("No trigger".into());
            rows.push(
                div()
                    .id(("au-item", ix))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .w_full()
                    .px(px(10.))
                    .py(px(9.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.surface_active))
                    .when(!on, |d| d.hover(|s| s.bg(t.surface_hover)))
                    .child(div().size(px(7.)).rounded_full().flex_none().when(x.active, |d| d.bg(t.green)).when(!x.active, |d| d.bg(t.text_3).opacity(0.5)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).truncate().child(x.name.clone()))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .text_color(t.text_3)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(sum)
                                    .when_some(lbl, |d, l| {
                                        d.child(" · ").child(div().size(px(7.)).rounded(px(2.)).bg(hexc(l.c)).ml(px(2.)).mr(px(4.)).flex_none()).child(l.name)
                                    }),
                            ),
                    )
                    .when_some(x.runs.first(), |d, r| {
                        d.child(
                            div()
                                .flex_none()
                                .font_family(MONO)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if r.ok { t.green } else { t.red })
                                .child(if r.ok { "✓" } else { "✗" }),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.au_pick(id.clone(), window, cx)))
                    .into_any_element(),
            );
        }
        if rows.is_empty() {
            rows.push(
                div()
                    .px(px(14.))
                    .py(px(24.))
                    .text_size(px(13.))
                    .text_color(t.text_3)
                    .child(if q.is_empty() { "No automations yet" } else { "No automations match" })
                    .into_any_element(),
            );
        }
        if let Some(d) = &s.draft {
            rows.push(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(10.))
                    .py(px(9.))
                    .rounded(px(8.))
                    .bg(t.surface_active)
                    .child(div().size(px(7.)).rounded_full().flex_none().bg(t.text_3).opacity(0.5))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).italic().text_color(t.text_2).truncate().child(d.name.clone()))
                            .child(div().text_size(px(12.)).text_color(t.text_3).child("Draft")),
                    )
                    .into_any_element(),
            );
        }
        div()
            .w(px(300.))
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(t.border)
            .flex()
            .flex_col()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .h(px(48.))
                    .pl(px(16.))
                    .pr(px(10.))
                    .border_b_1()
                    .border_color(t.border)
                    .child(icon("search", 15., t.text_3))
                    .child(div().flex_1().min_w_0().child(Input::new(&s.filter).appearance(false).bordered(false).text_size(px(13.5)).p_0()))
                    .child(ib("au-new", "plus", 28., 16., cx).on_click(cx.listener(|this, _, window, cx| this.au_new(window, cx)))),
            )
            .child(div().id("au-items").flex_1().min_h_0().overflow_y_scroll().p(px(8.)).flex().flex_col().gap(px(2.)).children(rows))
    }

    /// An `.au-sel` chip with its dropdown.
    fn au_select(&self, m: Menu, content: AnyElement, opts: Vec<(String, AnyElement)>, cur: String, small: bool, right: bool, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let open = self.pages.auto.menu.as_ref() == Some(&m);
        let id = format!("{m:?}");
        let mm = m.clone();
        let chip = div()
            .id(SharedString::from(format!("sel-{id}")))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .h(px(if small { 28. } else { 30. }))
            .pl(px(12.))
            .pr(px(10.))
            .rounded(px(7.))
            .bg(t.surface_active)
            .text_size(px(if small { 12.5 } else { 13. }))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(|s| s.bg(mix(t.text, transparent(), 0.12)))
            .child(content)
            .child(icon("chev-d", 12., t.text_3))
            .on_click(cx.listener(move |this, _, _, cx| this.au_toggle_menu(mm.clone(), cx)));
        let mut wrap = div().flex().flex_col().flex_none().when(right, |d| d.items_end()).child(chip);
        if open {
            let items: Vec<AnyElement> = opts
                .into_iter()
                .enumerate()
                .map(|(i, (val, label))| {
                    let sel = val == cur;
                    let mm = m.clone();
                    menu_item((SharedString::from(format!("opt-{id}")), i), cx)
                        .pr(px(14.))
                        .child(div().w(px(14.)).flex_none().when(sel, |d| d.child(icon("check", 14., t.text_2))))
                        .child(label)
                        .on_click(cx.listener(move |this, _, window, cx| this.au_set(&mm, val.clone(), window, cx)))
                        .into_any_element()
                })
                .collect();
            let mc = m.clone();
            let panel = menu_panel(cx)
                .id(SharedString::from(format!("menu-{id}")))
                .mt(px(4.))
                .min_w(px(160.))
                .max_h(px(280.))
                .overflow_y_scroll()
                .children(items)
                .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                    let s = &mut this.pages.auto;
                    if s.menu.as_ref() == Some(&mc) {
                        s.menu = None;
                        s.menu_closed = Some((mc.clone(), Instant::now()));
                        cx.notify();
                    }
                }));
            wrap = wrap.child(div().size_0().relative().child(deferred(anchored().anchor(if right { Anchor::TopRight } else { Anchor::TopLeft }).snap_to_window_with_margin(px(8.)).child(panel)).with_priority(1)));
        }
        wrap.into_any_element()
    }

    fn sel_simple(&self, m: Menu, key: &str, val: String, small: bool, right: bool, cx: &mut Context<Self>) -> AnyElement {
        let opts = options(key).iter().map(|o| (o.to_string(), o.to_string().into_any_element())).collect();
        self.au_select(m, val.clone().into_any_element(), opts, val, small, right, cx)
    }

    fn au_main(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let Some(a) = self.pages.auto.cur().cloned() else {
            return div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .p(px(40.))
                .text_color(t.text_3)
                .child(div().mb(px(6.)).child(icon("bolt", 26., t.accent)))
                .child(div().text_color(t.text).font_weight(FontWeight::MEDIUM).text_size(px(15.)).child("No automation selected"))
                .child(div().max_w(px(360.)).mb(px(10.)).text_center().child("Automations run an agent on a schedule or when something happens in your repos."))
                .child(
                    btn("au-none-new", Btn::Primary, false, false, cx)
                        .child(icon("plus", 14., white()))
                        .child("New automation")
                        .on_click(cx.listener(|this, _, window, cx| this.au_new(window, cx))),
                )
                .into_any_element();
        };
        let s = &self.pages.auto;
        let is_draft = s.draft.is_some();
        let can_save = !a.triggers.is_empty() && !a.instructions.trim().is_empty();
        let running = s.running.contains(&a.id);
        let name_focused = s.name.read(cx).focus_handle(cx).is_focused(window);

        // ---- header
        let actions = if is_draft {
            div()
                .flex()
                .gap(px(8.))
                .flex_none()
                .child(btn("au-cancel", Btn::Plain, false, false, cx).child("Cancel").on_click(cx.listener(|this, _, window, cx| {
                    this.pages.auto.draft = None;
                    this.pages.auto.sync_inputs(window, cx);
                    cx.notify();
                })))
                .child(btn("au-create", Btn::Primary, false, !can_save, cx).child("Create").when(can_save, |d| {
                    d.on_click(cx.listener(|this, _, _, cx| {
                        let s = &mut this.pages.auto;
                        if let Some(d) = s.draft.take() {
                            s.sel = Some(d.id.clone());
                            s.autos.insert(0, d);
                        }
                        cx.notify();
                    }))
                }))
        } else {
            let sure = s.sure_delete;
            div()
                .flex()
                .gap(px(8.))
                .flex_none()
                .child(
                    btn("au-run", Btn::Plain, false, running, cx)
                        .when(running, |d| d.child(spinner("au-run-spin", 12., t.text_3)).child("Running"))
                        .when(!running, |d| d.child(icon("play", 14., t.text)).child("Run now"))
                        .on_click(cx.listener(|this, _, _, cx| this.au_run(cx))),
                )
                .child(
                    btn("au-del", if sure { Btn::Danger } else { Btn::Ghost }, false, false, cx)
                        .when(sure, |d| d.child("Delete?"))
                        .when(!sure, |d| d.child(icon("x", 14., t.text_3)))
                        .on_click(cx.listener(|this, _, window, cx| this.au_delete(window, cx))),
                )
        };
        let knob = {
            let e = s.toggle_tween.as_ref().map(|tw| ease_snappy(tw.raw())).unwrap_or(1.);
            if a.active { e } else { 1. - e }
        };
        let toggle = div()
            .id("au-toggle")
            .w(px(36.))
            .h(px(20.))
            .rounded_full()
            .flex_none()
            .relative()
            .cursor_pointer()
            .bg(mix(t.accent, t.surface_active, knob))
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .left(px(2. + 16. * knob))
                    .size(px(16.))
                    .rounded_full()
                    .bg(white())
                    .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.3), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false }]),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                let s = &mut this.pages.auto;
                if let Some(a) = s.cur_mut() {
                    a.active = !a.active;
                }
                s.toggle_tween = Some(Tween::new(200.));
                cx.notify();
            }));
        let head = div()
            .pt(px(26.))
            .px(px(40.))
            .pb(px(22.))
            .border_b_1()
            .border_color(t.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .py(px(2.))
                            .border_b_1()
                            .border_color(if name_focused { t.border } else { transparent() })
                            .child(Input::new(&s.name).appearance(false).bordered(false).p_0().text_size(px(26.)).font_weight(FontWeight::SEMIBOLD)),
                    )
                    .child(actions),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .mt(px(14.))
                    .text_size(px(13.))
                    .text_color(t.text_2)
                    .child(toggle)
                    .child(if a.active { "Active" } else { "Paused" })
                    .when_some(a.runs.first().filter(|_| !is_draft), |d, r| {
                        d.child(div().w(px(1.)).h(px(14.)).bg(t.border)).child(div().text_color(t.text_3).child(format!("Last run {}", r.when)))
                    }),
            );

        // ---- body
        let h3 = |s: &'static str| div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(t.text_2).mt(px(26.)).mb(px(10.)).mx(px(2.)).child(s);
        let card = || div().border_1().border_color(t.border).rounded(px(10.)).bg(t.panel_solid).flex().flex_col();
        let row = |first: bool| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .w_full()
                .px(px(18.))
                .py(px(14.))
                .min_h(px(64.))
                .when(!first, |d| d.border_t_1().border_color(t.border))
        };
        let rt = |b: SharedString, sm: SharedString| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(div().text_size(px(13.5)).font_weight(FontWeight::MEDIUM).text_color(t.text).child(b))
                .child(div().text_size(px(12.5)).text_color(t.text_3).child(sm))
        };

        // triggers
        let mut trig_rows: Vec<AnyElement> = vec![];
        for (i, tr) in a.triggers.iter().enumerate() {
            let mut r = row(i == 0).child(icon(tr.kind.icon(), 16., t.accent)).child(rt(tr.kind.label().into(), tr.sum().into()));
            match tr.kind {
                TrigKind::Schedule => {
                    r = r.child(self.sel_simple(Menu::Trig(i, "every"), "every", tr.every(), false, true, cx));
                    r = r.child(self.time_chip(i, tr.at(), cx));
                }
                TrigKind::Ci => r = r.child(self.sel_simple(Menu::Trig(i, "branch"), "branch", tr.get("branch"), false, true, cx)),
                TrigKind::Issue => r = r.child(self.sel_simple(Menu::Trig(i, "label"), "label", tr.get("label"), false, true, cx)),
                _ => {}
            }
            r = r.child(ib(("rmtrig", i), "x", 26., 13., cx).on_click(cx.listener(move |this, _, _, cx| {
                if let Some(a) = this.pages.auto.cur_mut() {
                    if i < a.triggers.len() {
                        a.triggers.remove(i);
                    }
                }
                cx.notify();
            })));
            trig_rows.push(r.into_any_element());
        }
        let menu_open = self.pages.auto.menu == Some(Menu::AddTrig);
        let add = div()
            .flex()
            .flex_col()
            .when(!a.triggers.is_empty(), |d| d.border_t_1().border_color(t.border))
            .child(
                div()
                    .id("au-add")
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .w_full()
                    .px(px(18.))
                    .py(px(16.))
                    .text_size(px(13.5))
                    .text_color(t.text_2)
                    .rounded(px(10.))
                    .cursor_pointer()
                    .hover(|s| s.text_color(t.text).bg(t.surface_hover))
                    .child(icon("plus", 15., t.text_2))
                    .child("Add Trigger")
                    .on_click(cx.listener(|this, _, _, cx| this.au_toggle_menu(Menu::AddTrig, cx))),
            )
            .when(menu_open, |d| {
                let items: Vec<AnyElement> = TRIG_KINDS
                    .iter()
                    .enumerate()
                    .map(|(i, k)| {
                        let k = *k;
                        menu_item(("trig", i), cx)
                            .child(icon(k.icon(), 15., t.text_2))
                            .child(k.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let s = &mut this.pages.auto;
                                s.menu = None;
                                if let Some(a) = s.cur_mut() {
                                    a.triggers.push(Trigger::new(k));
                                }
                                cx.notify();
                            }))
                            .into_any_element()
                    })
                    .collect();
                let panel = menu_panel(cx).min_w(px(230.)).ml(px(12.)).mt(px(-6.)).children(items).on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    let s = &mut this.pages.auto;
                    if s.menu == Some(Menu::AddTrig) {
                        s.menu = None;
                        s.menu_closed = Some((Menu::AddTrig, Instant::now()));
                        cx.notify();
                    }
                }));
                d.child(div().size_0().relative().child(deferred(anchored().snap_to_window_with_margin(px(8.)).child(panel)).with_priority(1)))
            });
        let triggers = card().children(trig_rows).child(add);

        // instructions
        let chips = div()
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .px(px(12.))
            .pt(px(8.))
            .pb(px(12.))
            .child(self.sel_simple(Menu::Field("model"), "model", a.model.clone(), true, false, cx))
            .child(self.sel_simple(Menu::Field("effort"), "effort", a.effort.clone(), true, false, cx))
            .child(self.sel_simple(Menu::Field("mode"), "mode", a.mode.clone(), true, false, cx));
        let ins = card()
            .child(
                div()
                    .min_h(px(140.))
                    .px(px(8.))
                    .pt(px(6.))
                    .pb(px(0.))
                    .child(Textarea::new(&self.pages.auto.ins).appearance(false).bordered(false).p_0().text_size(px(14.)).line_height(relative(1.55))),
            )
            .child(chips);

        // session
        let session = card()
            .child(row(true).child(rt("Repository".into(), "Where the agent works".into())).child(self.sel_simple(Menu::Field("repo"), "repo", a.repo.clone(), false, true, cx)))
            .child(row(false).child(rt("Working copy".into(), "This repo, or a fresh worktree".into())).child(self.sel_simple(Menu::Field("working"), "working", a.working.clone(), false, true, cx)))
            .child(
                row(false)
                    .child(rt("Conversation".into(), "New chat, or continue the last run".into()))
                    .child(self.sel_simple(Menu::Field("conversation"), "conversation", a.conversation.clone(), false, true, cx)),
            )
            .child(row(false).child(rt("Label".into(), "Tag runs so they're easy to find in the sidebar".into())).child(self.label_pick(&a, cx)));

        // advanced
        let adv_open = self.pages.auto.adv_open;
        let adv = card()
            .mt(px(26.))
            .child(
                row(true)
                    .id("au-adv")
                    .cursor_pointer()
                    .child(rt("Advanced".into(), "Catch-up window, notifications and time limit".into()))
                    .child(icon("chev-d", 15., t.text_3).when(adv_open, |s| s.with_transformation(Transformation::rotate(radians(std::f32::consts::PI)))))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pages.auto.adv_open = !this.pages.auto.adv_open;
                        cx.notify();
                    })),
            )
            .when(adv_open, |d| {
                d.child(row(false).child(rt("Catch-up window".into(), "Run once if the app was closed at trigger time".into())).child(self.sel_simple(Menu::Field("catchup"), "catchup", a.catchup.clone(), false, true, cx)))
                    .child(row(false).child(rt("Notify me".into(), "When a run finishes".into())).child(self.sel_simple(Menu::Field("notify"), "notify", a.notify.clone(), false, true, cx)))
                    .child(row(false).child(rt("Time limit".into(), "Stop runs that take longer".into())).child(self.sel_simple(Menu::Field("timeout"), "timeout", a.timeout.clone(), false, true, cx)))
            });

        // recent runs
        let runs = (!is_draft && !a.runs.is_empty()).then(|| {
            div().child(h3("Recent runs")).child(card().children(a.runs.iter().enumerate().map(|(i, r)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(18.))
                    .py(px(11.))
                    .when(i > 0, |d| d.border_t_1().border_color(t.border))
                    .child(div().flex_none().font_family(MONO).text_size(px(11.5)).font_weight(FontWeight::SEMIBOLD).text_color(if r.ok { t.green } else { t.red }).child(if r.ok { "✓" } else { "✗" }))
                    .child(rt(r.out.clone().into(), r.when.clone().into()))
            })))
        });

        let body = div()
            .pt(px(8.))
            .px(px(40.))
            .pb(px(60.))
            .max_w(px(980.))
            .child(h3("Triggers"))
            .child(triggers)
            .child(h3("Instructions"))
            .child(ins)
            .child(div().mt(px(10.)).mx(px(2.)).text_size(px(12.5)).text_color(t.text_3).child("Skills, @file references, and built-in commands work here."))
            .child(h3("Session"))
            .child(session)
            .child(adv)
            .children(runs);

        div().id("au-main").flex_1().min_w_0().h_full().overflow_y_scroll().child(head).child(body).into_any_element()
    }

    /// The `<input type=time>` — a chip with a half-hour menu.
    fn time_chip(&self, i: usize, at: String, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let fmt12 = |s: &str| {
            let (h, m) = s.split_once(':').unwrap_or(("9", "00"));
            let h: u32 = h.parse().unwrap_or(9);
            format!("{:02}:{} {}", if h % 12 == 0 { 12 } else { h % 12 }, m, if h < 12 { "AM" } else { "PM" })
        };
        let m = Menu::Time(i);
        let open = self.pages.auto.menu.as_ref() == Some(&m);
        let mm = m.clone();
        let chip = div()
            .id(("au-time", i))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(30.))
            .px(px(8.))
            .rounded(px(7.))
            .bg(t.surface_active)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .child(fmt12(&at))
            .child(icon("clock", 13., t.text_3))
            .on_click(cx.listener(move |this, _, _, cx| this.au_toggle_menu(mm.clone(), cx)));
        let mut wrap = div().flex().flex_col().flex_none().items_end().child(chip);
        if open {
            let items: Vec<AnyElement> = (0..48usize)
                .map(|k| {
                    let v = format!("{:02}:{:02}", k / 2, (k % 2) * 30);
                    let sel = v == at;
                    let mm = m.clone();
                    menu_item(("time-opt", k), cx)
                        .child(div().w(px(14.)).flex_none().when(sel, |d| d.child(icon("check", 14., t.text_2))))
                        .child(fmt12(&v))
                        .on_click(cx.listener(move |this, _, window, cx| this.au_set(&mm, v.clone(), window, cx)))
                        .into_any_element()
                })
                .collect();
            let mc = m.clone();
            let panel = menu_panel(cx).id("time-menu").mt(px(4.)).min_w(px(130.)).max_h(px(280.)).overflow_y_scroll().children(items).on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                let s = &mut this.pages.auto;
                if s.menu.as_ref() == Some(&mc) {
                    s.menu = None;
                    s.menu_closed = Some((mc.clone(), Instant::now()));
                    cx.notify();
                }
            }));
            wrap = wrap.child(div().size_0().relative().child(deferred(anchored().anchor(Anchor::TopRight).snap_to_window_with_margin(px(8.)).child(panel)).with_priority(1)));
        }
        wrap.into_any_element()
    }

    fn label_pick(&self, a: &Automation, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let s = &self.pages.auto;
        if s.adding_label {
            let cols = div().flex().gap(px(6.)).children(LABEL_COLORS.iter().enumerate().map(|(i, c)| {
                let on = i == s.lb_color;
                div()
                    .id(("alcol", i))
                    .size(px(16.))
                    .rounded(px(4.))
                    .bg(hexc(*c))
                    .cursor_pointer()
                    .when(on, |d| {
                        d.shadow(vec![
                            BoxShadow { color: t.panel_solid, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false },
                            BoxShadow { color: hexc(*c), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(3.5), inset: false },
                        ])
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pages.auto.lb_color = i;
                        cx.notify();
                    }))
            }));
            return div()
                .id("au-lbnew")
                .flex()
                .items_center()
                .gap(px(10.))
                .flex_wrap()
                .justify_end()
                .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" {
                        this.pages.auto.adding_label = false;
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .w(px(150.))
                        .h(px(30.))
                        .px(px(9.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(t.border)
                        .bg(t.surface)
                        .flex()
                        .items_center()
                        .child(Input::new(&s.lb_name).appearance(false).bordered(false).p_0().text_size(px(13.))),
                )
                .child(cols)
                .child(btn("alcancel", Btn::Plain, true, false, cx).child("Cancel").on_click(cx.listener(|this, _, _, cx| {
                    this.pages.auto.adding_label = false;
                    cx.notify();
                })))
                .child(btn("aladd", Btn::Primary, true, false, cx).child("Add").on_click(cx.listener(|this, _, window, cx| this.add_label(window, cx))))
                .into_any_element();
        }
        let dot = |c: u32| div().size(px(7.)).rounded(px(2.)).bg(hexc(c)).flex_none();
        let content = match s.label(&a.label) {
            Some(l) => div().flex().items_center().gap(px(6.)).child(dot(l.c)).child(l.name.clone()).into_any_element(),
            None => "No label".into_any_element(),
        };
        let mut opts: Vec<(String, AnyElement)> = vec![(String::new(), "No label".into_any_element())];
        for l in &s.labels {
            opts.push((l.id.clone(), div().flex().items_center().gap(px(8.)).child(dot(l.c)).child(l.name.clone()).into_any_element()));
        }
        opts.push(("__new".into(), div().text_color(t.text_2).child("+ New label…").into_any_element()));
        self.au_select(Menu::Field("label"), content, opts, a.label.clone(), false, true, cx)
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
///   auto:<index>   select the n-th automation      auto:new       start a draft
///   auto:adv       open Advanced                     auto:trig      open the Add Trigger menu
///   auto:menu:<k>  open a select (model, repo, label…) auto:every    open the schedule select
///   auto:time      open the time menu                auto:run / auto:delete / auto:toggle
///   auto:filter:<q> filter the list                  auto:newlabel  start a new label
pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k != "auto" {
        return false;
    }
    let (a, b) = v.split_once(':').unwrap_or((v, ""));
    match a {
        "new" => app.au_new(window, cx),
        "adv" => app.pages.auto.adv_open = true,
        "trig" => app.pages.auto.menu = Some(Menu::AddTrig),
        "every" => app.pages.auto.menu = Some(Menu::Trig(0, "every")),
        "time" => app.pages.auto.menu = Some(Menu::Time(0)),
        "menu" => {
            let key: &'static str = match b {
                "model" => "model",
                "effort" => "effort",
                "mode" => "mode",
                "repo" => "repo",
                "working" => "working",
                "conversation" => "conversation",
                "catchup" => "catchup",
                "notify" => "notify",
                "timeout" => "timeout",
                _ => "label",
            };
            app.pages.auto.menu = Some(Menu::Field(key));
        }
        "run" => app.au_run(cx),
        "delete" => app.au_delete(window, cx),
        "toggle" => {
            if let Some(a) = app.pages.auto.cur_mut() {
                a.active = !a.active;
            }
        }
        "newlabel" => app.au_set(&Menu::Field("label"), "__new".into(), window, cx),
        "filter" => {
            let f = app.pages.auto.filter.clone();
            f.update(cx, |s, cx| s.set_value(b.to_string(), window, cx));
        }
        _ => {
            if let Ok(i) = a.parse::<usize>() {
                if let Some(id) = app.pages.auto.autos.get(i).map(|x| x.id.clone()) {
                    app.au_pick(id, window, cx);
                }
            }
        }
    }
    cx.notify();
    true
}
