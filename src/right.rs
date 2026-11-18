//! Right panel: the git / environment panel for the active worktree.
//! Three views picked from the top bar — Source control, Files, Actions —
//! ported from the mock's "right panel" CSS and renderPanel / rpBar scripts.
use crate::{
    app::TakoApp,
    chat::TabKind,
    data::{self, Change, PrState},
    theme::{ActiveTheme, Theme},
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    component::{
        input::{Input, InputEvent, InputState, Textarea, TextareaState},
        tooltip::Tooltip,
    },
    *,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RpView {
    Scm,
    Files,
    Actions,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Commit,
    Push,
    Pr,
}
impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Commit => "Commit",
            Mode::Push => "Commit & push",
            Mode::Pr => "Commit & create PR",
        }
    }
}

/// Per-worktree source-control state (the mock's `SC[activeId]`).
#[derive(Default)]
pub struct Scm {
    pub staged: HashSet<&'static str>,
    pub done: HashSet<&'static str>,
    pub msg: String,
    pub ahead: u32,
    pub fold_staged: bool,
    pub fold_changes: bool,
    pub filter: String,
    pub search: bool,
    pub mode: Option<Mode>,
    pub menu: bool,
    /// "Committed n files" flash on the main button.
    pub flash: Option<(Instant, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActState {
    Idle,
    Live,
    Run,
    Ok,
}

struct Action {
    id: &'static str,
    name: &'static str,
    cmd: &'static str,
    server: bool,
    port: u16,
    ok: &'static str,
    ms: u64,
}

const ACTIONS: &[Action] = &[
    Action { id: "dev", name: "Dev server", cmd: "bun run dev", server: true, port: 3000, ok: "", ms: 0 },
    Action { id: "test", name: "Test", cmd: "bun test", server: false, port: 0, ok: "6153 passed", ms: 2400 },
    Action { id: "typecheck", name: "Typecheck", cmd: "tsc --noEmit", server: false, port: 0, ok: "No errors", ms: 1700 },
    Action { id: "lint", name: "Lint", cmd: "bun run lint", server: false, port: 0, ok: "Clean", ms: 1100 },
    Action { id: "build", name: "Build", cmd: "bun run build", server: false, port: 0, ok: "1.84 MB · 3.1s", ms: 3100 },
    Action { id: "storybook", name: "Storybook", cmd: "bun run storybook", server: true, port: 6006, ok: "", ms: 0 },
];

/// The worktree's files for the Files view.
const TREE_FILES: &[&str] = &[
    "src/components/chat/Timeline.tsx",
    "src/components/chat/timeline.collapse.test.ts",
    "src/components/chat/Divider.tsx",
    "src/components/chat/Row.tsx",
    "src/components/chat/ResumedHeader.tsx",
    "src/lib/timeline.ts",
    "src/lib/session.ts",
    "src/styles/timeline.css",
    "src/styles/tokens.css",
    "src/app.tsx",
    "tests/fixtures.ts",
    "tests/setup.ts",
    "CHANGELOG.md",
    "PLAN.md",
    "README.md",
    "package.json",
    "tsconfig.json",
    "bun.lockb",
];

pub struct RightState {
    pub view: RpView,
    pub scm: HashMap<usize, Scm>,
    pub msg: Entity<TextareaState>,
    pub filter: Entity<InputState>,
    /// worktree whose message is currently loaded in `msg`
    msg_owner: usize,
    pub open_dirs: HashSet<String>,
    pub actions: HashMap<usize, HashMap<&'static str, ActState>>,
    /// file whose row was last opened (highlighted while the Changes tab is active)
    pub open_file: Option<&'static str>,
    pub viewed: HashSet<&'static str>,
}

impl RightState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        let msg = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 10).placeholder("Message"));
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter changed files"));
        cx.subscribe_in(&msg, window, |this: &mut TakoApp, st, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                let v = st.read(cx).value().to_string();
                let owner = this.right.msg_owner;
                this.right.scm.entry(owner).or_default().msg = v;
            }
            InputEvent::PressEnter { secondary: true, .. } => this.commit(window, cx),
            _ => {}
        })
        .detach();
        cx.subscribe_in(&filter, window, |this: &mut TakoApp, st, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let v = st.read(cx).value().to_string();
                this.scm_mut().filter = v;
                cx.notify();
            }
        })
        .detach();
        RightState {
            view: RpView::Scm,
            scm: HashMap::new(),
            msg,
            filter,
            msg_owner: 0,
            open_dirs: ["src", "src/components", "src/components/chat", "src/lib"].iter().map(|s| s.to_string()).collect(),
            actions: HashMap::new(),
            open_file: None,
            viewed: HashSet::new(),
        }
    }
}

fn file_icon(p: &str) -> &'static str {
    if p.ends_with(".json") {
        "fjson"
    } else if [".ts", ".tsx", ".js", ".jsx", ".css"].iter().any(|e| p.ends_with(e)) {
        "fcode"
    } else if p.ends_with(".md") {
        "doc"
    } else {
        "file"
    }
}

fn split_path(p: &str) -> (&str, &str) {
    match p.rsplit_once('/') {
        Some((d, f)) => (d, f),
        None => ("", p),
    }
}

fn st_color(t: &Theme, st: &str) -> Hsla {
    match st {
        "A" => t.green,
        "D" => t.red,
        _ => t.yellow,
    }
}

fn st_title(st: &str) -> &'static str {
    match st {
        "A" => "Added",
        "D" => "Deleted",
        _ => "Modified",
    }
}

fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a: c.a * a, ..c }
}

/// color-mix(in srgb, a p%, b) for opaque colours.
fn mix(a: Hsla, p: f32, b: Hsla) -> Hsla {
    let (a, b) = (Rgba::from(a), Rgba::from(b));
    let q = 1. - p;
    Rgba { r: a.r * p + b.r * q, g: a.g * p + b.g * q, b: a.b * p + b.b * q, a: a.a * p + b.a * q }.into()
}

fn tip(text: &'static str) -> impl Fn(&mut Window, &mut App) -> AnyView {
    move |window, cx| Tooltip::new(text).build(window, cx)
}

/// `.rp .ib` — a 26px (or `s`) square icon button.
fn ib(t: &Theme, id: impl Into<ElementId>, ic: &'static str, s: f32, icon_s: f32) -> Stateful<Div> {
    ib_c(t, id, ic, s, icon_s, t.text)
}

fn ib_c(t: &Theme, id: impl Into<ElementId>, ic: &'static str, s: f32, icon_s: f32, hover_c: Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(s))
        .flex_none()
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .group("ib")
        .hover(move |d| d.bg(t.surface_hover))
        .child(icon(ic, icon_s, t.text_3).group_hover("ib", move |s| s.text_color(hover_c)))
}

impl TakoApp {
    fn wt_id(&self) -> usize {
        self.active
    }

    pub fn scm_mut(&mut self) -> &mut Scm {
        let id = self.wt_id();
        self.right.scm.entry(id).or_default()
    }

    fn live_changes(&self) -> Vec<Change> {
        let done = self.right.scm.get(&self.wt_id()).map(|s| s.done.clone()).unwrap_or_default();
        data::changes().into_iter().filter(|c| !done.contains(c.path)).collect()
    }

    fn stage_all(&mut self, cx: &mut Context<Self>) {
        let live: Vec<&'static str> = self.live_changes().iter().map(|c| c.path).collect();
        let st = self.scm_mut();
        st.staged.extend(live);
        cx.notify();
    }

    /// Typewriter-fill the commit message (the mock's generate()).
    fn generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let live = self.live_changes();
        let st = self.scm_mut();
        let has_md = live.iter().filter(|f| st.staged.is_empty() || st.staged.contains(f.path)).any(|f| f.path.ends_with(".md"));
        let text = format!(
            "Only render dividers for “worked for” summaries\n\nResumed headers no longer draw a divider. Adds a regression test{}",
            if has_md { " and a changelog entry." } else { "." }
        );
        let input = self.right.msg.clone();
        input.update(cx, |s, cx| s.set_value("", window, cx));
        let owner = self.wt_id();
        cx.spawn_in(window, async move |this, cx| {
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            while i <= chars.len() {
                let s: String = chars[..i].iter().collect();
                let ok = this
                    .update_in(cx, |this, window, cx| {
                        if this.right.msg_owner != owner {
                            return false;
                        }
                        input.update(cx, |st, cx| st.set_value(s.clone(), window, cx));
                        this.right.scm.entry(owner).or_default().msg = s;
                        true
                    })
                    .unwrap_or(false);
                if !ok {
                    break;
                }
                i += 3;
                cx.background_executor().timer(Duration::from_millis(12)).await;
            }
        })
        .detach();
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let st = self.scm_mut();
        if st.staged.is_empty() {
            return;
        }
        if st.msg.trim().is_empty() {
            self.generate(window, cx);
            return;
        }
        let n = st.staged.len();
        let staged: Vec<_> = st.staged.drain().collect();
        st.done.extend(staged);
        st.msg.clear();
        st.ahead += 1;
        st.flash = Some((Instant::now(), n));
        self.right.msg.update(cx, |s, cx| s.set_value("", window, cx));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(1600)).await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
        cx.notify();
    }

    /// Open the Changes (diff) tab in the session, like forgeOpenDiff / "View all".
    fn open_diff(&mut self, file: Option<&'static str>, cx: &mut Context<Self>) {
        if let Some(i) = self.chat.tabs.iter().position(|t| t.kind == TabKind::Diff) {
            self.chat.active_tab = i;
        }
        self.right.open_file = file;
        cx.notify();
    }

    fn diff_tab_active(&self) -> bool {
        self.chat.tabs.get(self.chat.active_tab).is_some_and(|t| t.kind == TabKind::Diff)
    }

    pub fn render_right(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        // load this worktree's commit message into the shared input
        let wt = self.wt_id();
        if self.right.msg_owner != wt {
            self.right.msg_owner = wt;
            let (m, f) = self.right.scm.get(&wt).map(|s| (s.msg.clone(), s.filter.clone())).unwrap_or_default();
            self.right.msg.update(cx, |s, cx| s.set_value(m, window, cx));
            self.right.filter.update(cx, |s, cx| s.set_value(f, window, cx));
        }
        let body: AnyElement = match self.right.view {
            RpView::Scm => self.render_scm(window, cx).into_any_element(),
            RpView::Files => self.render_files(cx).into_any_element(),
            RpView::Actions => self.render_actions(cx).into_any_element(),
        };
        div()
            .w(px(340.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .overflow_hidden()
            .border_l_1()
            .border_color(t.border)
            .text_size(px(13.))
            .child(self.render_rp_bar(cx))
            .child(div().flex_1().min_h_0().flex().flex_col().child(body))
    }

    fn render_rp_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let n = self.live_changes().len();
        let view = self.right.view;
        let btn = |v: RpView, ic: &'static str, title: &'static str| {
            let on = view == v;
            let hover_c = t.text;
            div()
                .id(SharedString::from(format!("rpv-{ic}")))
                .relative()
                .size(px(38.))
                .rounded_t(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .group("rpv")
                .child(icon(ic, 18., if on { t.text } else { t.text_3 }).group_hover("rpv", move |s| s.text_color(hover_c)))
                .when(on, |d| d.child(div().absolute().left(px(9.)).right(px(9.)).bottom(px(-1.)).h(px(2.)).rounded(px(2.)).bg(t.text)))
                .when(v == RpView::Scm && n > 0, |d| {
                    // badge with a 2px ring in the page background
                    d.child(
                        div()
                            .absolute()
                            .top(px(4.))
                            .left(px(20.))
                            .p(px(2.))
                            .rounded(px(9.))
                            .bg(t.bg)
                            .child(
                                div()
                                    .min_w(px(14.))
                                    .h(px(14.))
                                    .px(px(3.5))
                                    .rounded(px(7.))
                                    .bg(t.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(9.))
                                    .line_height(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(white())
                                    .child(n.to_string()),
                            ),
                    )
                })
                .tooltip(tip(title))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.right.view = v;
                    cx.notify();
                }))
        };
        div()
            .h(px(46.))
            .flex_none()
            .flex()
            .items_end()
            .gap(px(4.))
            .px(px(12.))
            .border_b_1()
            .border_color(t.border)
            .child(btn(RpView::Scm, "branch", "Source control"))
            .child(btn(RpView::Files, "files", "Files"))
            .child(btn(RpView::Actions, "grid", "Actions"))
    }

    // ---------------------------------------------------------------- SCM
    fn render_scm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let th = self.thread().clone();
        let live = self.live_changes();
        let (a_sum, d_sum): (u32, u32) = live.iter().fold((0, 0), |(a, d), f| (a + f.add, d + f.del));
        let diff_active = self.diff_tab_active();
        let open_file = self.right.open_file;
        let viewed = self.right.viewed.clone();
        let st = self.scm_mut();
        if st.flash.is_some_and(|(i, _)| i.elapsed() >= Duration::from_millis(1600)) {
            st.flash = None;
        }
        let q = st.filter.to_lowercase();
        let matches = |f: &Change| q.is_empty() || f.path.to_lowercase().contains(&q);
        let staged: Vec<Change> = live.iter().filter(|f| st.staged.contains(f.path) && matches(f)).cloned().collect();
        let unstaged: Vec<Change> = live.iter().filter(|f| !st.staged.contains(f.path) && matches(f)).cloned().collect();
        let has_staged = !st.staged.is_empty();
        let (ahead, search, menu, mode, flash, fold_s, fold_c) =
            (st.ahead, st.search, st.menu, st.mode.unwrap_or(Mode::Commit), st.flash, st.fold_staged, st.fold_changes);
        let filtering = !q.is_empty();
        let filter_focused = self.right.filter.read(cx).focus_handle(cx).is_focused(window);
        let msg_focused = self.right.msg.read(cx).focus_handle(cx).is_focused(window);

        // PR button: open / merged / draft → Create PR; checks dot
        let (pr_label, pr_icon_c, pr_tip): (String, Hsla, &'static str) = match th.pr_state {
            PrState::Merged => (format!("#{}", th.pr), rgb(0xb48ae8).into(), "Pull request · Merged"),
            PrState::Draft => ("Create PR".into(), t.text_2, "Create pull request"),
            _ => (format!("#{}", th.pr), t.green, "Pull request · Open"),
        };
        let checks: Option<&str> = match th.pr_state {
            PrState::Merged => None,
            _ if th.running => Some("run"),
            _ if th.id == 1 => Some("bad"),
            PrState::Open => Some("ok"),
            _ => None,
        };
        let rp_btn = |id: &'static str| {
            let hb = with_alpha(t.text, 0.16);
            div()
                .id(id)
                .h(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(5.))
                .rounded(px(7.))
                .border_1()
                .border_color(t.border)
                .bg(t.surface)
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.text)
                .whitespace_nowrap()
                .hover(move |d| d.bg(t.surface_hover).border_color(hb))
        };
        let dot = checks.map(|c| {
            let col = match c {
                "ok" => t.green,
                "bad" => t.red,
                _ => t.yellow,
            };
            let d = div().ml(px(1.)).size(px(7.)).rounded_full().flex_none().bg(col);
            if c == "run" {
                d.with_animation("ck-pulse", Animation::new(Duration::from_millis(1600)).repeat(), |d, p| {
                    d.opacity(0.35 + 0.65 * (0.5 + 0.5 * (p * std::f32::consts::TAU).cos()))
                })
                .into_any_element()
            } else {
                d.into_any_element()
            }
        });

        let top = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(8.))
            .pt(px(12.))
            .pr(px(12.))
            .pb(px(12.))
            .pl(px(16.))
            .border_b_1()
            .border_color(t.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .mb(px(4.))
                    .child(
                        rp_btn("rp-pr")
                            .pl(px(7.))
                            .pr(px(9.))
                            .child(icon("pr", 14., pr_icon_c))
                            .child(pr_label)
                            .children(dot)
                            .tooltip(tip(match checks {
                                Some("bad") => "Pull request · 1 check failing",
                                Some("run") => "Pull request · Checks running",
                                _ => pr_tip,
                            })),
                    )
                    .child(rp_btn("rp-editor").w(px(28.)).justify_center().child(icon("code", 14., t.text_2)).tooltip(tip("Open in editor")))
                    .child(div().flex_1())
                    .child(ib(&t, "rp-search", "search", 26., 15.).tooltip(tip("Filter files")).on_click(cx.listener(|this, _, window, cx| {
                        let st = this.scm_mut();
                        st.search = !st.search;
                        if st.search {
                            this.right.filter.update(cx, |s, cx| s.focus(window, cx));
                        } else {
                            st.filter.clear();
                            this.right.filter.update(cx, |s, cx| s.set_value("", window, cx));
                        }
                        cx.notify();
                    })))
                    .child(ib(&t, "rp-more", "dots", 26., 15.).tooltip(tip("More"))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .pr(px(4.))
                    .child(icon("branch", 14., t.text_2))
                    .child(div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(MONO).text_size(px(12.5)).child(th.branch.clone()))
                    .child(div().ml(px(-3.)).child(icon("chev-d", 12., t.text_3)))
                    .child(div().flex_1())
                    .child(div().font_family(MONO).text_size(px(12.)).text_color(t.green).child(format!("+{a_sum}")))
                    .child(div().font_family(MONO).text_size(px(12.)).text_color(t.red).child(format!("-{d_sum}"))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .pr(px(4.))
                    .child(div().w(px(14.)).flex().justify_center().text_color(t.text_3).child("→"))
                    .child(div().font_family(MONO).text_size(px(12.5)).text_color(t.text_2).underline().text_decoration_color(t.border).child("origin/main"))
                    .child(div().flex_1())
                    .when(ahead > 0, |d| d.child(div().font_family(MONO).text_size(px(12.)).text_color(t.accent).child(format!("↑{ahead}"))))
                    .child(div().font_family(MONO).text_size(px(12.)).text_color(t.text_3).child("↓248")),
            );

        // commit box
        let accent_b = with_alpha(t.accent, 0.55);
        let split_bg = if flash.is_some() { mix(t.green, 0.72, black()) } else if has_staged { t.accent } else { t.surface };
        let primary = has_staged || flash.is_some();
        let fg = if primary { white() } else { t.text };
        let hover_bg = if primary { hsla(0., 0., 1., 0.1) } else { t.surface_hover };
        let main_label: AnyElement = if let Some((_, n)) = flash {
            div().flex().items_center().gap(px(7.)).child(icon("check", 14., fg)).child(format!("Committed {n} file{}", if n > 1 { "s" } else { "" })).into_any_element()
        } else if has_staged {
            div().flex().items_center().gap(px(7.)).child(icon("check", 14., fg)).child(mode.label()).into_any_element()
        } else {
            div().flex().items_center().gap(px(7.)).child(icon("plus", 14., fg)).child("Stage All").into_any_element()
        };
        let menu_el = menu.then(|| {
            div()
                .id("rp-menu")
                .absolute()
                .top(px(34. + 6.))
                .right(px(-1.))
                .min_w(px(190.))
                .p(px(4.))
                .rounded(px(10.))
                .bg(t.panel_solid)
                .border_1()
                .border_color(t.border)
                .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.18), offset: point(px(0.), px(8.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
                .flex()
                .flex_col()
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.scm_mut().menu = false;
                    cx.notify();
                }))
                .children([Mode::Commit, Mode::Push, Mode::Pr].into_iter().map(|m| {
                    let on = m == mode;
                    div()
                        .id(SharedString::from(format!("mode-{m:?}")))
                        .flex()
                        .items_center()
                        .px(px(10.))
                        .py(px(7.))
                        .rounded(px(7.))
                        .text_size(px(13.))
                        .text_color(if on { t.text } else { t.text_2 })
                        .hover(|d| d.bg(t.surface_hover))
                        .child(m.label())
                        .when(on, |d| d.child(div().ml_auto().pl(px(12.)).text_color(t.accent).child("✓")))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let st = this.scm_mut();
                            st.mode = Some(m);
                            st.menu = false;
                            cx.notify();
                        }))
                }))
        });

        let commit_box = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(8.))
            .pt(px(12.))
            .pr(px(12.))
            .pb(px(8.))
            .pl(px(16.))
            .when(search || filtering || filter_focused, |d| {
                d.child(
                    div()
                        .h(px(32.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(if filter_focused { accent_b } else { t.border })
                        .bg(t.surface)
                        .child(icon("search", 14., t.text_3))
                        .child(div().flex_1().child(Input::new(&self.right.filter).appearance(false).bordered(false).text_size(px(13.)).p_0())),
                )
            })
            .child(
                div()
                    .relative()
                    .min_h(px(72.))
                    .pt(px(9.))
                    .pr(px(34.))
                    .pb(px(9.))
                    .pl(px(11.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(if msg_focused { accent_b } else { t.border })
                    .bg(t.surface)
                    .text_size(px(13.))
                    .child(Textarea::new(&self.right.msg).appearance(false).bordered(false).text_size(px(13.)).p_0())
                    .child(
                        div().absolute().top(px(5.)).right(px(5.)).child(
                            ib_c(&t, "rp-gen", "stars", 26., 15., t.accent)
                                .tooltip(tip("Generate commit message"))
                                .on_click(cx.listener(|this, _, window, cx| this.generate(window, cx))),
                        ),
                    ),
            )
            .child(
                div()
                    .relative()
                    .h(px(34.))
                    .flex()
                    .rounded(px(9.))
                    .border_1()
                    .border_color(if primary { transparent_black() } else { t.border })
                    .bg(split_bg)
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .child(
                        div()
                            .id("rp-main")
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_l(px(8.))
                            .hover(move |d| d.bg(hover_bg))
                            .child(main_label)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.scm_mut().staged.is_empty() {
                                    this.stage_all(cx)
                                } else {
                                    this.commit(window, cx)
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("rp-menu-btn")
                            .w(px(34.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_l_1()
                            .border_color(if primary { hsla(0., 0., 1., 0.25) } else { t.border })
                            .rounded_r(px(8.))
                            .hover(move |d| d.bg(hover_bg))
                            .child(icon("chev-d", 14., if primary { white() } else { t.text_2 }))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let st = this.scm_mut();
                                st.menu = !st.menu;
                                cx.notify();
                            })),
                    )
                    .children(menu_el.map(|m| deferred(m).with_priority(1))),
            );

        // file sections
        let row = |f: &Change, is_staged: bool| {
            let path = f.path;
            let (dir, name) = split_path(path);
            let on = diff_active && open_file == Some(path);
            let is_viewed = viewed.contains(path);
            let group = SharedString::from(format!("rpf-{path}"));
            let small = |id: String, ic: &'static str, title: &'static str| ib(&t, SharedString::from(id), ic, 22., 13.).tooltip(tip(title));
            let acts = div()
                .flex()
                .flex_none()
                .invisible()
                .group_hover(group.clone(), |d| d.visible())
                .child(small(format!("open-{path}"), "file", "Open file").on_click(cx.listener(move |this, _, _, cx| this.open_diff(Some(path), cx))))
                .when(is_staged, |d| {
                    d.child(small(format!("unstage-{path}"), "minus", "Unstage").on_click(cx.listener(move |this, _, _, cx| {
                        this.scm_mut().staged.remove(path);
                        cx.notify();
                    })))
                })
                .when(!is_staged, |d| {
                    d.child(small(format!("discard-{path}"), "undo", "Discard changes").on_click(cx.listener(move |this, _, _, cx| {
                        this.scm_mut().done.insert(path);
                        cx.notify();
                    })))
                    .child(small(format!("stage-{path}"), "plus", "Stage").on_click(cx.listener(move |this, _, _, cx| {
                        this.scm_mut().staged.insert(path);
                        cx.notify();
                    })))
                });
            let cnt = div()
                .flex()
                .flex_none()
                .gap(px(5.))
                .font_family(MONO)
                .text_size(px(12.))
                .group_hover(group.clone(), |d| d.invisible())
                .when(f.add > 0, |d| d.child(div().text_color(t.green).child(format!("+{}", f.add))))
                .when(f.del > 0, |d| d.child(div().text_color(t.red).child(format!("-{}", f.del))));
            div()
                .id(SharedString::from(format!("rpf-{path}")))
                .group(group)
                .h(px(30.))
                .flex()
                .items_center()
                .gap(px(8.))
                .pl(px(30.))
                .pr(px(12.))
                .min_w_0()
                .whitespace_nowrap()
                .when(on, |d| d.bg(t.surface_active))
                .hover(|d| d.bg(t.surface_hover))
                .child(div().opacity(0.85).child(icon(file_icon(path), 15., t.yellow)))
                .child(div().flex_none().max_w(relative(0.6)).overflow_hidden().text_ellipsis().text_color(if is_viewed { t.text_3 } else { t.text }).child(name))
                .child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().text_size(px(12.)).text_color(t.text_3).child(dir.to_string()))
                // counts and hover actions share the slot (actions replace the counts on hover)
                .child(div().relative().flex().flex_none().justify_end().child(cnt).child(div().absolute().right_0().top(px(-4.)).child(acts)))
                .child(div().w(px(12.)).flex_none().flex().justify_center().font_family(MONO).text_size(px(11.5)).font_weight(FontWeight::SEMIBOLD).text_color(st_color(&t, f.st)).child(f.st))
                .tooltip({
                    let s = format!("{path} · {}", st_title(f.st));
                    move |window, cx| Tooltip::new(s.clone()).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| this.open_diff(Some(path), cx)))
        };
        let section = |key: &'static str, title: &'static str, list: &[Change], folded: bool, actions: Div| {
            let none = if filtering { "No files match" } else if key == "staged" { "Nothing staged" } else { "Working tree clean" };
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(SharedString::from(format!("sh-{key}")))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .pt(px(8.))
                        .pr(px(10.))
                        .pb(px(6.))
                        .pl(px(12.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text_2)
                        .child(chev(13., t.text_3, folded))
                        .child(tracked(&title.to_uppercase(), 11.5, 0.06))
                        .child(div().font_weight(FontWeight::MEDIUM).text_color(t.text_3).child(list.len().to_string()))
                        .child(div().flex_1())
                        .child(actions)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let st = this.scm_mut();
                            if key == "staged" {
                                st.fold_staged = !st.fold_staged
                            } else {
                                st.fold_changes = !st.fold_changes
                            }
                            cx.notify();
                        })),
                )
                .when(!folded, |d| {
                    d.when(list.is_empty(), |d| d.child(div().pt(px(6.)).pb(px(10.)).px(px(30.)).text_size(px(12.)).text_color(t.text_3).child(none)))
                        .children(list.iter().map(|f| row(f, key == "staged")))
                })
        };
        let sh_ib = |id: &'static str, ic: &'static str, title: &'static str| ib(&t, id, ic, 24., 14.).opacity(0.8).tooltip(tip(title));
        let staged_sec = (has_staged || !staged.is_empty()).then(|| {
            section(
                "staged",
                "Staged changes",
                &staged,
                fold_s,
                div().flex().items_center().child(sh_ib("unstage-all", "minus", "Unstage all").on_click(cx.listener(|this, _, _, cx| {
                    this.scm_mut().staged.clear();
                    cx.notify();
                }))),
            )
        });
        let changes_sec = section(
            "changes",
            "Changes",
            &unstaged,
            fold_c,
            div()
                .flex()
                .items_center()
                .child(sh_ib("discard-all", "undo", "Discard all").on_click(cx.listener(|this, _, _, cx| {
                    // (the mock asks confirm() first)
                    let live: Vec<&'static str> = this.live_changes().iter().map(|c| c.path).collect();
                    let st = this.scm_mut();
                    for p in live {
                        if !st.staged.contains(p) {
                            st.done.insert(p);
                        }
                    }
                    cx.notify();
                })))
                .child(sh_ib("stage-all", "plus", "Stage all").on_click(cx.listener(|this, _, _, cx| this.stage_all(cx))))
                .child(
                    div()
                        .id("view-all")
                        .ml(px(2.))
                        .px(px(6.))
                        .py(px(3.))
                        .rounded(px(6.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.text_2)
                        .hover(|d| d.bg(t.surface_hover).text_color(t.text))
                        .child("View all")
                        .on_click(cx.listener(|this, _, _, cx| this.open_diff(None, cx))),
                ),
        );

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(top)
            .child(commit_box)
            .child(div().id("rp-scroll").flex_1().min_h_0().overflow_y_scroll().pt(px(4.)).pb(px(16.)).children(staged_sec).child(changes_sec))
    }

    // ---------------------------------------------------------------- Files
    fn render_files(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let th = self.thread().clone();
        let live = self.live_changes();
        let status = |p: &str| live.iter().find(|c| c.path == p).map(|c| c.st);

        #[derive(Default)]
        struct Node {
            dirs: BTreeMap<String, Node>,
            files: Vec<String>,
        }
        let mut root = Node::default();
        for p in TREE_FILES {
            let parts: Vec<&str> = p.split('/').collect();
            let mut n = &mut root;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    n.files.push(part.to_string());
                } else {
                    n = n.dirs.entry(part.to_string()).or_default();
                }
            }
        }
        enum Row {
            Dir { full: String, name: String, d: usize, open: bool, dirty: bool },
            File { full: String, name: String, d: usize, st: Option<&'static str> },
        }
        let mut rows = vec![];
        fn walk(n: &Node, base: &str, d: usize, open: &HashSet<String>, live: &[Change], rows: &mut Vec<Row>, status: &dyn Fn(&str) -> Option<&'static str>) {
            for (name, child) in &n.dirs {
                let full = if base.is_empty() { name.clone() } else { format!("{base}/{name}") };
                let is_open = open.contains(&full);
                let dirty = live.iter().any(|c| c.path.starts_with(&format!("{full}/")));
                rows.push(Row::Dir { full: full.clone(), name: name.clone(), d, open: is_open, dirty });
                if is_open {
                    walk(child, &full, d + 1, open, live, rows, status);
                }
            }
            let mut files = n.files.clone();
            files.sort();
            for f in files {
                let full = if base.is_empty() { f.clone() } else { format!("{base}/{f}") };
                let st = status(&full);
                rows.push(Row::File { full, name: f, d, st });
            }
        }
        walk(&root, "", 0, &self.right.open_dirs, &live, &mut rows, &status);

        let rows = rows.into_iter().enumerate().map(|(i, r)| {
            let base = div().id(("ft", i)).h(px(28.)).flex().items_center().gap(px(6.)).pr(px(12.)).text_size(px(13.)).whitespace_nowrap().hover(|d| d.bg(t.surface_hover));
            match r {
                Row::Dir { full, name, d, open, dirty } => base
                    .pl(px(12. + d as f32 * 14.))
                    .child(chev(12., t.text_3, !open))
                    .child(icon("folder", 15., t.text_3))
                    .child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().child(name))
                    .when(dirty, |d| d.child(div().size(px(6.)).rounded_full().bg(t.yellow).opacity(0.8)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.right.open_dirs.remove(&full) {
                            this.right.open_dirs.insert(full.clone());
                        }
                        cx.notify();
                    })),
                Row::File { full, name, d, st } => {
                    let c = st.map(|s| st_color(&t, s)).unwrap_or(t.text);
                    let changed = data::changes().into_iter().find(|c| c.path == full).map(|c| c.path);
                    base.pl(px(30. + d as f32 * 14.))
                        .child(icon(file_icon(&full), 15., t.text_3))
                        .child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().text_color(c).when(st == Some("D"), |d| d.line_through()).child(name))
                        .when_some(st, |d, s| d.child(div().font_family(MONO).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(c).child(s)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if changed.is_some() {
                                this.open_diff(changed, cx)
                            }
                        }))
                }
            }
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                self.rp_vh(&t)
                    .child(tracked(&th.repo.to_uppercase(), 11.5, 0.06))
                    .child(div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(MONO).font_weight(FontWeight::NORMAL).text_color(t.text_3).child(th.branch.clone()))
                    .child(div().flex_1())
                    .child(ib(&t, "ft-collapse", "minus", 24., 14.).tooltip(tip("Collapse folders")).on_click(cx.listener(|this, _, _, cx| {
                        this.right.open_dirs.clear();
                        cx.notify();
                    }))),
            )
            .child(div().id("ft").flex_1().min_h_0().overflow_y_scroll().pt(px(2.)).pb(px(16.)).children(rows))
    }

    /// `.rp-vh` — the uppercase view header.
    fn rp_vh(&self, t: &Theme) -> Div {
        div()
            .flex_none()
            .h(px(44.))
            .flex()
            .items_center()
            .gap(px(8.))
            .pt(px(12.))
            .pr(px(10.))
            .pb(px(8.))
            .pl(px(16.))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(t.text_2)
    }

    // ---------------------------------------------------------------- Actions
    fn render_actions(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let th = self.thread().clone();
        let wt = self.wt_id();
        let states = self.right.actions.entry(wt).or_insert_with(|| HashMap::from([("dev", ActState::Live)])).clone();
        let list = ACTIONS.iter().enumerate().map(|(i, a)| {
            let s = states.get(a.id).copied().unwrap_or(ActState::Idle);
            let busy = matches!(s, ActState::Live | ActState::Run);
            let id = a.id;
            let status: Option<AnyElement> = match s {
                ActState::Live => Some(
                    div()
                        .id(("as-live", i))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(t.text_2)
                        .hover(|d| d.text_color(t.text).underline())
                        .child(div().size(px(7.)).rounded_full().bg(t.green))
                        .child(format!("localhost:{}", a.port))
                        .into_any_element(),
                ),
                ActState::Run => Some(div().flex().items_center().gap(px(6.)).text_size(px(12.)).text_color(t.text_3).child(spinner(("as-sp", i), 11., t.text_3)).child("Running").into_any_element()),
                ActState::Ok => Some(div().text_size(px(12.)).text_color(t.green).child(format!("✓ {}", a.ok)).into_any_element()),
                ActState::Idle => None,
            };
            let hb = with_alpha(t.text, 0.18);
            div()
                .id(("act", i))
                .flex()
                .items_center()
                .gap(px(10.))
                .pt(px(8.))
                .pr(px(8.))
                .pb(px(8.))
                .pl(px(6.))
                .rounded(px(8.))
                .hover(|d| d.bg(t.surface_hover))
                .child(
                    div()
                        .id(("act-btn", i))
                        .size(px(28.))
                        .flex_none()
                        .rounded(px(7.))
                        .border_1()
                        .border_color(t.border)
                        .flex()
                        .items_center()
                        .justify_center()
                        .group("actb")
                        .hover(move |d| d.border_color(hb))
                        .child(icon(if busy { "stop" } else { "play" }, 12., if busy { t.red } else { t.text_2 }).group_hover("actb", move |s| s.text_color(if busy { t.red } else { t.text })))
                        .tooltip(tip(if busy { "Stop" } else { "Run" }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let a = ACTIONS.iter().find(|x| x.id == id).unwrap();
                            let m = this.right.actions.entry(wt).or_default();
                            if busy {
                                m.insert(id, ActState::Idle);
                            } else if a.server {
                                m.insert(id, ActState::Live);
                            } else {
                                m.insert(id, ActState::Run);
                                let ms = a.ms;
                                cx.spawn(async move |this, cx| {
                                    cx.background_executor().timer(Duration::from_millis(ms)).await;
                                    this.update(cx, |this, cx| {
                                        let m = this.right.actions.entry(wt).or_default();
                                        if m.get(id) == Some(&ActState::Run) {
                                            m.insert(id, ActState::Ok);
                                        }
                                        cx.notify();
                                    })
                                    .ok();
                                })
                                .detach();
                            }
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().text_size(px(13.)).text_color(t.text).child(a.name))
                        .child(div().font_family(MONO).text_size(px(11.5)).text_color(t.text_3).overflow_hidden().text_ellipsis().whitespace_nowrap().child(a.cmd)),
                )
                .children(status)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                self.rp_vh(&t)
                    .child("ACTIONS")
                    .child(div().flex_1())
                    .child(div().font_weight(FontWeight::NORMAL).text_size(px(12.)).text_color(t.text_3).child(format!("Runs in {}", th.wt()))),
            )
            .child(div().flex().flex_col().py(px(2.)).px(px(8.)).children(list))
            .child(
                div()
                    .id("act-add")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .mx(px(16.))
                    .my(px(6.))
                    .py(px(6.))
                    .px(px(4.))
                    .text_size(px(12.5))
                    .text_color(t.text_3)
                    .group("actadd")
                    .hover(|d| d.text_color(t.text))
                    .child(icon("plus", 14., t.text_3).group_hover("actadd", move |s| s.text_color(t.text)))
                    .child("Add action"),
            )
    }
}

/// A chevron, turned -90° when `folded`.
fn chev(size: f32, color: Hsla, folded: bool) -> Svg {
    let c = icon("chev-d", size, color);
    if folded { c.with_transformation(Transformation::rotate(percentage(0.75))) } else { c }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// `rp:scm|files|actions`, `rpstage:all|<path>`, `rpmenu`, `rpsearch`, `rpgen`, `rpcommit`.
pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "rp" => {
            app.right.view = match v {
                "files" => RpView::Files,
                "actions" => RpView::Actions,
                _ => RpView::Scm,
            }
        }
        "rpstage" => {
            if v == "all" || v.is_empty() {
                app.stage_all(cx)
            } else if let Some(c) = data::changes().into_iter().find(|c| c.path.ends_with(v)) {
                app.scm_mut().staged.insert(c.path);
            }
        }
        "rpmenu" => app.scm_mut().menu = true,
        "rpsearch" => app.scm_mut().search = true,
        "rpgen" => app.generate(window, cx),
        "rpcommit" => app.commit(window, cx),
        _ => return false,
    }
    cx.notify();
    true
}
