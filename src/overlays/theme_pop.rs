//! Theme picker popover (port of the mock's THEME ENGINE picker).
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct ThemePopState {
    pub open: bool,
}

impl ThemePopState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        ThemePopState { open: false }
    }
}

impl TakoApp {
    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_theme_pop(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        None
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
