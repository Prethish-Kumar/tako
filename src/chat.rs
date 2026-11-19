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
use std::{cell::Cell, collections::{HashMap, HashSet}, rc::Rc, time::Duration};

pub struct FeedItem {
    pub key: usize,
    pub block: Block,
    pub enter: Option<(Tween, usize)>, // tween + stagger index
    pub leave: Option<Tween>,
    pub h: Rc<Cell<f32>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabKind {
    Chat,
    NewTab,
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

/// A tab being dragged in the strip (also its drag preview).
#[derive(Clone)]
pub struct DraggedTab {
    pub id: usize,
    pub name: String,
    pub icon: &'static str,
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
}
impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(30.))
            .pl(px(10.))
            .pr(px(12.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .gap(px(7.))
            .bg(self.bg)
            .border_1()
            .border_color(self.border)
            .shadow_lg()
            .text_size(px(12.5))
            .text_color(self.fg)
            .font_family(FONT)
            .opacity(0.92)
            .child(icon(self.icon, 14., self.fg))
            .child(self.name.clone())
    }
}

/// One worktree's tabs.
#[derive(Clone)]
pub struct Session {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub last_chat: usize,
}

pub struct ChatState {
    pub items: Vec<FeedItem>,
    next_key: usize,
    pub open_tools: HashSet<usize>,
    pub scroll: ScrollHandle,
    scroll_anim: Option<(Tween, f32)>,
    pub input: Entity<TextareaState>,
    /// Home has its own composer text ("Ask anything, @ to mention…").
    pub home_input: Entity<TextareaState>,
    /// Tabs of the current worktree (mirrors `sessions[thread]`).
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub running: bool,
    thread_id: usize,
    /// Which chat tab the feed shows.
    chat_tab: usize,
    sessions: HashMap<usize, Session>,
    /// Transcripts of chat sessions not on screen, by (thread, tab).
    stored: HashMap<(usize, usize), Vec<Block>>,
    next_tab: usize,
    pub entering_tab: Option<(usize, Tween)>,
    pub tab_scroll: ScrollHandle,
    tab_scrolled_to: Option<(usize, usize)>,
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
            cx.notify();
        })
        .detach();
        let home_input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).placeholder("Ask anything, @ to mention…"));
        cx.subscribe_in(&home_input, window, |this: &mut TakoApp, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { secondary: false, shift: false } = ev {
                this.send(window, cx);
            }
            cx.notify();
        })
        .detach();
        ChatState {
            home_input,
            items: vec![],
            next_key: 0,
            open_tools: HashSet::new(),
            scroll: ScrollHandle::new(),
            scroll_anim: None,
            input,
            tabs: vec![],
            active_tab: 0,
            running: false,
            thread_id: 0,
            chat_tab: 0,
            sessions: HashMap::new(),
            stored: HashMap::new(),
            next_tab: 100,
            entering_tab: None,
            tab_scroll: ScrollHandle::new(),
            tab_scrolled_to: None,
            pending_send: false,
        }
    }

    fn session_for(&mut self, id: usize) -> Session {
        self.sessions
            .entry(id)
            .or_insert_with(|| {
                let mut tabs = vec![Tab { id: 0, kind: TabKind::Chat, name: "Chat".into() }];
                if id == 1 {
                    tabs.push(Tab { id: 1, kind: TabKind::Chat, name: "Review".into() });
                    tabs.push(Tab { id: 2, kind: TabKind::Diff, name: "Changes".into() });
                    tabs.push(Tab { id: 3, kind: TabKind::Browser, name: "localhost:3000".into() });
                    tabs.push(Tab { id: 4, kind: TabKind::Terminal, name: "Terminal".into() });
                }
                Session { tabs, active: 0, last_chat: 0 }
            })
            .clone()
    }

    fn save_session(&mut self) {
        if self.thread_id == 0 {
            return;
        }
        self.sessions.insert(self.thread_id, Session { tabs: self.tabs.clone(), active: self.active_tab, last_chat: self.chat_tab });
        let blocks: Vec<Block> = self.items.iter().filter(|i| i.leave.is_none()).map(|i| i.block.clone()).collect();
        self.stored.insert((self.thread_id, self.chat_tab), blocks);
    }

    /// Show worktree `id` (its tabs and its last chat session).
    pub fn load(&mut self, threads: &[Thread], id: usize) {
        self.save_session();
        let s = self.session_for(id);
        self.thread_id = id;
        self.tabs = s.tabs;
        self.active_tab = s.active;
        self.chat_tab = s.last_chat;
        let th = threads.iter().find(|t| t.id == id).unwrap_or(&threads[0]).clone();
        self.show_chat(&th, self.chat_tab);
    }

    fn show_chat(&mut self, th: &Thread, tab: usize) {
        self.chat_tab = tab;
        let blocks = self.stored.remove(&(th.id, tab)).unwrap_or_else(|| match tab {
            0 => data::transcript(th),
            1 if th.id == 1 => vec![
                Block::User { text: "Review the open diff in this worktree and flag anything risky.".into(), steer: false },
                Block::Text("Read all three files. The divider change is safe; the CSS collapse drops the hr opacity, which is intentional per the PR. One nit: `showDivider` could live in `groupItems` so tests don't need to render.".into()),
            ],
            _ => vec![],
        });
        self.items.clear();
        self.open_tools.clear();
        for b in blocks {
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

    /// Apply an agent step to a session that may not be on screen.
    fn step_offscreen(&mut self, sess: (usize, usize), block: Block, last: bool) {
        let v = self.stored.entry(sess).or_default();
        let pos = v.iter().rposition(|b| *b == Block::Thinking).unwrap_or(v.len());
        v.insert(pos, block);
        if last {
            v.retain(|b| *b != Block::Thinking);
        }
    }
}

impl TakoApp {
    /// The composer input used on Home (shared with chat).
    pub fn home_input(&self) -> Entity<TextareaState> {
        self.chat.home_input.clone()
    }

    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = if self.view == crate::app::View::Home { self.home_input() } else { self.chat.input.clone() };
        let text = input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        input.update(cx, |s, cx| s.set_value("", window, cx));
        if self.view == crate::app::View::Home {
            // A prompt from Home starts a new worktree and moves you to Chats.
            let nid = self.threads.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            let slug: String = text
                .to_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-")
                .chars()
                .take(24)
                .collect();
            let repo = "tako".to_string();
            self.threads.insert(
                0,
                Thread {
                    id: nid,
                    title: text.chars().take(40).collect(),
                    repo: repo.clone(),
                    pr: 2150 + nid as u32,
                    pr_state: data::PrState::Draft,
                    branch: format!("{repo}/{}", if slug.is_empty() { "task".into() } else { slug }),
                    running: false,
                    unread: false,
                    ago_min: 0,
                },
            );
            self.chat.stored.insert((nid, 0), vec![]);
            self.active = nid;
            self.view = crate::app::View::Chats;
            let threads = self.threads.clone();
            self.chat.load(&threads, nid);
        }
        let c = &mut self.chat;
        c.remove_thinking();
        let near = c.near_bottom() || c.items.is_empty();
        let k1 = c.key();
        let k2 = c.key();
        c.items.push(FeedItem { key: k1, block: Block::User { text, steer: false }, enter: Some((Tween::new(300.), 0)), leave: None, h: Rc::default() });
        c.items.push(FeedItem { key: k2, block: Block::Thinking, enter: Some((Tween::new(300.), 1)), leave: None, h: Rc::default() });
        if near {
            c.scroll_anim = Some((Tween::new(520.), -f32::from(c.scroll.offset().y)));
        }
        c.running = true;
        let sess = (c.thread_id, c.chat_tab);
        let id = self.active;
        if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
            t.running = true;
            t.ago_min = 0;
        }
        let steps = data::agent_steps();
        let n = steps.len();
        for (i, s) in steps.into_iter().enumerate() {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(700 * (i as u64 + 1))).await;
                this.update(cx, |this, cx| {
                    let last = i == n - 1;
                    let c = &mut this.chat;
                    let on_screen = (c.thread_id, c.chat_tab) == sess;
                    if on_screen {
                        if !c.items.iter().any(|it| it.leave.is_none() && it.block == Block::Thinking) {
                            return; // stopped
                        }
                        c.insert_blocks(vec![s]);
                        if last {
                            c.remove_thinking();
                            c.running = false;
                        }
                    } else {
                        if !c.stored.get(&sess).is_some_and(|v| v.contains(&Block::Thinking)) {
                            return;
                        }
                        c.step_offscreen(sess, s, last);
                    }
                    if last {
                        if let Some(t) = this.threads.iter_mut().find(|t| t.id == sess.0) {
                            t.running = false;
                            if !on_screen {
                                t.unread = true;
                            }
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

    pub fn activate_tab(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.chat.tabs.iter().find(|t| t.id == id).cloned() else { return };
        self.chat.active_tab = id;

        if tab.kind == TabKind::Chat && tab.id != self.chat.chat_tab {
            self.chat.save_session();
            let th = self.thread().clone();
            self.chat.show_chat(&th, id);
        }
        cx.notify();
    }

    /// Open a tab of `kind` after the current one (the ⌘T launcher turns into what you pick).
    pub fn open_tab(&mut self, kind: TabKind, name: &str, cx: &mut Context<Self>) {
        if self.view != crate::app::View::Chats {
            self.view = crate::app::View::Chats;
        }
        let c = &mut self.chat;
        let cur = c.tabs.iter().position(|t| t.id == c.active_tab).unwrap_or(0);
        if kind != TabKind::NewTab && c.tabs[cur].kind == TabKind::NewTab {
            c.tabs[cur].kind = kind;
            c.tabs[cur].name = name.into();
            let id = c.tabs[cur].id;
            if kind == TabKind::Chat {
                c.stored.insert((c.thread_id, id), vec![]);
            }
            self.activate_tab(id, cx);
            return;
        }
        c.next_tab += 1;
        let id = c.next_tab;
        if kind == TabKind::Chat {
            c.stored.insert((c.thread_id, id), vec![]);
        }
        c.tabs.insert(cur + 1, Tab { id, kind, name: name.into() });
        c.entering_tab = Some((id, Tween::new(320.)));
        self.activate_tab(id, cx);
    }

    /// Drag-reorder: drop `id` before/after `target` (the first Chat tab stays first).
    pub fn move_tab(&mut self, id: usize, target: usize, after: bool, cx: &mut Context<Self>) {
        let c = &mut self.chat;
        if id == target {
            return;
        }
        let Some(from) = c.tabs.iter().position(|t| t.id == id) else { return };
        if from == 0 {
            return;
        }
        let tab = c.tabs.remove(from);
        let mut to = c.tabs.iter().position(|t| t.id == target).unwrap_or(c.tabs.len());
        if after {
            to += 1;
        }
        c.tabs.insert(to.max(1), tab);
        cx.notify();
    }

    pub fn close_tab(&mut self, id: usize, cx: &mut Context<Self>) {
        let c = &mut self.chat;
        let Some(i) = c.tabs.iter().position(|t| t.id == id) else { return };
        if i == 0 {
            return;
        }
        c.tabs.remove(i);
        c.stored.remove(&(c.thread_id, id));
        if c.active_tab == id {
            let next = c.tabs.get(i).or(c.tabs.get(i - 1)).map(|t| t.id).unwrap_or(0);
            if c.chat_tab == id {
                c.chat_tab = 0;
                let th = self.thread().clone();
                self.chat.show_chat(&th, 0);
            }
            self.activate_tab(next, cx);
        }
        cx.notify();
    }

    pub fn tab_key(&mut self, key: &str, shift: bool, cx: &mut Context<Self>) {
        if key == "t" {
            self.open_tab(TabKind::NewTab, "New Tab", cx);
            return;
        }
        if self.view != crate::app::View::Chats {
            return;
        }
        let tabs: Vec<usize> = self.chat.tabs.iter().map(|t| t.id).collect();
        let i = tabs.iter().position(|t| *t == self.chat.active_tab).unwrap_or(0);
        match key {
            "w" => self.close_tab(self.chat.active_tab, cx),
            "]" if shift => self.activate_tab(tabs[(i + 1) % tabs.len()], cx),
            "[" if shift => self.activate_tab(tabs[(i + tabs.len() - 1) % tabs.len()], cx),
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && d != "0" => {
                let n: usize = d.parse().unwrap();
                let t = if n == 9 { *tabs.last().unwrap() } else if n <= tabs.len() { tabs[n - 1] } else { return };
                self.activate_tab(t, cx);
            }
            _ => {}
        }
    }

    pub fn render_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.chat.tabs.iter().find(|t| t.id == self.chat.active_tab).cloned().unwrap_or(self.chat.tabs[0].clone());
        let body: AnyElement = if tab.kind == TabKind::Chat {
            let empty = self.chat.items.is_empty();
            let t = cx.t();
            let branch = self.thread().branch.clone();
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .when(empty, |d| {
                    d.child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(6.))
                            .pt(relative(0.18))
                            .text_color(t.text_3)
                            .child(icon("spark", 22., t.accent).mb(px(6.)))
                            .child(div().text_color(t.text).font_weight(FontWeight::MEDIUM).text_size(px(15.)).child(format!("New session in {branch}")))
                            .child("Same worktree, same files — a fresh agent with its own context."),
                    )
                })
                .when(!empty, |d| d.child(self.render_feed(window, cx)))
                .child(self.render_composer(false, window, cx))
                .into_any_element()
        } else {
            div().flex_1().min_h_0().flex().bg(cx.t().bg).child(self.render_tool_tab(&tab, window, cx)).into_any_element()
        };
        div().size_full().flex().flex_col().child(self.render_tabbar(window, cx)).child(body)
    }

    fn render_tabbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let active = self.chat.active_tab;
        let tabs = self.chat.tabs.clone();
        let live = self.chat.running;
        let chat_tab = self.chat.chat_tab;
        let entering = self.chat.entering_tab.clone();
        // keep the active tab in view whenever it (or the tab count) changes
        let want = (active, tabs.len());
        if self.chat.tab_scrolled_to != Some(want) {
            if let Some(ix) = tabs.iter().position(|t| t.id == active) {
                self.chat.tab_scroll.scroll_to_item(ix);
            }
            self.chat.tab_scrolled_to = Some(want);
        }
        if let Some((_, tw)) = &entering {
            if tw.done() {
                self.chat.entering_tab = None;
            } else {
                window.request_animation_frame();
            }
        }
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .pt(px(8.))
            .pb(px(6.))
            .px(px(12.))
            .border_b_1()
            .border_color(t.border)
            .when(!self.sidebar_open, |d| d.pl(px(150.)))
            .child(
                // The tabs scroll sideways when they don't fit (trackpad swipe, or the mouse wheel).
                div()
                    .id("tab-strip")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .overflow_x_scroll()
                    .track_scroll(&self.chat.tab_scroll)
                    .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _, cx| {
                        let d = ev.delta.pixel_delta(px(20.));
                        if d.y.abs() > d.x.abs() {
                            let h = &this.chat.tab_scroll;
                            let max = h.max_offset();
                            let mut o = h.offset();
                            o.x = (o.x + d.y).clamp(-max.x, px(0.));
                            h.set_offset(o);
                            cx.notify();
                        }
                    }))
            .children(tabs.into_iter().enumerate().map(|(i, tab)| {
                let on = tab.id == active;
                let id = tab.id;
                let ic = match tab.kind {
                    TabKind::Chat => "spark",
                    TabKind::NewTab => "plus",
                    TabKind::Browser => "globe",
                    TabKind::Editor => "code",
                    TabKind::Terminal => "term",
                    TabKind::Diff => "diff",
                    TabKind::Markdown => "doc",
                    TabKind::Whiteboard => "pen",
                    TabKind::Emulator => "phone",
                };
                let is_live = live && tab.kind == TabKind::Chat && tab.id == chat_tab;
                let tab_icon: AnyElement = if is_live {
                    svg()
                        .path("icons/spark.svg")
                        .size(px(14.))
                        .text_color(t.accent)
                        .with_animation(("tab-live", id), Animation::new(Duration::from_millis(2400)).repeat(), |s, p| s.with_transformation(Transformation::rotate(percentage(p))))
                        .into_any_element()
                } else {
                    icon(ic, 14., if tab.kind == TabKind::Chat { t.accent } else { t.text_3 }).into_any_element()
                };
                let enter = entering.as_ref().filter(|(e, _)| *e == id).map(|(_, tw)| ease_snappy(tw.raw()));
                div()
                    .id(("tab", id))
                    .group("tab")
                    .h(px(30.))
                    .pl(px(10.))
                    .pr(px(if i == 0 { 12. } else { 5. }))
                    .flex_none()
                    .max_w(px(210.))
                    .min_w(px(44.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.5))
                    .text_color(if on { t.text } else { t.text_3 })
                    .when(on, |d| d.bg(t.surface_active))
                    .when(!on, |d| d.hover(|s| s.bg(t.surface_hover).text_color(t.text_2)))
                    .when_some(enter, |d, e| d.opacity(e).relative().top(px(4. * (1. - e))))
                    .child(tab_icon)
                    .child(div().min_w_0().truncate().child(tab.name.clone()))
                    .when(i > 0, |d| {
                        d.child(
                            div()
                                .id(("tab-x", id))
                                .size(px(18.))
                                .rounded(px(5.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(!on, |d| d.invisible().group_hover("tab", |s| s.visible()))
                                .hover(|s| s.bg(t.surface_active))
                                .child(icon("x", 11., t.text_3))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_tab(id, cx);
                                })),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.activate_tab(id, cx)))
                    .when(i > 0, |d| {
                        let dt = DraggedTab { id, name: tab.name.clone(), icon: ic, bg: t.panel_solid, fg: t.text, border: t.border };
                        d.on_drag(dt, |d, _, _, cx| cx.new(|_| d.clone()))
                    })
                    .drag_over::<DraggedTab>(move |s, _, _, _| s.bg(t.surface_hover))
                    .on_drop(cx.listener(move |this, d: &DraggedTab, window, cx| {
                        // after the target when dropped on its right half, like the mock
                        let after = window.mouse_position().x > px(0.) && this.chat.tabs.iter().position(|x| x.id == d.id).unwrap_or(0) < i;
                        this.move_tab(d.id, id, after, cx);
                    }))
            }))
            )
            .child(
                div()
                    .id("tab-add")
                    .size(px(28.))
                    .ml(px(2.))
                    .rounded(px(8.))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_hover))
                    .child(icon("plus", 14., t.text_3))
                    .on_click(cx.listener(|this, _, _, cx| this.open_tab(TabKind::NewTab, "New Tab", cx))),
            )
            .child(div().flex_none().ml(px(4.)).mr(px(4.)).text_size(px(11.)).text_color(t.text_3).opacity(0.6).child("⌘T"))
            .child(div().flex_none().pl(px(12.)).child(self.render_tb_right(cx)))
    }

    fn render_feed(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        // retire finished animations
        self.chat.items.retain(|i| !i.leave.as_ref().is_some_and(|l| l.done()));
        let mut animating = false;
        for i in self.chat.items.iter_mut() {
            if let Some((tw, j)) = &i.enter {
                if tw.ms() > 300. + 40. * *j as f32 + 400. {
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
                    let ms = tw.ms();
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

    pub fn render_composer(&mut self, home: bool, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let running = !home && self.chat.running;
        let input = if home { self.chat.home_input.clone() } else { self.chat.input.clone() };
        let ready = !input.read(cx).value().trim().is_empty();
        let send_bg = if running || ready { t.text } else { t.surface_active };
        let send_fg = if running || ready { t.bg } else { t.text_3 };
        div().w_full().flex_none().pb(px(16.)).child(
            div().max_w(px(760.)).mx_auto().px(px(32.)).child(
                div()
                    .rounded(px(14.))
                    .bg(t.composer_foot)
                    // Home's `rise` blur(8px) → 0, imitated by a halo that clears as the card lands
                    .when_some(if home { self.home_rise_halo(80.) } else { None }, |d, k| d.shadow(vec![BoxShadow { color: t.composer_card.opacity(0.85 * k), offset: point(px(0.), px(0.)), blur_radius: px(14. * k), spread_radius: px(3. * k), inset: false }]))
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
                                    .child(Textarea::new(&input).appearance(false).bordered(false).text_size(px(15.)).p_0()),
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
                    .child(self.render_ctx_chips(window, cx)),
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
                div().with_animation(("think", key), Animation::new(Duration::from_millis(1600)).repeat(), move |d, p| {
                    // The mock's shimmer: a text-3 → text → text-3 gradient (200% wide) slides across
                    // the word. GPUI can't clip a gradient to glyphs, so colour each glyph by where
                    // it sits under the moving band.
                    let word = "Thinking";
                    let n = word.chars().count() as f32;
                    let (c0, c1) = (Rgba::from(t.text_3), Rgba::from(t.text));
                    let centre = 1.5 - 2. * p;
                    let runs: Vec<TextRun> = word
                        .chars()
                        .enumerate()
                        .map(|(i, ch)| {
                            let x = (i as f32 + 0.5) / n;
                            let k = (1. - (x - centre).abs()).clamp(0., 1.);
                            let col = Rgba { r: c0.r + (c1.r - c0.r) * k, g: c0.g + (c1.g - c0.g) * k, b: c0.b + (c1.b - c0.b) * k, a: 1. };
                            TextRun { len: ch.len_utf8(), font: font(FONT), color: col.into(), background_color: None, underline: None, strikethrough: None }
                        })
                        .collect();
                    d.child(StyledText::new(word).with_runs(runs))
                }),
            )
            .into_any_element(),
    }
}
