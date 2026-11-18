//! Chat view: session tab strip, the transcript feed (with the smooth
//! "open space, then fade in" animation) and the two-tier composer.

use gpui_kit::prelude::FluentBuilder as _;
use crate::{
    app::TakoApp,
    data::{self, Block, Thread},
    pix::pix,
    theme::{ActiveTheme, Theme},
    ui::*,
};
use gpui_kit::{
    component::input::{InputEvent, Textarea, TextareaState},
    *,
};
use std::{cell::Cell, collections::HashSet, rc::Rc, time::Duration};

pub struct FeedItem {
    pub key: usize,
    pub block: Block,
    pub enter: Option<(Tween, usize)>, // tween + stagger index
    pub leave: Option<Tween>,
    pub h: Rc<Cell<f32>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Chat,
    Browser,
    Editor,
    Terminal,
    Diff,
    Markdown,
    Whiteboard,
    Emulator,
}

#[derive(Clone)]
pub struct Tab {
    pub id: usize,
    pub kind: TabKind,
    pub name: String,
}

pub struct ChatState {
    pub items: Vec<FeedItem>,
    next_key: usize,
    pub open_tools: HashSet<usize>,
    pub scroll: ScrollHandle,
    scroll_anim: Option<(Tween, f32)>,
    pub input: Entity<TextareaState>,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub running: bool,
    thread_id: usize,
    pending_send: bool,
}

impl ChatState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).placeholder("Ask for follow-up changes"));
        cx.subscribe_in(&input, window, |this: &mut TakoApp, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { secondary: false, shift: false } = ev {
                this.chat.pending_send = true;
                this.send(window, cx);
            }
        })
        .detach();
        let tabs = vec![
            Tab { id: 0, kind: TabKind::Chat, name: "Chat".into() },
            Tab { id: 1, kind: TabKind::Chat, name: "Review".into() },
            Tab { id: 2, kind: TabKind::Diff, name: "Changes".into() },
            Tab { id: 3, kind: TabKind::Browser, name: "localhost:3000".into() },
            Tab { id: 4, kind: TabKind::Terminal, name: "Terminal".into() },
        ];
        ChatState {
            items: vec![],
            next_key: 0,
            open_tools: HashSet::new(),
            scroll: ScrollHandle::new(),
            scroll_anim: None,
            input,
            tabs,
            active_tab: 0,
            running: false,
            thread_id: 0,
            pending_send: false,
        }
    }

    pub fn load(&mut self, threads: &[Thread], id: usize) {
        let th = threads.iter().find(|t| t.id == id).unwrap_or(&threads[0]);
        self.thread_id = id;
        self.items.clear();
        self.open_tools.clear();
        for b in data::transcript(th) {
            let k = self.key();
            self.items.push(FeedItem { key: k, block: b, enter: None, leave: None, h: Rc::default() });
        }
        self.running = matches!(self.items.last().map(|i| &i.block), Some(Block::Thinking));
        self.scroll_anim = None;
        self.scroll.scroll_to_bottom();
    }

    fn key(&mut self) -> usize {
        self.next_key += 1;
        self.next_key
    }

    fn live(&self) -> impl Iterator<Item = &FeedItem> {
        self.items.iter().filter(|i| i.leave.is_none())
    }

    fn near_bottom(&self) -> bool {
        let off = -f32::from(self.scroll.offset().y);
        let max = f32::from(self.scroll.max_offset().y);
        max - off < 140.
    }

    /// Insert blocks before the trailing Thinking row (or at the end).
    fn insert_blocks(&mut self, blocks: Vec<Block>) {
        let near = self.near_bottom();
        let pos = self.items.iter().rposition(|i| i.leave.is_none() && i.block == Block::Thinking).unwrap_or(self.items.len());
        for (j, b) in blocks.into_iter().enumerate() {
            let k = self.key();
            self.items.insert(pos + j, FeedItem { key: k, block: b, enter: Some((Tween::new(300.), j)), leave: None, h: Rc::default() });
        }
        if near {
            self.scroll_anim = Some((Tween::new(520.), -f32::from(self.scroll.offset().y)));
        }
    }

    fn remove_thinking(&mut self) {
        for i in self.items.iter_mut() {
            if i.block == Block::Thinking && i.leave.is_none() {
                i.leave = Some(Tween::new(260.));
            }
        }
    }
}

impl TakoApp {
    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.chat.input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.chat.input.update(cx, |s, cx| s.set_value("", window, cx));
        let c = &mut self.chat;
        // a stale Thinking (stopped run) folds away
        c.remove_thinking();
        let near = c.near_bottom();
        let k1 = c.key();
        let k2 = c.key();
        c.items.push(FeedItem { key: k1, block: Block::User { text, steer: false }, enter: Some((Tween::new(300.), 0)), leave: None, h: Rc::default() });
        c.items.push(FeedItem { key: k2, block: Block::Thinking, enter: Some((Tween::new(300.), 1)), leave: None, h: Rc::default() });
        if near {
            c.scroll_anim = Some((Tween::new(520.), -f32::from(c.scroll.offset().y)));
        }
        c.running = true;
        let id = self.active;
        if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
            t.running = true;
        }
        let steps = data::agent_steps();
        let n = steps.len();
        for (i, s) in steps.into_iter().enumerate() {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(700 * (i as u64 + 1))).await;
                this.update(cx, |this, cx| {
                    if this.active != id || !this.chat.running {
                        return;
                    }
                    this.chat.insert_blocks(vec![s]);
                    if i == n - 1 {
                        this.chat.remove_thinking();
                        this.chat.running = false;
                        if let Some(t) = this.threads.iter_mut().find(|t| t.id == id) {
                            t.running = false;
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.chat.remove_thinking();
        self.chat.running = false;
        self.chat.insert_blocks(vec![Block::Text("_Stopped._".into())]);
        let id = self.active;
        if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
            t.running = false;
        }
        cx.notify();
    }

    pub fn render_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.chat.tabs.iter().find(|t| t.id == self.chat.active_tab).cloned().unwrap_or(self.chat.tabs[0].clone());
        let body: AnyElement = if tab.kind == TabKind::Chat {
            div().flex_1().min_h_0().flex().flex_col().child(self.render_feed(window, cx)).child(self.render_composer(false, window, cx)).into_any_element()
        } else {
            self.render_tool_tab(&tab, window, cx).into_any_element()
        };
        div().size_full().flex().flex_col().child(self.render_tabbar(cx)).child(body)
    }

    fn render_tabbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let active = self.chat.active_tab;
        let tabs = self.chat.tabs.clone();
        div()
            .h(px(45.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .px(px(12.))
            .when(!self.sidebar_open, |d| d.pl(px(150.)))
            .children(tabs.into_iter().map(|tab| {
                let on = tab.id == active;
                let id = tab.id;
                let ic = match tab.kind {
                    TabKind::Chat => "spark",
                    TabKind::Browser => "globe",
                    TabKind::Editor => "code",
                    TabKind::Terminal => "term",
                    TabKind::Diff => "diff",
                    TabKind::Markdown => "doc",
                    TabKind::Whiteboard => "pen",
                    TabKind::Emulator => "phone",
                };
                div()
                    .id(("tab", id))
                    .h(px(30.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.5))
                    .text_color(if on { t.text } else { t.text_2 })
                    .when(on, |d| d.bg(t.surface_active))
                    .when(!on, |d| d.hover(|s| s.bg(t.surface_hover)))
                    .child(icon(ic, 14., if tab.kind == TabKind::Chat { t.accent } else { t.text_3 }))
                    .child(tab.name.clone())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.chat.active_tab = id;
                        cx.notify();
                    }))
            }))
            .child(
                div()
                    .id("tab-new")
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .text_color(t.text_3)
                    .hover(|s| s.bg(t.surface_hover))
                    .child(icon("plus", 14., t.text_3))
                    .child("⌘T"),
            )
            .child(div().flex_1())
            .child(self.render_tb_right(cx))
    }

    fn render_feed(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        // retire finished animations
        self.chat.items.retain(|i| !i.leave.as_ref().is_some_and(|l| l.done()));
        let mut animating = false;
        for i in self.chat.items.iter_mut() {
            if let Some((tw, j)) = &i.enter {
                if tw.start.elapsed().as_secs_f32() * 1000. > 300. + 40. * *j as f32 + 400. {
                    i.enter = None;
                } else {
                    animating = true;
                }
            }
            if i.leave.is_some() {
                animating = true;
            }
        }
        // eased scroll that keeps chasing the bottom while blocks grow
        if let Some((tw, from)) = &self.chat.scroll_anim {
            let p = tw.raw();
            let e = ease_out(p);
            let to = f32::from(self.chat.scroll.max_offset().y);
            let y = from + (to - from) * e;
            self.chat.scroll.set_offset(point(px(0.), px(-y)));
            if p >= 1. {
                self.chat.scroll_anim = None;
            } else {
                animating = true;
            }
        }
        if animating {
            window.request_animation_frame();
        }

        let open = self.chat.open_tools.clone();
        // CSS collapses adjacent vertical margins; GPUI doesn't, so do it here:
        // the gap above a block is max(prev.margin-bottom, this.margin-top).
        let margins = |b: &Block| -> (f32, f32) {
            match b {
                Block::User { .. } => (22., 28.),
                Block::Text(_) => (14., 14.),
                Block::Tool { .. } => (4., 4.),
                Block::Thinking => (0., 0.),
            }
        };
        let mut prev_mb = 0.;
        let n_items = self.chat.items.len();
        let gaps: Vec<(f32, f32)> = self
            .chat
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let (mt, mb) = margins(&it.block);
                let top = if i == 0 { mt } else { mt.max(prev_mb) };
                prev_mb = mb;
                (top, if i + 1 == n_items { mb } else { 0. })
            })
            .collect();
        let items: Vec<AnyElement> = self
            .chat
            .items
            .iter()
            .zip(gaps)
            .map(|(it, (gt, gb))| {
                let el = block_view(&it.block, it.key, open.contains(&it.key), t, cx).into_any_element();
                let natural = it.h.clone();
                let inner = div().relative().flex_none().pt(px(gt)).pb(px(gb)).child(el).child(measurer(natural.clone()));
                if let Some((tw, j)) = &it.enter {
                    let ms = tw.start.elapsed().as_secs_f32() * 1000.;
                    let hp = ease_snappy(((ms - 40. * *j as f32) / 300.).clamp(0., 1.));
                    let op = ease_snappy(((ms - 120. - 60. * *j as f32) / 320.).clamp(0., 1.));
                    let h = natural.get();
                    div()
                        .when(h > 0., |d| d.h(px(h * hp)))
                        .when(h == 0., |d| d.h(px(0.)))
                        .overflow_hidden()
                        .child(inner.opacity(op).top(px(6. * (1. - op))))
                        .into_any_element()
                } else if let Some(lv) = &it.leave {
                    let e = ease_snappy(lv.raw());
                    let h = natural.get();
                    div().h(px(h * (1. - e))).overflow_hidden().opacity(1. - e).child(inner).into_any_element()
                } else {
                    inner.into_any_element()
                }
            })
            .collect();

        div()
            .id("transcript")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.chat.scroll)
            .child(
                div()
                    .max_w(px(760.))
                    .mx_auto()
                    .px(px(32.))
                    .pt(px(28.))
                    .pb(px(40.))
                    .flex()
                    .flex_col()
                    .children(items),
            )
    }

    pub fn render_composer(&mut self, home: bool, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let running = !home && self.chat.running;
        let ready = !self.chat.input.read(cx).value().trim().is_empty();
        let th = self.thread().clone();
        let proj = self.projects.iter().find(|p| p.id == th.repo).cloned();
        let chip = |id: &'static str| div().id(id).flex().items_center().gap(px(6.)).px(px(7.)).py(px(4.)).rounded(px(7.)).text_color(t.text_2).hover(|s| s.bg(t.surface_hover));
        let sep = || div().flex().items_center().child(icon("chev-r", 12., t.text_3));
        let send_bg = if running || ready { t.text } else { t.surface_active };
        let send_fg = if running || ready { t.bg } else { t.text_3 };
        div().w_full().flex_none().pb(px(16.)).child(
            div().max_w(px(760.)).mx_auto().px(px(32.)).child(
                div()
                    .rounded(px(14.))
                    .bg(t.composer_foot)
                    .overflow_hidden()
                    .child(
                        div()
                            .rounded(px(14.))
                            .bg(t.composer_card)
                            .pt(px(14.))
                            .pr(px(12.))
                            .pb(px(12.))
                            .pl(px(16.))
                            .child(
                                div()
                                    .min_h(px(44.))
                                    .text_size(px(15.))
                                    .child(Textarea::new(&self.chat.input).appearance(false).bordered(false).text_size(px(15.)).p_0()),
                            )
                            .child(
                                div()
                                    .mt(px(10.))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().id("attach").size(px(34.)).rounded_full().bg(t.surface_active).flex().items_center().justify_center().child(icon("plus", 18., t.text)))
                                    .child(div().flex_1())
                                    .child(div().id("mic").size(px(34.)).rounded_full().flex().items_center().justify_center().hover(|s| s.bg(t.surface_hover)).child(icon("mic", 16., t.text_2)))
                                    .child(
                                        div()
                                            .id("send")
                                            .size(px(34.))
                                            .rounded_full()
                                            .bg(send_bg)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(running, |d| d.child(div().size(px(10.)).rounded(px(2.)).bg(send_fg)))
                                            .when(!running, |d| d.child(icon("up", 16., send_fg)))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                if running {
                                                    this.stop(cx)
                                                } else {
                                                    this.send(window, cx)
                                                }
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .px(px(10.))
                            .py(px(7.))
                            .text_size(px(13.))
                            .child(chip("c-access").child(icon("lock", 14., t.text_2)).child("Full access"))
                            .child(chip("c-machine").child(icon("laptop", 14., t.text_2)).child("My MacBook"))
                            .child(sep())
                            .child(chip("c-proj").when_some(proj.clone(), |d, p| d.child(pix(p.icon, 14.))).child(proj.map(|p| p.name).unwrap_or("tako")))
                            .child(sep())
                            .child(chip("c-branch").child(icon("branch", 14., t.text_2)).child(if home { "main".to_string() } else { th.branch.clone() }))
                            .child(div().flex_1().min_w(px(8.)))
                            .child(
                                chip("c-model")
                                    .child(icon("spark", 14., t.accent))
                                    .child(div().font_weight(FontWeight::SEMIBOLD).text_color(t.text).child("Opus 5.5"))
                                    .child(div().text_color(t.text_3).child("High")),
                            )
                            .child(ctx_ring(t, if home { 0.08 } else { (0.08 + self.chat.items.len() as f32 * 0.06).min(0.9) })),
                    ),
            ),
        )
    }
}

fn ctx_ring(t: Theme, frac: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let c = b.center();
            let r = px(5.5);
            let ring = |a0: f32, a1: f32, col: Hsla, window: &mut Window| {
                let mut pb = PathBuilder::stroke(px(2.));
                let n = 40;
                for i in 0..=n {
                    let a = a0 + (a1 - a0) * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                    let p = point(c.x + r * a.cos(), c.y + r * a.sin());
                    if i == 0 { pb.move_to(p) } else { pb.line_to(p) }
                }
                if let Ok(path) = pb.build() {
                    window.paint_path(path, col);
                }
            };
            ring(0., std::f32::consts::TAU, t.surface_active, window);
            ring(0., std::f32::consts::TAU * frac, t.text_2, window);
        },
    )
    .size(px(14.))
    .ml(px(4.))
    .mr(px(4.))
}

fn block_view(b: &Block, key: usize, open: bool, t: Theme, cx: &mut Context<TakoApp>) -> impl IntoElement {
    match b {
        Block::User { text, steer } => div()
            .flex()
            .flex_col()
            .items_end()
            .when(*steer, |d| d.child(div().flex().items_center().gap(px(6.)).mb(px(8.)).text_size(px(13.)).text_color(t.text_2).child(icon("steer", 14., t.text_2)).child("Steering conversation")))
            .child(
                div()
                    .max_w(relative(0.85))
                    .bg(t.surface_active)
                    .border_1()
                    .border_color(t.border)
                    .rounded(px(14.))
                    .px(px(18.))
                    .py(px(12.))
                    .line_height(relative(1.6))
                    .child(text.clone()),
            )
            .into_any_element(),
        Block::Text(s) => {
            let italic = s.starts_with('_') && s.ends_with('_');
            rich_par(s.trim_matches('_'), if italic { t.text_3 } else { t.text }, t, 14., 1.7).when(italic, |d| d.italic()).into_any_element()
        }
        Block::Tool { label, body, running, icon: ic } => {
            let body = body.clone();
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(("tool", key))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .px(px(8.))
                        .py(px(6.))
                        .mx(px(-8.))
                        .rounded(px(8.))
                        .text_color(t.text_2)
                        .hover(|s| s.bg(t.surface).text_color(t.text))
                        .when(*running, |d| d.child(spinner(("tool-spin", key), 13., t.text_3)))
                        .when(!*running, |d| d.child(icon(ic, 16., t.text_2)))
                        .child(div().flex_1().min_w_0().truncate().child(label.clone()))
                        .child(svg().path("icons/chev-r.svg").size(px(16.)).text_color(t.text_3).with_transformation(Transformation::rotate(gpui_kit::radians(if open { std::f32::consts::FRAC_PI_2 } else { 0. }))))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.chat.open_tools.remove(&key) {
                                this.chat.open_tools.insert(key);
                            }
                            cx.notify();
                        })),
                )
                .when(open, |d| {
                    d.child(
                        div()
                            .mt(px(2.))
                            .mb(px(10.))
                            .ml(px(26.))
                            .px(px(14.))
                            .py(px(12.))
                            .bg(if t.light { hsla(0., 0., 0., 0.04) } else { hsla(0., 0., 0., 0.28) })
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(10.))
                            .font_family(MONO)
                            .text_size(px(12.))
                            .line_height(relative(1.65))
                            .children(body.lines().map(|l| {
                                let c = if l.starts_with('$') { t.text } else if l.starts_with('✓') { t.green } else if l.starts_with('✗') { t.red } else { t.text_2 };
                                div().text_color(c).child(l.to_string())
                            })),
                    )
                })
                .into_any_element()
        }
        Block::Thinking => div()
            .flex()
            .items_center()
            .gap(px(10.))
            .py(px(6.))
            .text_color(t.text_3)
            .child(icon("clock", 16., t.text_3))
            .child(
                div().child("Thinking").with_animation(("think", key), Animation::new(Duration::from_millis(1600)).repeat(), move |d, p| {
                    // shimmer approximation: GPUI can't clip a gradient to text, so pulse the colour instead
                    let k = (1. - ((p * 2. - 1.).abs())).powf(1.5);
                    let c = Rgba::from(t.text_3);
                    let w = Rgba::from(t.text);
                    d.text_color(Rgba { r: c.r + (w.r - c.r) * k, g: c.g + (w.g - c.g) * k, b: c.b + (w.b - c.b) * k, a: 1. })
                }),
            )
            .into_any_element(),
    }
}
