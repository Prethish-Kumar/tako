//! Dev hook: `TAKO_SNAP=out.png` renders the window offscreen to a PNG and quits.
//! `TAKO_VIEW` picks the starting view, `TAKO_ACT` runs scripted actions
//! ("send:text", "fold:tako", "theme", "profile", "tab:2", "scene:aurora"…) and
//! `TAKO_SNAP_MS` sets when the frame is captured.

use crate::app::{TakoApp, View};
use gpui_kit::*;
use std::time::Duration;

pub fn setup(app: &mut TakoApp, window: &mut Window, cx: &mut Context<TakoApp>) {
    if let Ok(v) = std::env::var("TAKO_VIEW") {
        app.view = match v.as_str() {
            "chats" => View::Chats,
            "auto" | "automations" => View::Automations,
            "plugins" => View::Plugins,
            "settings" => View::Settings,
            "stats" => View::Stats,
            _ => View::Home,
        };
    }
    let acts = std::env::var("TAKO_ACT").unwrap_or_default();
    let snap = std::env::var("TAKO_SNAP").ok();
    let ms: u64 = std::env::var("TAKO_SNAP_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(1500);
    cx.spawn_in(window, async move |this, cx| {
        cx.background_executor().timer(Duration::from_millis(300)).await;
        for a in acts.split(';').filter(|s| !s.is_empty()) {
            let a = a.to_string();
            if let Some(w) = a.strip_prefix("wait:") {
                let w: u64 = w.parse().unwrap_or(100);
                cx.background_executor().timer(Duration::from_millis(w)).await;
                continue;
            }
            // Real input, through GPUI's own dispatch (hit-testing / key contexts included):
            //   key:cmd-]      click:x,y (logical px)
            if let Some(k) = a.strip_prefix("key:") {
                let k: &'static str = Box::leak(k.to_string().into_boxed_str());
                cx.update(|window, cx| {
                    window.refresh();
                    let _ = window.draw(cx);
                    if let Ok(ks) = Keystroke::parse(k) {
                        let handled = window.dispatch_keystroke(ks, cx);
                        eprintln!("key {k} handled={handled}");
                    }
                })
                .ok();
                continue;
            }
            if let Some(xy) = a.strip_prefix("click:") {
                let mut it = xy.split(',').filter_map(|v| v.trim().parse::<f32>().ok());
                if let (Some(x), Some(y)) = (it.next(), it.next()) {
                    cx.update(|window, cx| {
                        window.refresh();
                        let _ = window.draw(cx);
                        let position = point(px(x), px(y));
                        let m = Modifiers::default();
                        window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers: m }), cx);
                        window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position, modifiers: m, click_count: 1, first_mouse: false }), cx);
                        window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position, modifiers: m, click_count: 1 }), cx);
                    })
                    .ok();
                }
                continue;
            }
            let r = this.update_in(cx, |this, window, cx| this.run_action(&a, window, cx));
            if r.is_err() {
                eprintln!("action {a} failed");
            }
        }
        if let Some(path) = snap {
            cx.background_executor().timer(Duration::from_millis(ms)).await;
            // Hidden windows don't redraw on their own: force a fresh frame, then read it back.
            cx.update(|window, cx| {
                // two passes: the first lets measured heights (height animations) settle
                window.refresh();
                let _ = window.draw(cx);
                window.refresh();
                let _ = window.draw(cx);
                #[cfg(feature = "snap")]
                match window.render_to_image() {
                    Ok(img) => {
                        img.save(&path).ok();
                        eprintln!("snap saved {path}");
                    }
                    Err(e) => eprintln!("snap failed: {e}"),
                }
            })
            .ok();
            cx.background_executor().timer(Duration::from_millis(50)).await;
            this.update(cx, |_, cx| cx.quit()).ok();
        }
    })
    .detach();
}
