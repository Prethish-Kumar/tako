//! The ⌘T "New Tab" launcher: a quiet, keyboard-first list (the mock's
//! `VIEWS.newtab`). Picking a row turns the launcher tab into that tool.

use super::{mix, snap_mode, st};
use crate::{
    app::TakoApp,
    chat::TabKind,
    theme::ActiveTheme,
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::time::Instant;

pub struct PickerItem {
    pub key: &'static str,
    pub kind: TabKind,
    pub fork: bool,
    pub name: &'static str,
    pub icon: &'static str,
    /// From the mock's TYPES table (not shown by the launcher, but handy for menus).
    pub desc: &'static str,
}

pub static PICKER_ITEMS: &[PickerItem] = &[
    PickerItem { key: "N", kind: TabKind::Chat, fork: false, name: "New chat", icon: "spark", desc: "A new agent session in this worktree" },
    PickerItem { key: "K", kind: TabKind::Chat, fork: true, name: "Fork chat", icon: "branch", desc: "A new agent session with this chat's history" },
    PickerItem { key: "B", kind: TabKind::Browser, fork: false, name: "Browser", icon: "globe", desc: "Preview localhost or any URL" },
    PickerItem { key: "F", kind: TabKind::Editor, fork: false, name: "File editor", icon: "code", desc: "Edit files in the worktree" },
    PickerItem { key: "T", kind: TabKind::Terminal, fork: false, name: "Terminal", icon: "term", desc: "A shell in this session's worktree" },
    PickerItem { key: "C", kind: TabKind::Diff, fork: false, name: "Changes", icon: "diff", desc: "Review the diff before merging" },
    PickerItem { key: "M", kind: TabKind::Markdown, fork: false, name: "Markdown", icon: "doc", desc: "Read and write .md docs" },
    PickerItem { key: "W", kind: TabKind::Whiteboard, fork: false, name: "Whiteboard", icon: "pen", desc: "Sketch it out with the agent" },
    PickerItem { key: "E", kind: TabKind::Emulator, fork: false, name: "Emulator", icon: "phone", desc: "Run the app on a virtual device" },
];

#[derive(Default)]
pub struct PickerState {
    /// The tab currently showing the launcher (if any).
    pub tab: Option<usize>,
    pub sel: usize,
    pub focus: Option<FocusHandle>,
    pub shown: Option<(usize, Instant)>,
    focused_for: Option<usize>,
}

impl TakoApp {
    /// ⌘T / the "+" button: open a "New Tab" launcher after the active tab.
    ///
    /// Until chat.rs has a `TabKind::NewTab`, the launcher rides on a tab of any
    /// non-chat kind named "New Tab"; `render_tool_tab` shows the picker for it.
    pub fn open_new_tab(&mut self, cx: &mut Context<Self>) {
        let id = self.open_tool_tab(TabKind::NewTab, "New Tab", cx);
        st(|s| {
            s.picker.tab = Some(id);
            s.picker.sel = 0;
        });
    }

    /// Turn launcher tab `tab` into the picked tool (or open it as a new tab if
    /// `tab` isn't in the strip).
    pub fn pick_new_tab(&mut self, tab: usize, i: usize, _: &mut Window, cx: &mut Context<Self>) {
        let it = &PICKER_ITEMS[i];
        let name = if it.kind == TabKind::Chat {
            if it.fork { "Fork of Chat".to_string() } else { "New chat".to_string() }
        } else {
            TakoApp::default_tab_name(it.kind)
        };
        st(|s| {
            if s.picker.tab == Some(tab) {
                s.picker.tab = None;
                s.picker.focused_for = None;
            }
            // a fresh tool in a reused id
            s.browser.remove(&tab);
            s.term.remove(&tab);
            s.diff.remove(&tab);
            s.editor.remove(&tab);
            s.md.remove(&tab);
            s.wb.remove(&tab);
            s.emu.remove(&tab);
        });
        if let Some(t) = self.chat.tabs.iter_mut().find(|t| t.id == tab) {
            t.kind = it.kind;
            t.name = name;
            self.chat.active_tab = tab;
        } else {
            self.open_tool_tab(it.kind, name, cx);
        }
        cx.notify();
    }

    /// The launcher UI for tab `tab`.
    pub fn render_new_tab_picker(&mut self, tab: usize, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let (focus, sel, shown_ms, needs_focus) = st(|s| {
            let p = &mut s.picker;
            p.tab = Some(tab);
            let f = p.focus.get_or_insert_with(|| cx.focus_handle()).clone();
            if p.shown.map(|(id, _)| id) != Some(tab) {
                p.shown = Some((tab, Instant::now()));
            }
            let nf = p.focused_for != Some(tab);
            p.focused_for = Some(tab);
            (f, p.sel, p.shown.unwrap().1.elapsed().as_secs_f32() * 1000., nf)
        });
        if needs_focus {
            window.focus(&focus, cx);
        }
        let n = PICKER_ITEMS.len();
        if shown_ms < 45. * 22. + 450. {
            window.request_animation_frame();
        }
        let vh = f32::from(window.viewport_size().height);
        let rows = PICKER_ITEMS.iter().enumerate().map(|(i, it)| {
            let on = i == sel;
            // `rise` .45s, staggered 22ms
            let p = if snap_mode() { 1. } else { ease_snappy(((shown_ms - 22. * i as f32) / 450.).clamp(0., 1.)) };
            div()
                .id(("nt-row", i))
                .relative()
                .top(px(16. * (1. - p)))
                .opacity(p)
                .flex()
                .items_center()
                .gap(px(14.))
                .w_full()
                .h(px(40.))
                .pl(px(12.))
                .pr(px(8.))
                .rounded(px(9.))
                .text_size(px(14.))
                .text_color(t.text)
                .when(on, |d| d.bg(t.surface_hover))
                .child(icon(it.icon, 18., if on { t.accent } else { t.text_2 }))
                .child(div().flex_1().child(it.name))
                .child(
                    div()
                        .size(px(22.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.5))
                        .bg(if on { mix(t.accent, t.surface_active, 0.18) } else { t.surface_active })
                        .text_color(if on { t.text } else { t.text_3 })
                        .child(it.key),
                )
                .on_mouse_move(move |_, window, _| {
                    if st(|s| std::mem::replace(&mut s.picker.sel, i) != i) {
                        window.refresh();
                    }
                })
                .on_click(cx.listener(move |this, _, window, cx| this.pick_new_tab(tab, i, window, cx)))
        });
        div().id(("tv-new", tab)).flex_1().min_w_0().h_full().overflow_y_scroll().flex().justify_center().items_start().child(
            div()
                .id("nt-list")
                .track_focus(&focus)
                .w(px(420.))
                .max_w(relative(0.9))
                .mt(px(vh * 0.14))
                .px(px(10.))
                .py(px(14.))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                    let ks = &ev.keystroke;
                    if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                        return;
                    }
                    let k = ks.key.as_str();
                    if let Some(i) = PICKER_ITEMS.iter().position(|it| it.key.eq_ignore_ascii_case(k)) {
                        cx.stop_propagation();
                        return this.pick_new_tab(tab, i, window, cx);
                    }
                    match k {
                        "down" => st(|s| s.picker.sel = (s.picker.sel + 1) % n),
                        "up" => st(|s| s.picker.sel = (s.picker.sel + n - 1) % n),
                        "enter" => {
                            let i = st(|s| s.picker.sel);
                            return this.pick_new_tab(tab, i, window, cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().text_size(px(12.5)).text_color(t.text_3).px(px(12.)).pb(px(10.)).child("Open"))
                .children(rows),
        )
    }
}

