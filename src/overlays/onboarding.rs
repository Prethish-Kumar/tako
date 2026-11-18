//! Onboarding: the four-beat pixel story (port of the mock's ONBOARDING section).
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct OnboardingState {
    pub open: bool,
}

impl OnboardingState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        OnboardingState { open: false }
    }
}

impl TakoApp {
    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_onboarding(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        None
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
pub fn act(_app: &mut TakoApp, _k: &str, _v: &str, _window: &mut Window, _cx: &mut Context<TakoApp>) -> bool {
    false
}
