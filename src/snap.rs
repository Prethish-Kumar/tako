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
            this.update_in(cx, |this, window, cx| this.run_action(&a, window, cx)).ok();
        }
        if let Some(path) = snap {
            cx.background_executor().timer(Duration::from_millis(ms)).await;
            this.update_in(cx, |_, window, cx| {
                window.refresh();
                let _ = cx;
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
