//! A small FPS counter (bottom right). Its own view, so ticking it every frame
//! doesn't re-render the app: it asks for the next frame each time it draws and
//! counts how many frames the window actually presented in the last second.

use crate::{theme::ActiveTheme, ui::MONO};
use gpui_kit::*;
use std::{collections::VecDeque, time::Instant};

pub struct Fps {
    frames: VecDeque<Instant>,
    shown: (u32, f32),
    shown_at: Instant,
}

impl Fps {
    pub fn new() -> Self {
        Fps { frames: VecDeque::new(), shown: (0, 0.), shown_at: Instant::now() }
    }
}

impl Render for Fps {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.frames.push_back(now);
        while self.frames.front().is_some_and(|t| now.duration_since(*t).as_secs_f32() > 1.) {
            self.frames.pop_front();
        }
        // refresh the label 4× a second so the number is readable
        if now.duration_since(self.shown_at).as_millis() >= 250 {
            let n = self.frames.len() as u32;
            let ms = if n > 1 {
                now.duration_since(*self.frames.front().unwrap()).as_secs_f32() * 1000. / (n - 1) as f32
            } else {
                0.
            };
            self.shown = (n, ms);
            self.shown_at = now;
        }
        window.request_animation_frame();
        let t = cx.t();
        let (n, ms) = self.shown;
        let col = if n >= 100 { t.green } else if n >= 50 { t.yellow } else { t.red };
        div()
            .absolute()
            .bottom(px(8.))
            .right(px(8.))
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(7.))
            .py(px(3.))
            .rounded(px(6.))
            .bg(t.panel_solid.opacity(0.85))
            .border_1()
            .border_color(t.border)
            .font_family(MONO)
            .text_size(px(10.5))
            .line_height(px(14.))
            .text_color(t.text_2)
            .child(div().size(px(6.)).rounded_full().bg(col))
            .child(format!("{n} fps"))
            .child(div().text_color(t.text_3).child(format!("{ms:.1}ms")))
    }
}
