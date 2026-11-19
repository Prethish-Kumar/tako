//! Session tool tabs: browser, editor, terminal, changes, markdown, whiteboard,
//! emulator, plus the ⌘T "new tab" launcher. Port of the mock's
//! "SESSION TABS — every session can hold any number of tool tabs" section.
//!
//! Per-tab state (terminal history, drawings, edits…) lives in a thread-local
//! [`ToolsState`] keyed by tab id, so this module needs no field on `TakoApp`.

mod browser;
mod data;
mod diff;
mod editor;
mod emulator;
mod markdown;
mod picker;
mod terminal;
mod whiteboard;

use crate::{
    app::TakoApp,
    chat::{Tab, TabKind},
    theme::{ActiveTheme, Theme},
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

pub use picker::{PICKER_ITEMS, PickerItem};

thread_local! {
    static STATE: RefCell<ToolsState> = RefCell::new(ToolsState::default());
}

/// Run `f` with the module state. Never call back into anything that could
/// re-enter `st` (GPUI listeners run later, so building elements is fine).
pub(crate) fn st<R>(f: impl FnOnce(&mut ToolsState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

#[derive(Default)]
pub struct ToolsState {
    pub diff: HashMap<usize, diff::DiffState>,
    pub browser: HashMap<usize, browser::BrowserState>,
    pub term: HashMap<usize, terminal::TermState>,
    pub editor: HashMap<usize, editor::EditorTab>,
    pub md: HashMap<usize, markdown::MdState>,
    pub wb: HashMap<usize, whiteboard::WbState>,
    pub emu: HashMap<usize, emulator::EmuState>,
    pub picker: picker::PickerState,
    /// Files ticked "Viewed" in any Changes tab (shared, like the mock's `viewedFiles`).
    pub viewed: HashSet<String>,
}

/// Paths marked "Viewed" in the Changes tabs (the right panel dims these).
pub fn viewed_files() -> HashSet<String> {
    st(|s| s.viewed.clone())
}

/// Offscreen snapshots (`TAKO_SNAP`) draw a single frame, so entrance
/// animations that start on that frame would be captured at 0 — skip them.
pub(crate) fn snap_mode() -> bool {
    thread_local!(static S: bool = std::env::var_os("TAKO_SNAP").is_some());
    S.with(|s| *s)
}

// ---------- colour + widget helpers shared by the views ----------

/// `color-mix(in srgb, a p, b)` (premultiplied, like the browser does).
pub(crate) fn mix(a: Hsla, b: Hsla, p: f32) -> Hsla {
    let (a, b) = (Rgba::from(a), Rgba::from(b));
    let al = a.a * p + b.a * (1. - p);
    if al <= 0. {
        return hsla(0., 0., 0., 0.);
    }
    let ch = |x: f32, y: f32| (x * a.a * p + y * b.a * (1. - p)) / al;
    Rgba { r: ch(a.r, b.r), g: ch(a.g, b.g), b: ch(a.b, b.b), a: al }.into()
}

/// `color-mix(in srgb, c p, transparent)`.
pub(crate) fn tr(c: Hsla, p: f32) -> Hsla {
    c.alpha(c.a * p)
}

pub(crate) fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

/// `.ib` — a 28px square icon button.
pub(crate) fn ib(id: impl Into<ElementId>, ic: &str, t: Theme) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(28.))
        .flex_none()
        .rounded(px(7.))
        .flex()
        .items_center()
        .justify_center()
        .hover(|s| s.bg(t.surface_hover))
        .child(icon(ic, 16., t.text_3))
}

/// `.bz-seg` — a segmented control. Items are (label, icon); returns the
/// container and lets the caller attach a click per index.
pub(crate) fn seg(
    id: &'static str,
    items: &[(&'static str, Option<&'static str>)],
    on: usize,
    t: Theme,
    mut click: impl FnMut(usize, Stateful<Div>) -> Stateful<Div>,
) -> Div {
    div().flex().flex_none().gap(px(2.)).p(px(2.)).rounded(px(8.)).bg(t.surface).children(items.iter().enumerate().map(|(i, (label, ic))| {
        let sel = i == on;
        let b = div()
            .id((id, i))
            .px(px(9.))
            .py(px(4.))
            .rounded(px(6.))
            .text_size(px(12.))
            .line_height(relative(1.25))
            .flex()
            .items_center()
            .text_color(if sel { t.text } else { t.text_3 })
            .when(sel, |d| d.bg(t.surface_active))
            .when_some(*ic, |d, ic| d.child(icon(ic, 13., if sel { t.text } else { t.text_3 }).my(px(1.5))))
            .when(!label.is_empty(), |d| d.child(*label));
        click(i, b)
    }))
}

/// `.cbtn` (optionally `.primary`).
pub(crate) fn cbtn(id: impl Into<ElementId>, primary: bool, t: Theme) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(11.))
        .py(px(6.))
        .rounded(px(8.))
        .border_1()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .line_height(relative(1.2))
        .when(primary, |d| d.bg(t.accent).text_color(white()).border_color(transparent_black()))
        .when(!primary, |d| d.bg(t.surface_active).border_color(t.border).hover(|s| s.border_color(tr(t.accent, 0.55))))
}

/// `.st` — the M / A / D status chip.
pub(crate) fn st_badge(s: char, t: Theme) -> Div {
    let c = match s {
        'A' => t.green,
        'D' => t.red,
        _ => t.yellow,
    };
    div()
        .size(px(16.))
        .flex_none()
        .rounded(px(4.))
        .flex()
        .items_center()
        .justify_center()
        .bg(tr(c, 0.14))
        .text_color(c)
        .font_family(MONO)
        .font_weight(FontWeight::SEMIBOLD)
        .text_size(px(10.))
        .child(s.to_string())
}

/// A native-looking 13px checkbox.
pub(crate) fn checkbox(on: bool, t: Theme) -> Div {
    div()
        .size(px(13.))
        .flex_none()
        .rounded(px(3.))
        .flex()
        .items_center()
        .justify_center()
        .when(on, |d| d.bg(t.accent).child(icon("check", 10., white())))
        .when(!on, |d| d.border_1().border_color(t.text_3).bg(tr(t.text, 0.04)))
}

pub(crate) fn kind_name(kind: TabKind) -> &'static str {
    match kind {
        TabKind::Chat => "Chat",
        TabKind::Browser => "Browser",
        TabKind::Editor => "Editor",
        TabKind::Terminal => "Terminal",
        TabKind::Diff => "Changes",
        TabKind::Markdown => "Markdown",
        TabKind::Whiteboard => "Whiteboard",
        TabKind::Emulator => "Emulator",
        #[allow(unreachable_patterns)]
        _ => "New Tab",
    }
}

impl TakoApp {
    /// Entry point: everything inside a non-chat session tab.
    pub fn render_tool_tab(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let is_picker = st(|s| s.picker.tab == Some(tab.id));
        #[allow(unreachable_patterns)]
        let body: AnyElement = match tab.kind {
            _ if is_picker => self.render_new_tab_picker(tab.id, window, cx).into_any_element(),
            TabKind::Diff => self.render_diff(tab, window, cx).into_any_element(),
            TabKind::Browser => self.render_browser(tab, window, cx).into_any_element(),
            TabKind::Terminal => self.render_terminal(tab, window, cx).into_any_element(),
            TabKind::Editor => self.render_editor(tab, window, cx).into_any_element(),
            TabKind::Markdown => self.render_markdown(tab, window, cx).into_any_element(),
            TabKind::Whiteboard => self.render_whiteboard(tab, window, cx).into_any_element(),
            TabKind::Emulator => self.render_emulator(tab, window, cx).into_any_element(),
            // TabKind::NewTab (the ⌘T launcher) and anything else
            _ => self.render_new_tab_picker(tab.id, window, cx).into_any_element(),
        };
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .bg(t.bg)
            .line_height(relative(1.21))
            .text_size(px(14.))
            // No entry fade: the mock's `tvIn` (.25s) replayed on every tab switch and made
            // switching feel slow, so tool tabs appear instantly.
            .child(body)
    }

    /// Insert a tool tab after the active one and focus it (the mock's `open()`).
    pub fn open_tool_tab(&mut self, kind: TabKind, name: impl Into<String>, cx: &mut Context<Self>) -> usize {
        let id = self.chat.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let pos = self.chat.tabs.iter().position(|t| t.id == self.chat.active_tab).map(|p| p + 1).unwrap_or(self.chat.tabs.len());
        self.chat.tabs.insert(pos, Tab { id, kind, name: name.into() });
        self.chat.active_tab = id;
        cx.notify();
        id
    }

    /// Default tab title for a kind, like the mock's `title()`.
    pub fn default_tab_name(kind: TabKind) -> String {
        match kind {
            TabKind::Browser => "localhost:3000".into(),
            TabKind::Editor => "Timeline.tsx".into(),
            TabKind::Markdown => "PLAN.md".into(),
            k => kind_name(k).into(),
        }
    }

    pub(crate) fn rename_tab(&mut self, id: usize, name: impl Into<String>) {
        if let Some(t) = self.chat.tabs.iter_mut().find(|t| t.id == id) {
            t.name = name.into();
        }
    }

    /// Open an editor on `path` (reuses an editor already showing it).
    pub fn open_file_in_editor(&mut self, path: &str, cx: &mut Context<Self>) {
        if path.ends_with(".md") {
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            let id = self.open_tool_tab(TabKind::Markdown, name.clone(), cx);
            st(|s| s.md_file(id, name));
            return;
        }
        if let Some(id) = st(|s| s.editor.iter().find(|(_, e)| e.path == path).map(|(id, _)| *id)) {
            if self.chat.tabs.iter().any(|t| t.id == id) {
                self.chat.active_tab = id;
                cx.notify();
                return;
            }
        }
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let id = self.open_tool_tab(TabKind::Editor, name, cx);
        st(|s| s.editor.insert(id, editor::EditorTab::new(path)));
    }

    fn first_tab(&self, kind: TabKind) -> Option<usize> {
        let act = self.chat.active_tab;
        if self.chat.tabs.iter().any(|t| t.id == act && t.kind == kind) {
            return Some(act);
        }
        self.chat.tabs.iter().find(|t| t.kind == kind).map(|t| t.id)
    }
}

impl ToolsState {
    fn md_file(&mut self, id: usize, name: String) {
        self.md.entry(id).or_default().file = Some(name);
    }
}

fn parse_kind(v: &str) -> Option<TabKind> {
    Some(match v {
        "browser" => TabKind::Browser,
        "editor" => TabKind::Editor,
        "terminal" => TabKind::Terminal,
        "diff" | "changes" => TabKind::Diff,
        "markdown" | "md" => TabKind::Markdown,
        "whiteboard" | "wb" => TabKind::Whiteboard,
        "emulator" | "emu" => TabKind::Emulator,
        _ => return None,
    })
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
///
/// - `opentab:<browser|editor|terminal|diff|markdown|whiteboard|emulator>` — push + focus a tab
/// - `newtab` — open the ⌘T launcher; `pick:<letter>` picks in it
/// - `term:<command>` — run a command in the (first) terminal tab
/// - `url:<url>` — navigate the browser tab
/// - `edit:<path>` — open a file in the editor; `diff:<path>` single-file diff
/// - `viewed:<i>` / `dfold:<i>` in the active Changes tab
/// - `md:<preview|split|source>`, `emu:<phone|tablet|rotate|reload>`, `wb:<pen|hl|sticky|clear|scribble>`
pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "opentab" => {
            let Some(kind) = parse_kind(v) else { return false };
            app.open_tool_tab(kind, TakoApp::default_tab_name(kind), cx);
        }
        "newtab" => app.open_new_tab(cx),
        "pick" => {
            if let Some(tab) = st(|s| s.picker.tab) {
                if let Some(i) = PICKER_ITEMS.iter().position(|it| it.key.eq_ignore_ascii_case(v)) {
                    app.pick_new_tab(tab, i, window, cx);
                }
            }
        }
        "term" => {
            let id = app.first_tab(TabKind::Terminal).unwrap_or_else(|| app.open_tool_tab(TabKind::Terminal, "Terminal", cx));
            app.chat.active_tab = id;
            app.ensure_term(id, window, cx);
            app.term_run(id, v.to_string(), cx);
        }
        "url" => {
            let id = app.first_tab(TabKind::Browser).unwrap_or_else(|| app.open_tool_tab(TabKind::Browser, v, cx));
            app.chat.active_tab = id;
            app.ensure_browser(id, v, window, cx);
            app.browser_go(id, v.to_string(), true, window, cx);
        }
        "edit" => app.open_file_in_editor(v, cx),
        "diff" => app.open_diff_file(v, cx),
        "viewed" | "dfold" => {
            let Some(id) = app.first_tab(TabKind::Diff) else { return false };
            let i: usize = v.parse().unwrap_or(0);
            if k == "viewed" {
                app.diff_toggle_viewed(id, i);
            } else {
                st(|s| {
                    let d = s.diff.entry(id).or_default();
                    if !d.folded.remove(&i) {
                        d.folded.insert(i);
                    }
                });
            }
        }
        "md" => {
            let Some(id) = app.first_tab(TabKind::Markdown) else { return false };
            st(|s| s.md.entry(id).or_default().mode = markdown::MdMode::parse(v));
        }
        "emu" => {
            let Some(id) = app.first_tab(TabKind::Emulator) else { return false };
            emulator::emu_act(id, v);
        }
        "wb" => {
            let Some(id) = app.first_tab(TabKind::Whiteboard) else { return false };
            whiteboard::wb_act(id, v);
        }
        _ => return false,
    }
    cx.notify();
    true
}
