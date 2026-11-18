//! Session tool tabs: browser, editor, terminal, changes, markdown, whiteboard, emulator.
use gpui_kit::prelude::FluentBuilder as _;
use crate::{app::TakoApp, chat::Tab, theme::ActiveTheme};
use gpui_kit::*;

impl TakoApp {
    pub fn render_tool_tab(&mut self, tab: &Tab, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        div().flex_1().flex().items_center().justify_center().text_color(t.text_3).child(tab.name.clone())
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
