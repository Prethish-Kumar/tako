//! Home: the scene backdrop, composer, starter chips and the PR heatmap card.
use gpui_kit::prelude::FluentBuilder as _;
use crate::{app::TakoApp, theme::ActiveTheme};
use gpui_kit::*;

pub struct HomeState {}
impl HomeState {
    pub fn new(_: &mut Window, _: &mut Context<TakoApp>) -> Self {
        HomeState {}
    }
}
impl TakoApp {
    pub fn render_home(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        div().size_full().rounded(px(12.)).bg(t.panel_solid).flex().flex_col().justify_center().child(self.render_composer(true, window, cx))
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
