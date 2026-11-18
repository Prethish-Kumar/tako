//! Usage rings in the rail + usage popover (port of the mock's USAGE section).
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct UsageState {
    pub open: bool,
}

impl UsageState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        UsageState { open: false }
    }
}

impl TakoApp {
    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_usage_pop(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        None
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}

/// The rail button: Claude + Codex rings. Clicking toggles the popover.
pub fn usage_ring(app: &mut TakoApp, cx: &mut Context<TakoApp>) -> impl IntoElement {
    let t = cx.t();
    let _ = app;
    div().id("usage").size(px(36.)).rounded(px(9.)).flex().items_center().justify_center().child(spinner("usage-spin", 18., t.text_3))
}
