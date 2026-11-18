//! Profile menu, sign-out overlay (port of the mock's PROFILE section).
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct ProfileState {
    pub open: bool,
}

impl ProfileState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        ProfileState { open: false }
    }
}

impl TakoApp {
    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_profile(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        None
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
