//! Right panel: the git / environment panel for the active worktree.
use gpui_kit::prelude::FluentBuilder as _;
use crate::{app::TakoApp, theme::ActiveTheme};
use gpui_kit::*;

pub struct RightState {}
impl RightState {
    pub fn new(_: &mut Window, _: &mut Context<TakoApp>) -> Self {
        RightState {}
    }
}
impl TakoApp {
    pub fn render_right(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        div().w(px(340.)).h_full().flex_none().border_l_1().border_color(t.border)
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
