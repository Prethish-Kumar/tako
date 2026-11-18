//! Whiteboard tab: smooth pen / highlighter strokes, sticky notes you can drag
//! and edit, colour swatches, clear (the mock's `VIEWS.whiteboard`).

use super::{hex, mix, st, tr};
use crate::{app::TakoApp, chat::Tab, theme::{ActiveTheme, Theme}, ui::*};
use gpui_kit::{
    component::input::{Textarea, TextareaState},
    prelude::FluentBuilder as _,
    *,
};
use std::{cell::Cell, rc::Rc, time::Instant};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum WbTool {
    #[default]
    Pen,
    Hl,
    Sticky,
}

#[derive(Clone)]
pub struct Stroke {
    pub hl: bool,
    pub color: usize,
    pub pts: Vec<(f32, f32)>,
}

pub struct Note {
    pub x: f32,
    pub y: f32,
    pub hue: Hsla,
    pub text: Entity<TextareaState>,
    pub born: Instant,
}

#[derive(Default)]
pub struct WbState {
    pub tool: WbTool,
    pub color: usize,
    pub strokes: Vec<Stroke>,
    pub notes: Vec<Note>,
    pub drawing: bool,
    pub drag: Option<(usize, f32, f32)>,
    pub bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub seeded: bool,
}

fn swatch(i: usize, t: Theme) -> Hsla {
    match i {
        1 => t.accent,
        2 => hex(0x5fc48a),
        3 => hex(0x7aa7e8),
        _ => t.text,
    }
}

pub fn wb_act(id: usize, v: &str) {
    st(|s| {
        let w = s.wb.entry(id).or_default();
        match v {
            "pen" => w.tool = WbTool::Pen,
            "hl" => w.tool = WbTool::Hl,
            "sticky" => w.tool = WbTool::Sticky,
            "clear" => {
                w.strokes.clear();
                w.notes.clear();
            }
            "c0" | "c1" | "c2" | "c3" => w.color = v[1..].parse().unwrap_or(0),
            "scribble" => {
                let hl = w.tool == WbTool::Hl;
                let pts = (0..60).map(|i| (120. + i as f32 * 7., 420. + (i as f32 / 6.).sin() * 40. + i as f32)).collect();
                w.strokes.push(Stroke { hl, color: w.color, pts });
            }
            _ => {}
        }
    })
}

fn new_note(x: f32, y: f32, text: &str, hue: Hsla, window: &mut Window, cx: &mut Context<TakoApp>) -> Note {
    let ta = cx.new(|cx| TextareaState::new(window, cx));
    ta.update(cx, |s, cx| s.set_value(text.to_string(), window, cx));
    Note { x: x - 80., y: y - 20., hue, text: ta, born: Instant::now() }
}

impl TakoApp {
    pub(super) fn render_whiteboard(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        // seed two notes + an arrow once the board has a size
        let need_seed = st(|s| {
            let w = s.wb.entry(id).or_default();
            if w.seeded {
                return None;
            }
            match w.bounds.get() {
                Some(b) => {
                    w.seeded = true;
                    Some((f32::from(b.size.width), f32::from(b.size.height)))
                }
                None => None,
            }
        });
        if let Some((w, h)) = need_seed {
            let n1 = new_note(w * 0.3, h * 0.3, "Resumed header\n→ no divider", hex(0xe2b75a), window, cx);
            let n2 = new_note(w * 0.62, h * 0.42, "“Worked for” summary\n→ keeps divider", hex(0x5fc48a), window, cx);
            let pts = (0..24)
                .map(|i| {
                    let i = i as f32;
                    (w * 0.39 + i * w * 0.0095, h * 0.36 + (i / 23. * std::f32::consts::PI).sin() * -36. + i * 2.2)
                })
                .collect();
            st(|s| {
                let wb = s.wb.get_mut(&id).unwrap();
                wb.notes.extend([n1, n2]);
                wb.strokes.push(Stroke { hl: false, color: 1, pts });
            });
        }
        let (tool, color, strokes, notes, bounds) = st(|s| {
            let w = &s.wb[&id];
            let notes: Vec<(f32, f32, Hsla, Entity<TextareaState>, f32)> =
                w.notes.iter().map(|n| (n.x, n.y, n.hue, n.text.clone(), n.born.elapsed().as_secs_f32() / 0.4)).collect();
            (w.tool, w.color, w.strokes.clone(), notes, w.bounds.clone())
        });
        if bounds.get().is_none() || notes.iter().any(|n| n.4 < 1.) {
            window.request_animation_frame();
        }

        let b2 = bounds.clone();
        let dot = tr(t.text, 0.13);
        let board = canvas(
            move |b, _, _| b2.set(Some(b)),
            move |b, _, window, _| {
                // dot grid (radial-gradient 1px every 22px)
                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                let mut y = 11.;
                while y < h {
                    let mut x = 11.;
                    while x < w {
                        window.paint_quad(fill(Bounds::new(point(b.origin.x + px(x - 1.), b.origin.y + px(y - 1.)), size(px(2.), px(2.))), dot).corner_radii(px(1.)));
                        x += 22.;
                    }
                    y += 22.;
                }
                for s in &strokes {
                    let p = &s.pts;
                    if p.len() < 2 {
                        continue;
                    }
                    let at = |q: (f32, f32)| point(b.origin.x + px(q.0), b.origin.y + px(q.1));
                    let mut pb = PathBuilder::stroke(px(if s.hl { 16. } else { 2.4 }));
                    pb.move_to(at(p[0]));
                    for i in 1..p.len() - 1 {
                        let mid = ((p[i].0 + p[i + 1].0) / 2., (p[i].1 + p[i + 1].1) / 2.);
                        pb.curve_to(at(mid), at(p[i]));
                    }
                    pb.line_to(at(p[p.len() - 1]));
                    let c = swatch(s.color, t);
                    let c = if s.hl { c.alpha(0.28) } else { c };
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, c);
                    }
                    // round caps
                    let r = if s.hl { 8. } else { 1.2 };
                    for q in [p[0], p[p.len() - 1]] {
                        window.paint_quad(fill(Bounds::new(point(b.origin.x + px(q.0 - r), b.origin.y + px(q.1 - r)), size(px(2. * r), px(2. * r))), c).corner_radii(px(r)));
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let local = move |pos: Point<Pixels>| -> (f32, f32) {
            let o = bounds.get().map(|b| b.origin).unwrap_or_default();
            (f32::from(pos.x - o.x), f32::from(pos.y - o.y))
        };
        let (l1, l2) = (local.clone(), local.clone());

        let tool_btn = |key: &'static str, ic: &'static str, on: bool, f: WbTool| {
            div()
                .id(key)
                .size(px(30.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .when(on, |d| d.bg(t.surface_active))
                .when(!on, |d| d.hover(|s| s.bg(t.surface_hover)))
                .child(icon(ic, 16., if on { t.text } else { t.text_3 }))
                .on_click(move |_, window, _| {
                    st(|s| s.wb.get_mut(&id).map(|w| w.tool = f));
                    window.refresh();
                })
        };
        let sep = || div().w(px(1.)).h(px(18.)).bg(t.border).mx(px(4.));
        let tools = div()
            .id("wb-tools")
            .flex()
            .items_center()
            .gap(px(3.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(t.panel_solid.alpha(0.9))
            .border_1()
            .border_color(t.border)
            .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.2), offset: point(px(0.), px(10.)), blur_radius: px(30.), spread_radius: px(0.), inset: false }])
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(tool_btn("wb-pen", "pen", tool == WbTool::Pen, WbTool::Pen))
            .child(tool_btn("wb-hl", "edit", tool == WbTool::Hl, WbTool::Hl))
            .child(tool_btn("wb-sticky", "sticky", tool == WbTool::Sticky, WbTool::Sticky))
            .child(sep())
            .children((0..4).map(|i| {
                let c = swatch(i, t);
                let on = i == color;
                div()
                    .id(("wb-sw", i))
                    .size(px(30.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_hover))
                    .child(div().size(px(14.)).rounded_full().bg(c).shadow(if on {
                        vec![
                            BoxShadow { color: c, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(3.5), inset: false },
                            BoxShadow { color: t.panel_solid, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false },
                        ]
                    } else {
                        vec![BoxShadow { color: t.border, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(1.), inset: false }]
                    }))
                    .on_click(move |_, window, _| {
                        st(|s| s.wb.get_mut(&id).map(|w| w.color = i));
                        window.refresh();
                    })
            }))
            .child(sep())
            .child(
                div()
                    .id("wb-clear")
                    .size(px(30.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_hover))
                    .child(icon("eraser", 16., t.text_3))
                    .on_click(move |_, window, _| {
                        wb_act(id, "clear");
                        window.refresh();
                    }),
            );

        let note_els = notes.into_iter().enumerate().map(|(i, (x, y, hue, ta, p))| {
            let e = ease_snappy(p.min(1.));
            div()
                .absolute()
                .left(px(x))
                .top(px(y + 16. * (1. - e)))
                .opacity(e)
                .w(px(170.))
                .min_h(px(90.))
                .rounded_tl(px(4.))
                .rounded_tr(px(4.))
                .rounded_br(px(4.))
                .rounded_bl(px(14.))
                .bg(mix(hue, t.panel_solid, 0.22))
                .border_1()
                .border_color(tr(hue, 0.4))
                .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.18), offset: point(px(0.), px(10.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .id(("wb-grip", i))
                        .h(px(14.))
                        .border_b_1()
                        .border_dashed()
                        .border_color(tr(hue, 0.35))
                        .cursor_grab()
                        .on_mouse_down(MouseButton::Left, {
                            let l = local.clone();
                            move |ev: &MouseDownEvent, _, cx| {
                                let (mx, my) = l(ev.position);
                                st(|s| s.wb.get_mut(&id).map(|w| w.drag = Some((i, mx - x, my - y))));
                                cx.stop_propagation();
                            }
                        }),
                )
                .child(
                    div().px(px(12.)).pt(px(8.)).pb(px(12.)).text_size(px(13.)).line_height(relative(1.45)).text_color(t.text).child(
                        Textarea::new(&ta).appearance(false).bordered(false).p_0().text_size(px(13.)),
                    ),
                )
        });

        div()
            .id(("wb", id))
            .flex_1()
            .min_w_0()
            .h_full()
            .relative()
            .overflow_hidden()
            .bg(t.bg)
            .cursor_crosshair()
            .child(board)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                    let (x, y) = l1(ev.position);
                    let sticky = st(|s| s.wb.get(&id).is_some_and(|w| w.tool == WbTool::Sticky));
                    if sticky || ev.click_count >= 2 {
                        let _ = this;
                        let n = new_note(x, y, "New note", t.accent, window, cx);
                        n.text.update(cx, |s, cx| s.focus(window, cx));
                        st(|s| {
                            let w = s.wb.get_mut(&id).unwrap();
                            // a double-click's first press already began a dot stroke
                            if ev.click_count >= 2 && w.strokes.last().is_some_and(|s| s.pts.len() <= 2) {
                                w.strokes.pop();
                            }
                            w.notes.push(n);
                            w.tool = WbTool::Pen;
                            w.drawing = false;
                        });
                    } else {
                        st(|s| {
                            let w = s.wb.get_mut(&id).unwrap();
                            w.drawing = true;
                            w.strokes.push(Stroke { hl: w.tool == WbTool::Hl, color: w.color, pts: vec![(x, y)] });
                        });
                    }
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(move |_, ev: &MouseMoveEvent, _, cx| {
                let (x, y) = l2(ev.position);
                let changed = st(|s| {
                    let w = s.wb.get_mut(&id)?;
                    if let Some((i, ox, oy)) = w.drag {
                        let n = w.notes.get_mut(i)?;
                        n.x = x - ox;
                        n.y = y - oy;
                        return Some(());
                    }
                    if w.drawing {
                        w.strokes.last_mut()?.pts.push((x, y));
                        return Some(());
                    }
                    None
                });
                if changed.is_some() {
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, move |_, _, _| {
                st(|s| {
                    if let Some(w) = s.wb.get_mut(&id) {
                        w.drawing = false;
                        w.drag = None;
                    }
                })
            })
            .children(note_els)
            .child(div().absolute().top(px(14.)).left_0().right_0().flex().justify_center().child(tools))
            .child(
                div()
                    .absolute()
                    .bottom(px(14.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .text_size(px(11.5))
                    .text_color(t.text_3)
                    .child("Draw anywhere · double-click for a note"),
            )
    }
}
