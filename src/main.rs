#![allow(dead_code, unused_imports)]
mod app;
mod assets;
mod chat;
mod ctx;
mod data;
mod home;
mod overlays;
mod pages;
mod pix;
mod right;
mod scene;
mod sidebar;
mod snap;
mod theme;
mod tools;
mod ui;

use gpui_kit::*;

fn main() {
    gpui_kit::application().with_assets(assets::Assets).run(|cx| {
        gpui_kit::init(cx);
        assets::load_fonts(cx);
        cx.set_global(theme::ThemeState::new());
        app::bind_keys(cx);

        let size = size(px(1440.), px(900.));
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size, cx))),
            titlebar: Some(TitlebarOptions {
                title: Some("Tako".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.), px(16.))),
            }),
            window_min_size: Some(gpui_kit::size(px(900.), px(600.))),
            // Snapshot runs stay off-screen and unfocused.
            show: std::env::var("TAKO_SNAP").is_err(),
            focus: std::env::var("TAKO_SNAP").is_err(),
            ..Default::default()
        };
        gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| app::TakoApp::new(window, cx))).expect("open window");
        if std::env::var("TAKO_SNAP").is_err() {
            cx.activate(true);
        }
    });
}
