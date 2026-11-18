//! Popovers and full-window overlays.
pub mod onboarding;
pub mod profile;
pub mod theme_pop;
pub mod usage;

use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::*;

pub struct OverlayState {
    pub theme: theme_pop::ThemePopState,
    pub profile: profile::ProfileState,
    pub usage: usage::UsageState,
    pub onboarding: onboarding::OnboardingState,
}

impl OverlayState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        OverlayState {
            theme: theme_pop::ThemePopState::new(window, cx),
            profile: profile::ProfileState::new(window, cx),
            usage: usage::UsageState::new(window, cx),
            onboarding: onboarding::OnboardingState::new(window, cx),
        }
    }
}

impl TakoApp {
    pub fn render_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        [
            self.render_theme_pop(window, cx),
            self.render_usage_pop(window, cx),
            self.render_profile(window, cx),
            self.render_onboarding(window, cx),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Actions owned by feature modules.
    pub fn run_ext_action(&mut self, k: &str, v: &str, window: &mut Window, cx: &mut Context<Self>) {
        let _ = theme_pop::act(self, k, v, window, cx)
            || profile::act(self, k, v, window, cx)
            || usage::act(self, k, v, window, cx)
            || onboarding::act(self, k, v, window, cx)
            || crate::pages::act(self, k, v, window, cx)
            || crate::scene::act(self, k, v, window, cx)
            || crate::home::act(self, k, v, window, cx)
            || crate::right::act(self, k, v, window, cx)
            || crate::tools::act(self, k, v, window, cx);
    }

    /// The usage ring button in the rail (owned by overlays/usage.rs).
    pub fn render_usage_ring(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        crate::overlays::usage::usage_ring(self, cx)
    }
}
