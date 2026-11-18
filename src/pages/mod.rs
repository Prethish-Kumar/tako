//! Full-width pages reached from the rail / profile menu.
pub mod automations;
pub mod plugins;
pub mod settings;
pub mod stats;

use crate::app::TakoApp;
use gpui_kit::*;

pub struct PagesState {
    pub auto: automations::AutoState,
    pub plugins: plugins::PluginsState,
    pub settings: settings::SettingsState,
    pub stats: stats::StatsState,
}

impl PagesState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        PagesState {
            auto: automations::AutoState::new(window, cx),
            plugins: plugins::PluginsState::new(window, cx),
            settings: settings::SettingsState::new(window, cx),
            stats: stats::StatsState::new(window, cx),
        }
    }
}

pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    automations::act(app, k, v, window, cx) || plugins::act(app, k, v, window, cx) || settings::act(app, k, v, window, cx) || stats::act(app, k, v, window, cx)
}
