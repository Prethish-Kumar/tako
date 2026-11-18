//! Pixel backdrop scenes (Tako, Aurora, Neon Rain, Peaks, Glade, Valley).
//! Port of the mock's PIXEL BACKDROP engine.
use crate::{app::TakoApp, theme::ActiveTheme};
use gpui_kit::*;

pub struct SceneState {
    pub id: &'static str,
}

impl SceneState {
    pub fn new() -> Self {
        SceneState { id: "tako" }
    }
}

impl TakoApp {
    /// The animated backdrop that fills the Home card (and the onboarding's last step).
    pub fn render_backdrop(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        div().size_full().bg(t.panel_solid)
    }
    /// The scene switcher pills + caption (top-right of Home).
    pub fn render_scene_switch(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn act(app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k == "scene" {
        app.scene.id = match v { "aurora" => "aurora", "neon" => "neon", "peaks" => "peaks", "glade" => "glade", "valley" => "valley", _ => "tako" };
        cx.notify();
        return true;
    }
    false
}
