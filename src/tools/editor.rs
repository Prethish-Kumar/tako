//! Editor tab: breadcrumb, gutter with modified-line marks, highlighted code,
//! the agent's cursor, and a status bar (the mock's `VIEWS.editor`).

use super::{data::*, st};
use crate::{app::TakoApp, chat::Tab, theme::ActiveTheme, ui::*};
use gpui_kit::{
    component::input::{InputEvent, Textarea, TextareaState},
    prelude::FluentBuilder as _,
    *,
};
use std::time::Duration;

pub struct EditorTab {
    pub path: String,
    pub text: String,
    /// Live while you're editing (click the code); highlighted view otherwise.
    pub editing: Option<Entity<TextareaState>>,
}

impl EditorTab {
    pub fn new(path: &str) -> Self {
        EditorTab { path: path.into(), text: file_source(path), editing: None }
    }
}

const LH: f32 = 12.5 * 1.7;

impl TakoApp {
    fn editor_start_edit(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let text = st(|s| s.editor.get(&id).map(|e| e.text.clone())).unwrap_or_default();
        let ta = cx.new(|cx| TextareaState::new(window, cx));
        ta.update(cx, |s, cx| {
            s.set_value(text, window, cx);
            s.focus(window, cx);
        });
        cx.subscribe_in(&ta, window, move |_this: &mut TakoApp, ta, ev: &InputEvent, _, cx| match ev {
            InputEvent::Change => {
                let v = ta.read(cx).value().to_string();
                st(|s| s.editor.get_mut(&id).map(|e| e.text = v));
                cx.notify();
            }
            InputEvent::Blur => {
                st(|s| s.editor.get_mut(&id).map(|e| e.editing = None));
                cx.notify();
            }
            _ => {}
        })
        .detach();
        st(|s| s.editor.get_mut(&id).map(|e| e.editing = Some(ta)));
        cx.notify();
    }

    pub(super) fn render_editor(&mut self, tab: &Tab, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        let (path, text, editing) = st(|s| {
            let e = s.editor.entry(id).or_insert_with(|| {
                let p = if tab.name.contains('/') { tab.name.clone() } else { "src/components/chat/Timeline.tsx".into() };
                EditorTab::new(&p)
            });
            (e.path.clone(), e.text.clone(), e.editing.clone())
        });
        let n = text.split('\n').count();
        let modl: &[usize] = if path.ends_with("Timeline.tsx") {
            &[10, 11, 12, 18]
        } else if path.ends_with(".test.ts") {
            &[9, 10, 11, 12]
        } else {
            &[]
        };
        let lang = if path.ends_with(".tsx") {
            "TypeScript React"
        } else if path.ends_with(".ts") {
            "TypeScript"
        } else {
            "Plain text"
        };
        let parts: Vec<String> = path.split('/').map(String::from).collect();
        let crumb = div()
            .flex()
            .flex_none()
            .items_center()
            .px(px(16.))
            .py(px(9.))
            .text_size(px(12.))
            .text_color(t.text_3)
            .border_b_1()
            .border_color(t.border)
            .children(parts.iter().enumerate().map(|(i, p)| {
                div().flex().when(i > 0, |d| d.child(div().mx(px(6.)).opacity(0.6).child("›"))).child(p.clone())
            }));

        let gutter = div()
            .flex_none()
            .pt(px(14.))
            .pr(px(10.))
            .pb(px(40.))
            .pl(px(16.))
            .text_color(t.text_3)
            .children((1..=n).map(|i| {
                let m = modl.contains(&i);
                div()
                    .relative()
                    .h(px(LH))
                    .flex()
                    .justify_end()
                    .child(div().opacity(0.7).child(i.to_string()))
                    .when(m, |d| d.child(div().absolute().left(px(-12.)).top(px(3.)).bottom(px(3.)).w(px(2.)).rounded(px(2.)).bg(t.accent)))
            }));

        let code: AnyElement = if let Some(ta) = editing {
            div().flex_1().pt(px(14.)).pr(px(24.)).pb(px(40.)).pl(px(8.)).child(Textarea::new(&ta).appearance(false).bordered(false).p_0().text_size(px(12.5))).into_any_element()
        } else {
            div()
                .id(("ed-code", id))
                .flex_1()
                .pt(px(14.))
                .pr(px(24.))
                .pb(px(40.))
                .pl(px(8.))
                .cursor_text()
                .children(text.split('\n').map(|l| div().h(px(LH)).whitespace_nowrap().child(hl(l, &t, t.text))))
                .on_click(cx.listener(move |this, _, window, cx| this.editor_start_edit(id, window, cx)))
                .into_any_element()
        };

        // the agent's cursor: a label over a blinking caret at its line
        let at = modl.first().copied().unwrap_or(3);
        let top = (at - 1) as f32 * LH + 14.;
        let agent = div()
            .absolute()
            .left(px(260.))
            .top(px(top - 12.))
            .flex()
            .items_center()
            .gap(px(4.))
            .pl(px(4.))
            .pr(px(6.))
            .py(px(1.))
            .rounded(px(4.))
            .bg(t.accent_2)
            .text_color(white())
            .font_family(FONT)
            .text_size(px(10.))
            .line_height(relative(1.2))
            .child("Agent")
            .child(
                div().absolute().left_0().top(px(14.)).w(px(2.)).h(px(17.)).bg(t.accent_2).with_animation(
                    "ed-caret",
                    Animation::new(Duration::from_millis(1100)).repeat(),
                    |d, p| d.opacity(if p < 0.5 { 1. } else { 0. }),
                ),
            );

        let status = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.))
            .px(px(14.))
            .py(px(5.))
            .border_t_1()
            .border_color(t.border)
            .text_size(px(11.5))
            .text_color(t.text_3)
            .child("Ln 1, Col 1")
            .child(lang)
            .child("UTF-8")
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(6.)).rounded_full().bg(t.accent_2).with_animation(
                        "ed-pulse",
                        Animation::new(Duration::from_millis(1600)).repeat(),
                        move |d, p| {
                            let r = 7. * (p / 0.7).min(1.);
                            let a = if p < 0.7 { 0.5 * (1. - p / 0.7) } else { 0. };
                            d.shadow(vec![BoxShadow { color: t.accent_2.alpha(a), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(r), inset: false }])
                        },
                    ))
                    .child("Agent is editing"),
            );

        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(crumb)
            .child(
                div()
                    .id(("ed-scroll", id))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .font_family(MONO)
                    .text_size(px(12.5))
                    .line_height(px(LH))
                    .child(div().relative().flex().min_w_full().child(gutter).child(code).child(agent)),
            )
            .child(status)
    }
}
