//! Stats page (port of the mock's PROFILE → Stats page).
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct StatsState {}

impl StatsState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        StatsState {}
    }
}

impl TakoApp {
    pub fn render_stats(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        div().size_full().flex().items_center().justify_center().text_color(t.text_3).child("Stats (todo)")
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
