//! Sidebar: new chat / new project, worktrees grouped by project (or label)
//! with pixel icons, project menu (rename, icon, labels), multi-select with
//! batch actions + undo, search, the New project dialog, and the smooth fold.

use crate::{
    app::{TakoApp, View},
    data::{self, PrState, Thread},
    pix::{icon_ids, pix},
    theme::{ActiveTheme, Theme},
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    component::input::{Input, InputEvent, InputState},
    *,
};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

pub struct Fold {
    pub opening: bool,
    pub tween: Tween,
    pub from: f32,
}

#[derive(Clone, Debug)]
pub struct LabelDef {
    pub id: String,
    pub name: String,
    pub color: u32,
}

pub const LABEL_COLORS: [u32; 8] = [0x7aa7e8, 0x5fc48a, 0xe2b75a, 0xc77dff, 0xec7aa0, 0x4fb3c8, 0xe5675f, 0x9aa3b0];

#[derive(Clone, Copy, PartialEq)]
pub enum NpSrc {
    Folder,
    Clone,
    Empty,
}

pub struct NewProject {
    pub src: NpSrc,
    pub icon: &'static str,
    pub label: Option<String>,
    pub picking: bool,
    pub path: Entity<InputState>,
    pub url: Entity<InputState>,
    pub name: Entity<InputState>,
    pub opened: Tween,
}

pub struct SidebarState {
    pub collapsed: HashSet<String>,
    pub folds: HashMap<String, Fold>,
    pub heights: HashMap<String, Rc<Cell<f32>>>,
    pub labels: Vec<LabelDef>,
    pub group_by_label: bool,
    pub menu: Option<String>,
    pub picker: Option<String>,
    pub renaming: Option<(String, Entity<InputState>)>,
    pub new_label: Option<(String, Entity<InputState>, usize)>,
    pub sel_mode: bool,
    pub sel: HashSet<usize>,
    pub last_sel: Option<usize>,
    pub confirm_del: bool,
    pub snack: Option<(String, bool, std::time::Instant)>,
    pub undo: Vec<Thread>,
    pub search_open: bool,
    pub search: Option<Entity<InputState>>,
    pub np: Option<NewProject>,
    pub flash: Option<(String, Tween)>,
}

impl Default for SidebarState {
    fn default() -> Self {
        SidebarState {
            collapsed: HashSet::new(),
            folds: HashMap::new(),
            heights: HashMap::new(),
            labels: vec![
                LabelDef { id: "work".into(), name: "Work".into(), color: 0x7aa7e8 },
                LabelDef { id: "personal".into(), name: "Personal".into(), color: 0x5fc48a },
                LabelDef { id: "oss".into(), name: "Open source".into(), color: 0xe2b75a },
                LabelDef { id: "client".into(), name: "Client".into(), color: 0xc77dff },
                LabelDef { id: "maint".into(), name: "Maintenance".into(), color: 0x9aa3b0 },
            ],
            group_by_label: false,
            menu: None,
            picker: None,
            renaming: None,
            new_label: None,
            sel_mode: false,
            sel: HashSet::new(),
            last_sel: None,
            confirm_del: false,
            snack: None,
            undo: vec![],
            search_open: false,
            search: None,
            np: None,
            flash: None,
        }
    }
}

fn slug(v: &str) -> String {
    let s: String = v.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

impl TakoApp {
    pub fn fold_project(&mut self, pid: &str, cx: &mut Context<Self>) {
        let s = &mut self.sidebar;
        let h = s.heights.entry(pid.to_string()).or_default().get();
        let shut = s.collapsed.contains(pid);
        let cur = s.folds.get(pid).map(|f| {
            let e = ease_snappy(f.tween.raw());
            if f.opening { f.from + (h - f.from) * e } else { f.from * (1. - e) }
        });
        if shut {
            s.collapsed.remove(pid);
            s.folds.insert(pid.into(), Fold { opening: true, tween: Tween::new(260. + (h / 5.).min(140.)), from: cur.unwrap_or(0.) });
        } else {
            s.folds.insert(pid.into(), Fold { opening: false, tween: Tween::new(240. + (h / 6.).min(120.)), from: cur.unwrap_or(h) });
        }
        cx.notify();
    }

    fn label_def(&self, id: Option<&str>) -> Option<LabelDef> {
        id.and_then(|l| self.sidebar.labels.iter().find(|x| x.id == l).cloned())
    }

    fn search_query(&self, cx: &App) -> String {
        self.sidebar.search.as_ref().map(|s| s.read(cx).value().to_lowercase()).unwrap_or_default()
    }

    fn visible_threads(&self, cx: &App) -> Vec<Thread> {
        let q = self.search_query(cx);
        let mut v: Vec<Thread> = self
            .threads
            .iter()
            .filter(|t| {
                q.is_empty() || {
                    let pname = self.projects.iter().find(|p| p.id == t.repo).map(|p| p.name).unwrap_or_default();
                    format!("{} {} {} {}", t.title, t.branch, t.wt(), pname).to_lowercase().contains(&q)
                }
            })
            .cloned()
            .collect();
        v.sort_by_key(|t| t.ago_min);
        v
    }

    /// Worktree ids in the order the sidebar shows them (search, grouping and folded
    /// projects respected) — what ⌘↑ / ⌘↓ step through.
    pub fn worktree_order(&self, cx: &App) -> Vec<usize> {
        let vis = self.visible_threads(cx);
        let mut projects: Vec<&data::Project> = self.projects.iter().collect();
        if self.sidebar.group_by_label {
            let rank = |p: &data::Project| p.label.and_then(|l| self.sidebar.labels.iter().position(|x| x.id == l)).unwrap_or(usize::MAX);
            projects.sort_by_key(|p| rank(p));
        }
        projects
            .into_iter()
            .filter(|p| !self.sidebar.collapsed.contains(p.id))
            .flat_map(|p| vis.iter().filter(|t| t.repo == p.id).map(|t| t.id).collect::<Vec<_>>())
            .collect()
    }

    /// ⌘↓ (`dir` = 1) / ⌘↑ (`dir` = -1): open the next / previous worktree.
    pub fn step_worktree(&mut self, dir: i32, cx: &mut Context<Self>) {
        let order = self.worktree_order(cx);
        if order.is_empty() {
            return;
        }
        let next = if self.view != View::Chats {
            if dir > 0 { order[0] } else { *order.last().unwrap() }
        } else {
            match order.iter().position(|id| *id == self.active) {
                Some(i) => order[(i as i32 + dir).clamp(0, order.len() as i32 - 1) as usize],
                None => order[0],
            }
        };
        self.open_thread(next, cx);
    }

    pub fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.search_open = !self.sidebar.search_open;
        if self.sidebar.search.is_none() {
            let s = cx.new(|cx| InputState::new(window, cx).placeholder("Filter worktrees…"));
            cx.subscribe(&s, |_, _, _: &InputEvent, cx| cx.notify()).detach();
            self.sidebar.search = Some(s);
        }
        if self.sidebar.search_open {
            if let Some(s) = &self.sidebar.search {
                s.update(cx, |s, cx| s.focus(window, cx));
            }
        }
        cx.notify();
    }

    pub fn set_sel_mode(&mut self, on: bool, cx: &mut Context<Self>) {
        self.sidebar.sel_mode = on;
        if !on {
            self.sidebar.sel.clear();
            self.sidebar.confirm_del = false;
        }
        cx.notify();
    }

    fn toggle_sel(&mut self, id: usize, range: bool, cx: &mut Context<Self>) {
        let s = &mut self.sidebar;
        if range && s.last_sel.is_some() {
            let ids: Vec<usize> = self.threads.iter().map(|t| t.id).collect();
            if let (Some(a), Some(b)) = (ids.iter().position(|x| Some(*x) == s.last_sel), ids.iter().position(|x| *x == id)) {
                for i in ids[a.min(b)..=a.max(b)].iter() {
                    s.sel.insert(*i);
                }
            }
        } else if !s.sel.remove(&id) {
            s.sel.insert(id);
        }
        s.last_sel = Some(id);
        s.confirm_del = false;
        s.sel_mode = true;
        cx.notify();
    }

    pub fn archive_threads(&mut self, ids: Vec<usize>, verb: &str, cx: &mut Context<Self>) {
        let gone: Vec<Thread> = self.threads.iter().filter(|t| ids.contains(&t.id)).cloned().collect();
        if gone.is_empty() {
            return;
        }
        self.threads.retain(|t| !ids.contains(&t.id));
        let n = gone.len();
        let active_gone = gone.iter().any(|t| t.id == self.active);
        self.sidebar.undo = gone;
        self.sidebar.sel.clear();
        self.sidebar.confirm_del = false;
        self.sidebar.sel_mode = false;
        if active_gone && self.view == View::Chats {
            if let Some(t) = self.threads.iter().min_by_key(|t| t.ago_min).map(|t| t.id) {
                self.open_thread(t, cx);
            } else {
                self.view = View::Home;
            }
        }
        self.show_snack(format!("{verb} {n} chat{}", if n > 1 { "s" } else { "" }), true, cx);
    }

    fn show_snack(&mut self, msg: String, undo: bool, cx: &mut Context<Self>) {
        let at = std::time::Instant::now();
        self.sidebar.snack = Some((msg, undo, at));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            this.update(cx, |this, cx| {
                if this.sidebar.snack.as_ref().is_some_and(|s| s.2 == at) {
                    this.sidebar.snack = None;
                    this.sidebar.undo.clear();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn start_rename(&mut self, pid: &str, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.projects.iter().find(|p| p.id == pid).map(|p| p.name.to_string()).unwrap_or_default();
        let inp = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let pid2 = pid.to_string();
        cx.subscribe(&inp, move |this, inp, ev: &InputEvent, cx| match ev {
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                if this.sidebar.renaming.is_none() {
                    return;
                }
                let v = inp.read(cx).value().trim().to_string();
                if !v.is_empty() {
                    if let Some(p) = this.projects.iter_mut().find(|p| p.id == pid2) {
                        p.name = leak(v);
                    }
                }
                this.sidebar.renaming = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        inp.update(cx, |s, cx| {
            s.focus(window, cx);
            s.select_all(window, cx);
        });
        self.sidebar.renaming = Some((pid.into(), inp));
        self.sidebar.menu = None;
        cx.notify();
    }

    pub fn open_new_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.new(|cx| InputState::new(window, cx).default_value("~/code/"));
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("github.com/org/repo"));
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("my-project"));
        for e in [&path, &url, &name] {
            cx.subscribe(e, |this, _, ev: &InputEvent, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    this.create_project(cx);
                }
                cx.notify();
            })
            .detach();
        }
        let ids: Vec<&'static str> = icon_ids().collect();
        let icon = ids[(self.projects.len() * 5) % ids.len()];
        path.update(cx, |s, cx| s.focus(window, cx));
        self.sidebar.np = Some(NewProject { src: NpSrc::Folder, icon, label: None, picking: false, path, url, name, opened: Tween::new(180.) });
        cx.notify();
    }

    fn np_guess(&self, cx: &App) -> String {
        let Some(n) = &self.sidebar.np else { return String::new() };
        match n.src {
            NpSrc::Clone => n.url.read(cx).value().rsplit('/').next().unwrap_or("").trim_end_matches(".git").to_string(),
            NpSrc::Folder => {
                let p = n.path.read(cx).value().to_string();
                let last = p.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
                if last == "~" { String::new() } else { last }
            }
            NpSrc::Empty => String::new(),
        }
    }

    fn create_project(&mut self, cx: &mut Context<Self>) {
        let Some(n) = &self.sidebar.np else { return };
        let typed = n.name.read(cx).value().to_string();
        let guess = self.np_guess(cx);
        let name = slug(if typed.trim().is_empty() { &guess } else { &typed });
        if name.is_empty() {
            return;
        }
        let mut id = name.clone();
        let mut i = 2;
        while self.projects.iter().any(|p| p.id == id) {
            id = format!("{name}-{i}");
            i += 1;
        }
        let icon = n.icon;
        let label = n.label.clone().map(leak);
        self.projects.insert(0, data::Project { id: leak(id.clone()), name: leak(name), icon, label });
        self.sidebar.np = None;
        self.sidebar.collapsed.remove(&id);
        self.sidebar.flash = Some((id, Tween::new(1600.)));
        cx.notify();
    }

    pub fn render_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let sb_row = |id: &'static str, ic: &'static str, label: &'static str| {
            div()
                .id(id)
                .mx(px(8.))
                .h(px(33.))
                .px(px(12.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .gap(px(10.))
                .hover(|s| s.bg(t.surface_hover))
                .child(icon(ic, 16., t.text))
                .child(label)
        };

        let done: Vec<String> = self.sidebar.folds.iter().filter(|(_, f)| f.tween.done()).map(|(k, _)| k.clone()).collect();
        for k in done {
            let f = self.sidebar.folds.remove(&k).unwrap();
            if !f.opening {
                self.sidebar.collapsed.insert(k);
            }
        }
        let mut animating = !self.sidebar.folds.is_empty();
        if let Some((_, tw)) = &self.sidebar.flash {
            if tw.done() {
                self.sidebar.flash = None;
            } else {
                animating = true;
            }
        }

        let vis = self.visible_threads(cx);
        let searching = !self.search_query(cx).is_empty();
        let order: Vec<data::Project> = self.projects.iter().filter(|p| !searching || vis.iter().any(|t| t.repo == p.id)).cloned().collect();
        let mut blocks: Vec<AnyElement> = vec![];
        if self.sidebar.group_by_label {
            let mut keys: Vec<Option<String>> = self.sidebar.labels.iter().map(|l| Some(l.id.clone())).collect();
            keys.push(None);
            for k in keys {
                let ps: Vec<data::Project> = order.iter().filter(|p| p.label.map(|s| s.to_string()) == k).cloned().collect();
                if ps.is_empty() {
                    continue;
                }
                let lab = self.label_def(k.as_deref());
                let mut sec = div().flex().flex_col().mt(px(6.)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .pt(px(14.))
                        .pb(px(2.))
                        .px(px(20.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text_3)
                        .when_some(lab.clone(), |d, l| d.child(div().size(px(7.)).rounded(px(2.)).bg(rgb(l.color))))
                        .child(tracked(&lab.map(|l| l.name.to_uppercase()).unwrap_or("NO LABEL".into()), 11.5, 0.06)),
                );
                for p in ps {
                    let list: Vec<Thread> = vis.iter().filter(|t| t.repo == p.id).cloned().collect();
                    sec = sec.child(self.project_block(&p, list, cx));
                }
                blocks.push(sec.into_any_element());
            }
        } else {
            for p in order {
                let list: Vec<Thread> = vis.iter().filter(|t| t.repo == p.id).cloned().collect();
                blocks.push(self.project_block(&p, list, cx).into_any_element());
            }
        }
        if animating {
            window.request_animation_frame();
        }

        let gl = self.sidebar.group_by_label;
        let sel_on = self.sidebar.sel_mode;
        div()
            .w(px(300.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(44.))
            .border_r_1()
            .border_color(t.border)
            .when_some(self.sidebar.search.clone().filter(|_| self.sidebar.search_open), |d, s| {
                d.child(div().mx(px(16.)).mt(px(4.)).mb(px(8.)).child(Input::new(&s).text_size(px(13.))))
            })
            .child(sb_row("sb-new", "plus", "New chat").on_click(cx.listener(|this, _, _, cx| this.go(View::Home, cx))))
            .child(sb_row("sb-proj", "folder", "New project").on_click(cx.listener(|this, _, window, cx| this.open_new_project(window, cx))))
            .child(
                div()
                    .mx(px(8.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id("sb-group")
                            .flex_1()
                            .mr(px(2.))
                            .h(px(27.))
                            .px(px(12.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .text_size(px(12.5))
                            .text_color(t.text_2)
                            .hover(|s| s.bg(t.surface_hover))
                            .child(icon(if gl { "list" } else { "folder" }, 14., t.text_3))
                            .child(if gl { "Grouped by label" } else { "Grouped by project" })
                            .child(div().flex_1())
                            .child(icon("sort", 14., t.text_3))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sidebar.group_by_label = !this.sidebar.group_by_label;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("sb-sel")
                            .size(px(28.))
                            .rounded(px(7.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(sel_on, |d| d.bg(t.accent.opacity(0.12)))
                            .hover(|s| s.bg(t.surface_hover))
                            .child(icon("checks", 14., if sel_on { t.accent } else { t.text_3 }))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sel_mode(!sel_on, cx))),
                    ),
            )
            .child(div().id("threads").flex_1().min_h_0().overflow_y_scroll().pt(px(8.)).pb(px(12.)).children(blocks))
            .children(self.render_batch(cx))
    }

    fn render_batch(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = cx.t();
        let s = &self.sidebar;
        let bar = || {
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .mx(px(8.))
                .mb(px(10.))
                .pl(px(12.))
                .pr(px(6.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(t.panel_solid)
                .border_1()
                .border_color(t.border)
                .shadow_md()
        };
        let ba = |id: &'static str, label: &'static str, danger: bool| {
            div()
                .id(id)
                .h(px(26.))
                .px(px(10.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if danger { white() } else { t.accent })
                .when(danger, |d| d.bg(t.red))
                .when(!danger, |d| d.hover(|s| s.bg(t.surface_hover)))
                .child(label)
        };
        if s.sel_mode {
            let n = s.sel.len();
            let list: Vec<&Thread> = self.threads.iter().filter(|t| s.sel.contains(&t.id)).collect();
            let any_run = list.iter().any(|t| t.running);
            let any_unread = list.iter().any(|t| t.unread);
            if s.confirm_del {
                return Some(
                    bar()
                        .child(div().flex_1().min_w_0().truncate().text_size(px(12.5)).text_color(t.text).font_weight(FontWeight::MEDIUM).child(format!("Delete {n} worktree{}? Branches and PRs are kept.", if n > 1 { "s" } else { "" })))
                        .child(ba("b-cancel", "Cancel", false).on_click(cx.listener(|this, _, _, cx| {
                            this.sidebar.confirm_del = false;
                            cx.notify();
                        })))
                        .child(ba("b-delok", "Delete", true).on_click(cx.listener(|this, _, _, cx| {
                            let ids: Vec<usize> = this.sidebar.sel.iter().copied().collect();
                            this.archive_threads(ids, "Deleted worktrees for", cx)
                        })))
                        .into_any_element(),
                );
            }
            let ib = |id: &'static str, ic: &'static str, enabled: bool| {
                div()
                    .id(id)
                    .size(px(28.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!enabled, |d| d.opacity(0.35))
                    .when(enabled, |d| d.hover(|s| s.bg(t.surface_hover)))
                    .child(icon(ic, 15., t.text_2))
            };
            return Some(
                bar()
                    .child(div().flex_1().min_w_0().truncate().text_size(px(12.5)).text_color(t.text).font_weight(FontWeight::MEDIUM).child(if n > 0 { format!("{n} selected") } else { "Select chats".into() }))
                    .child(ib("b-read", "inbox", any_unread).on_click(cx.listener(|this, _, _, cx| {
                        let sel = this.sidebar.sel.clone();
                        this.threads.iter_mut().filter(|t| sel.contains(&t.id)).for_each(|t| t.unread = false);
                        cx.notify();
                    })))
                    .child(ib("b-stop", "stop", any_run).on_click(cx.listener(|this, _, _, cx| {
                        let sel = this.sidebar.sel.clone();
                        this.threads.iter_mut().filter(|t| sel.contains(&t.id)).for_each(|t| t.running = false);
                        cx.notify();
                    })))
                    .child(ib("b-arch", "archive", n > 0).on_click(cx.listener(|this, _, _, cx| {
                        let ids: Vec<usize> = this.sidebar.sel.iter().copied().collect();
                        this.archive_threads(ids, "Archived", cx)
                    })))
                    .child(ib("b-del", "trash", n > 0).on_click(cx.listener(move |this, _, _, cx| {
                        if n > 0 {
                            this.sidebar.confirm_del = true;
                            cx.notify();
                        }
                    })))
                    .child(div().w(px(1.)).h(px(16.)).mx(px(3.)).bg(t.border))
                    .child(ib("b-done", "x", true).on_click(cx.listener(|this, _, _, cx| this.set_sel_mode(false, cx))))
                    .into_any_element(),
            );
        }
        let (msg, undo, _) = s.snack.clone()?;
        Some(
            bar()
                .child(div().flex_1().min_w_0().truncate().text_size(px(12.5)).text_color(t.text_2).child(msg))
                .when(undo, |d| {
                    d.child(ba("b-undo", "Undo", false).on_click(cx.listener(|this, _, _, cx| {
                        let back = std::mem::take(&mut this.sidebar.undo);
                        this.threads.extend(back);
                        this.sidebar.snack = None;
                        cx.notify();
                    })))
                })
                .into_any_element(),
        )
    }

    fn project_block(&mut self, p: &data::Project, list: Vec<Thread>, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let pid = p.id.to_string();
        let shut = self.sidebar.collapsed.contains(&pid);
        let fold = self.sidebar.folds.get(&pid);
        let natural = self.sidebar.heights.entry(pid.clone()).or_default().clone();
        let label = self.label_def(p.label);
        let rot = match fold {
            Some(f) => {
                let e = ease_snappy(f.tween.raw());
                if f.opening { -90. * (1. - e) } else { -90. * e }
            }
            None if shut => -90.,
            None => 0.,
        };
        let list_h: Option<f32> = fold.map(|f| {
            let e = ease_snappy(f.tween.raw());
            let h = natural.get();
            if f.opening { f.from + (h - f.from) * e } else { f.from * (1. - e) }
        });
        let show_list = !shut || fold.is_some();
        let active = self.active;
        let in_chats = self.view == View::Chats && !self.sidebar.sel_mode;
        let n = list.len();
        let rows_p = fold.map(|f| (f.opening, f.tween.clone()));
        let sel_mode = self.sidebar.sel_mode;
        let all_sel = n > 0 && list.iter().all(|t| self.sidebar.sel.contains(&t.id));
        let renaming = self.sidebar.renaming.as_ref().filter(|(r, _)| *r == pid).map(|(_, i)| i.clone());
        let flash = self.sidebar.flash.as_ref().filter(|(f, _)| *f == pid).map(|(_, tw)| tw.raw());
        let menu_open = self.sidebar.menu.as_deref() == Some(pid.as_str());
        let picker_open = self.sidebar.picker.as_deref() == Some(pid.as_str());
        let searching = !self.search_query(cx).is_empty();
        let (p1, p2, p3, p4) = (pid.clone(), pid.clone(), pid.clone(), pid.clone());
        let ids: Vec<usize> = list.iter().map(|t| t.id).collect();

        let header = div()
            .id(SharedString::from(format!("pjh-{pid}")))
            .group("pjh")
            .h(px(30.))
            .mx(px(8.))
            .pl(px(14.))
            .pr(px(8.))
            .rounded(px(7.))
            .flex()
            .items_center()
            .gap(px(8.))
            .hover(|s| s.bg(t.surface_hover))
            .when_some(flash, |d, f| {
                let a = if f < 0.4 { 0.18 } else { 0.18 * (1. - (f - 0.4) / 0.6) };
                d.bg(t.accent.opacity(a))
            })
            .child(
                div()
                    .id(SharedString::from(format!("pjic-{pid}")))
                    .size(px(22.))
                    .rounded(px(6.))
                    .bg(t.surface)
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_active))
                    .child(pix(p.icon, 14.))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sidebar.picker = if this.sidebar.picker.as_deref() == Some(p4.as_str()) { None } else { Some(p4.clone()) };
                        this.sidebar.menu = None;
                        cx.notify();
                    })),
            )
            .when_some(renaming.clone(), |d, inp| d.child(div().flex_1().min_w_0().child(Input::new(&inp).text_size(px(13.)))))
            .when(renaming.is_none(), |d| {
                d.child(
                    div()
                        .id(SharedString::from(format!("pjn-{pid}")))
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(p.name)
                        .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                            if ev.click_count() >= 2 {
                                this.start_rename(&p1, window, cx);
                            } else {
                                this.fold_project(&p1, cx);
                            }
                        })),
                )
            })
            .when_some(label.filter(|_| !self.sidebar.group_by_label && renaming.is_none()), |d, l| {
                let c: Hsla = rgb(l.color).into();
                d.child(div().flex_none().text_size(px(10.5)).px(px(6.)).py(px(1.)).rounded(px(4.)).text_color(c).bg(c.opacity(0.14)).child(l.name.clone()))
            })
            .child(div().flex_1())
            .when(sel_mode && n > 0, |d| {
                d.child(
                    div()
                        .id(SharedString::from(format!("pjall-{pid}")))
                        .text_size(px(11.))
                        .text_color(t.accent)
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(5.))
                        .hover(|s| s.bg(t.surface_hover))
                        .child(if all_sel { "None" } else { "All" })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            for i in &ids {
                                if all_sel {
                                    this.sidebar.sel.remove(i);
                                } else {
                                    this.sidebar.sel.insert(*i);
                                }
                            }
                            cx.notify();
                        })),
                )
            })
            .when(!(sel_mode && n > 0), |d| d.child(div().text_size(px(11.5)).text_color(t.text_3).child(n.to_string())))
            .child(
                div()
                    .id(SharedString::from(format!("pjm-{pid}")))
                    .size(px(22.))
                    .rounded(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!menu_open, |d| d.invisible().group_hover("pjh", |s| s.visible()))
                    .hover(|s| s.bg(t.surface_active))
                    .child(icon("dots", 13., t.text_3))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sidebar.menu = if this.sidebar.menu.as_deref() == Some(p2.as_str()) { None } else { Some(p2.clone()) };
                        this.sidebar.picker = None;
                        this.sidebar.new_label = None;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id(SharedString::from(format!("pjf-{pid}")))
                    .size(px(22.))
                    .rounded(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_active))
                    .child(svg().path("icons/chev-d.svg").size(px(13.)).text_color(t.text_3).with_transformation(Transformation::rotate(gpui_kit::radians(rot.to_radians()))))
                    .on_click(cx.listener(move |this, _, _, cx| this.fold_project(&p3, cx))),
            );

        let rows: Vec<AnyElement> = list
            .iter()
            .enumerate()
            .map(|(i, th)| {
                let el = self.wt_row(th, active == th.id && in_chats, cx).into_any_element();
                match &rows_p {
                    Some((opening, tw)) => {
                        let ms = tw.ms();
                        let (delay, dur) = if *opening { (30. + i as f32 * 22., 260.) } else { ((n - 1 - i) as f32 * 12., 160.) };
                        let p = ((ms - delay) / dur).clamp(0., 1.);
                        let (o, y) = if *opening {
                            let e = ease_snappy(p);
                            (e, -6. * (1. - e))
                        } else {
                            let e = p * p;
                            (1. - e, -4. * e)
                        };
                        div().opacity(o).relative().top(px(y)).child(el).into_any_element()
                    }
                    None => el,
                }
            })
            .collect();
        let none_text = if searching { "No worktrees match" } else { "No chats yet" };
        let menu = if menu_open { Some(self.project_menu(&pid, cx).into_any_element()) } else { None };
        let picker = if picker_open { Some(self.icon_picker(&pid, p.icon, cx).into_any_element()) } else { None };

        div()
            .relative()
            .flex()
            .flex_col()
            .mb(px(6.))
            .child(header)
            .children(menu)
            .children(picker)
            .when(show_list, |d| {
                d.child(
                    div().when_some(list_h, |d, h| d.h(px(h)).overflow_hidden()).child(
                        div()
                            .relative()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .mt(px(2.))
                            .when(rows.is_empty(), |d| d.child(div().px(px(22.)).pt(px(4.)).pb(px(6.)).text_size(px(12.)).text_color(t.text_3).child(none_text)))
                            .children(rows)
                            .child(measurer(natural.clone())),
                    ),
                )
            })
    }

    /// A popover box anchored 32px under the project header (like .pj-menu / .pj-pick).
    fn pop_box(t: Theme) -> Div {
        div().ml(px(20.)).mt(px(32.)).w(px(244.)).rounded(px(10.)).border_1().border_color(t.border).bg(t.panel_solid).shadow_lg()
    }

    fn icon_picker(&mut self, pid: &str, cur: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let cells: Vec<AnyElement> = icon_ids()
            .map(|id| {
                let pid = pid.to_string();
                let on = id == cur;
                div()
                    .id(SharedString::from(format!("pk-{id}")))
                    .size(px(27.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(on, |d| d.bg(t.surface_active).border_1().border_color(t.accent.opacity(0.6)))
                    .hover(|s| s.bg(t.surface_hover))
                    .child(pix(id, 18.))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(p) = this.projects.iter_mut().find(|p| p.id == pid) {
                            p.icon = id;
                        }
                        this.sidebar.picker = None;
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect();
        deferred(
            anchored().child(
                Self::pop_box(t)
                    .id("pj-pick")
                    .occlude()
                    .p(px(6.))
                    .flex()
                    .flex_wrap()
                    .gap(px(2.))
                    .children(cells)
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.sidebar.picker = None;
                        cx.notify();
                    })),
            ),
        )
        .with_priority(1)
    }

    fn save_new_label(&mut self, cx: &mut Context<Self>) {
        let Some((for_pid, inp, color)) = self.sidebar.new_label.clone() else { return };
        let name = inp.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let id = format!("l{}", self.sidebar.labels.len());
        self.sidebar.labels.push(LabelDef { id: id.clone(), name: name.chars().take(24).collect(), color: LABEL_COLORS[color] });
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == for_pid) {
            p.label = Some(leak(id));
        }
        self.sidebar.new_label = None;
        self.sidebar.menu = None;
        cx.notify();
    }

    fn project_menu(&mut self, pid: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let p = self.projects.iter().find(|x| x.id == pid).cloned().unwrap();
        let item = |id: SharedString| div().id(id).flex().items_center().gap(px(9.)).px(px(9.)).py(px(7.)).rounded(px(7.)).text_size(px(13.)).hover(|s| s.bg(t.surface_hover));
        let (a, b) = (pid.to_string(), pid.to_string());
        let mut m = Self::pop_box(t)
            .id("pj-menu")
            .occlude()
            .p(px(4.))
            .flex()
            .flex_col()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.sidebar.menu = None;
                this.sidebar.new_label = None;
                cx.notify();
            }))
            .child(item("pm-rn".into()).child(icon("edit", 14., t.text_2)).child("Rename").on_click(cx.listener(move |this, _, window, cx| this.start_rename(&a, window, cx))))
            .child(item("pm-ic".into()).child(pix(p.icon, 14.)).child("Change icon").on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar.picker = Some(b.clone());
                this.sidebar.menu = None;
                cx.notify();
            })))
            .child(div().text_size(px(11.)).text_color(t.text_3).px(px(9.)).pt(px(8.)).pb(px(3.)).border_t_1().border_color(t.border).mt(px(4.)).child("Label"));
        let labels = self.sidebar.labels.clone();
        for l in labels.into_iter().map(Some).chain(std::iter::once(None)) {
            let pid = pid.to_string();
            let on = p.label.map(|s| s.to_string()) == l.as_ref().map(|l| l.id.clone());
            let lid = l.as_ref().map(|l| l.id.clone());
            m = m.child(
                item(SharedString::from(format!("pm-l-{}", lid.clone().unwrap_or_default())))
                    .child(div().size(px(8.)).mx(px(3.)).rounded(px(2.)).bg(l.as_ref().map(|l| Hsla::from(rgb(l.color))).unwrap_or(t.text_3)))
                    .child(l.as_ref().map(|l| l.name.clone()).unwrap_or("No label".into()))
                    .when(on, |d| d.child(div().flex_1()).child(icon("check", 14., t.accent)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(p) = this.projects.iter_mut().find(|p| p.id == pid) {
                            p.label = lid.clone().map(leak);
                        }
                        this.sidebar.menu = None;
                        cx.notify();
                    })),
            );
        }
        if let Some((_, inp, color)) = self.sidebar.new_label.clone().filter(|(f, _, _)| f == pid) {
            m = m.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .px(px(9.))
                    .pt(px(8.))
                    .pb(px(6.))
                    .border_t_1()
                    .border_color(t.border)
                    .mt(px(4.))
                    .child(Input::new(&inp).text_size(px(13.)))
                    .child(div().flex().flex_wrap().gap(px(6.)).children(LABEL_COLORS.iter().enumerate().map(|(i, c)| {
                        div()
                            .id(("lcol", i))
                            .size(px(16.))
                            .rounded(px(4.))
                            .bg(rgb(*c))
                            .when(i == color, |d| d.border_2().border_color(t.text))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(nl) = this.sidebar.new_label.as_mut() {
                                    nl.2 = i;
                                }
                                cx.notify();
                            }))
                    })))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(6.))
                            .child(div().id("lb-cancel").px(px(10.)).py(px(5.)).rounded(px(7.)).text_size(px(12.5)).hover(|s| s.bg(t.surface_hover)).child("Cancel").on_click(cx.listener(|this, _, _, cx| {
                                this.sidebar.new_label = None;
                                cx.notify();
                            })))
                            .child(div().id("lb-save").px(px(10.)).py(px(5.)).rounded(px(7.)).text_size(px(12.5)).bg(t.accent).text_color(white()).child("Add label").on_click(cx.listener(|this, _, _, cx| this.save_new_label(cx)))),
                    ),
            );
        } else {
            let pid = pid.to_string();
            m = m.child(item("pm-new".into()).child(icon("plus", 14., t.text_2)).child("New label").on_click(cx.listener(move |this, _, window, cx| {
                let inp = cx.new(|cx| InputState::new(window, cx).placeholder("Label name"));
                cx.subscribe(&inp, |this, _, ev: &InputEvent, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.save_new_label(cx);
                    }
                    cx.notify();
                })
                .detach();
                inp.update(cx, |s, cx| s.focus(window, cx));
                let color = this.sidebar.labels.len() % LABEL_COLORS.len();
                this.sidebar.new_label = Some((pid.clone(), inp, color));
                cx.notify();
            })));
        }
        deferred(anchored().child(m)).with_priority(1)
    }

    fn wt_row(&mut self, th: &Thread, active: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = th.id;
        let sel_mode = self.sidebar.sel_mode;
        let selected = self.sidebar.sel.contains(&id);
        let pr_c = match th.pr_state {
            PrState::Open => t.green,
            PrState::Merged => rgb(0xb48ae8).into(),
            PrState::Closed => t.red,
            PrState::Draft => t.text_3,
        };
        div()
            .id(("wt", id))
            .relative()
            .mx(px(8.))
            .pl(px(if sel_mode { 38. } else { 14. }))
            .pr(px(10.))
            .py(px(7.))
            .rounded(px(8.))
            .flex()
            .flex_col()
            .gap(px(3.))
            .when(active, |d| d.bg(t.surface_active))
            .when(selected, |d| d.bg(t.accent.opacity(0.11)))
            .when(!active && !selected, |d| d.hover(|s| s.bg(t.surface_hover)))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                this.sidebar.menu = None;
                this.sidebar.picker = None;
                let m = ev.modifiers();
                if this.sidebar.sel_mode || m.platform || m.control || m.shift {
                    this.toggle_sel(id, m.shift, cx);
                } else {
                    this.open_thread(id, cx);
                }
            }))
            .when(sel_mode, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(13.))
                        .top(px(17.))
                        .size(px(16.))
                        .rounded(px(5.))
                        .border_1()
                        .border_color(if selected { t.accent } else { t.text_3 })
                        .when(selected, |d| d.bg(t.accent))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(selected, |d| d.child(icon("check", 11., white()))),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().min_w_0().text_size(px(13.)).truncate().child(th.title.clone()))
                    .when(!th.running && th.unread, |d| d.child(div().size(px(7.)).rounded_full().bg(t.accent)))
                    .when(th.running, |d| d.child(spinner(("wt-spin", id), 11., t.accent)))
                    .when(!th.running, |d| d.child(div().text_size(px(11.5)).text_color(t.text_3).child(if th.ago_min == 0 { "now".to_string() } else { data::ago(th.ago_min) }))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(11.5))
                    .text_color(t.text_3)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(icon("branch", 11., t.text_3))
                            .child(div().min_w_0().truncate().font_family(MONO).text_size(px(11.)).child(th.branch.clone())),
                    )
                    .child(div().flex().items_center().gap(px(3.)).text_color(pr_c).child(icon("pr", 11., pr_c)).child(format!("#{}", th.pr))),
            )
    }

    /// The New project dialog (scrim + card), rendered over everything.
    pub fn render_new_project(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = cx.t();
        let n = self.sidebar.np.as_ref()?;
        let fade = ease_out(n.opened.raw());
        if fade < 1. {
            window.request_animation_frame();
        }
        let guess = self.np_guess(cx);
        let name_v = n.name.read(cx).value().to_string();
        let nm = if name_v.trim().is_empty() { guess.clone() } else { name_v };
        let ok = !slug(&nm).is_empty();
        let src = n.src;
        let srcs = [
            (NpSrc::Folder, "folder", "Open a folder", "A repository already on this machine"),
            (NpSrc::Clone, "github", "Clone a repository", "From GitHub or any git URL"),
            (NpSrc::Empty, "plus", "Start from scratch", "A new, empty git repository"),
        ];
        let field = |label: &'static str| div().flex().flex_col().mb(px(12.)).child(div().text_size(px(12.)).text_color(t.text_2).mb(px(6.)).child(label));
        let inbox = || div().flex().items_center().gap(px(8.)).h(px(36.)).pl(px(10.)).pr(px(4.)).rounded(px(9.)).border_1().border_color(t.border).bg(t.surface);
        let btn = |id: &'static str, label: &'static str, primary: bool, enabled: bool| {
            div()
                .id(id)
                .h(px(28.))
                .px(px(11.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .border_1()
                .when(primary, |d| d.bg(t.accent).border_color(transparent_black()).text_color(white()))
                .when(!primary, |d| d.bg(t.surface).border_color(t.border).hover(|s| s.bg(t.surface_hover)))
                .when(!enabled, |d| d.opacity(0.45))
                .child(label)
        };
        let foot = match src {
            NpSrc::Clone => "Clones onto the machine picked in the composer.".to_string(),
            NpSrc::Empty => format!("Runs git init in ~/code/{}", if slug(&nm).is_empty() { "my-project".into() } else { slug(&nm) }),
            NpSrc::Folder => "Nothing is copied. Worktrees are created next to it.".to_string(),
        };
        let (path, url, name, icon_id, label, picking) = (n.path.clone(), n.url.clone(), n.name.clone(), n.icon, n.label.clone(), n.picking);
        let labels = self.sidebar.labels.clone();
        Some(
            div()
                .id("np-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(hsla(0., 0., 0., 0.32 * fade))
                .flex()
                .items_center()
                .justify_center()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.sidebar.np = None;
                    cx.notify();
                }))
                .child(
                    div()
                        .id("np")
                        .w(px(520.))
                        .bg(t.panel_solid)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(14.))
                        .shadow_2xl()
                        .pt(px(16.))
                        .px(px(18.))
                        .pb(px(14.))
                        .opacity(fade)
                        .relative()
                        .top(px(8. * (1. - fade)))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .mb(px(12.))
                                .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child("New project"))
                                .child(div().id("np-x").size(px(28.)).rounded(px(7.)).flex().items_center().justify_center().hover(|s| s.bg(t.surface_hover)).child(icon("x", 14., t.text_3)).on_click(cx.listener(|this, _, _, cx| {
                                    this.sidebar.np = None;
                                    cx.notify();
                                }))),
                        )
                        .child(div().flex().gap(px(8.)).mb(px(14.)).children(srcs.into_iter().map(|(k, ic, title, desc)| {
                            let on = k == src;
                            div()
                                .id(title)
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .px(px(12.))
                                .py(px(11.))
                                .rounded(px(10.))
                                .border_1()
                                .border_color(if on { t.accent } else { t.border })
                                .when(on, |d| d.bg(t.accent.opacity(0.08)))
                                .when(!on, |d| d.hover(|s| s.bg(t.surface_hover)))
                                .child(icon(ic, 16., if on { t.accent } else { t.text_2 }))
                                .child(div().child(div().text_size(px(12.5)).font_weight(FontWeight::SEMIBOLD).child(title)).child(div().mt(px(2.)).text_size(px(11.)).text_color(t.text_3).line_height(relative(1.35)).child(desc)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    if let Some(n) = this.sidebar.np.as_mut() {
                                        n.src = k;
                                        let f = match k {
                                            NpSrc::Clone => n.url.clone(),
                                            NpSrc::Folder => n.path.clone(),
                                            NpSrc::Empty => n.name.clone(),
                                        };
                                        f.update(cx, |s, cx| s.focus(window, cx));
                                    }
                                    cx.notify();
                                }))
                        })))
                        .when(src == NpSrc::Folder, |d| {
                            d.child(field("Folder").child(inbox().child(icon("folder", 15., t.text_3)).child(div().flex_1().child(Input::new(&path).appearance(false).font_family(MONO).text_size(px(13.)))).child(btn("np-browse", "Browse…", false, true).on_click(cx.listener(|this, _, window, cx| {
                                if let Some(n) = &this.sidebar.np {
                                    n.path.update(cx, |s, cx| s.set_value("~/code/side-quest", window, cx));
                                }
                                cx.notify();
                            })))))
                        })
                        .when(src == NpSrc::Clone, |d| d.child(field("Repository URL").child(inbox().child(icon("github", 15., t.text_3)).child(div().flex_1().child(Input::new(&url).appearance(false).font_family(MONO).text_size(px(13.)))))))
                        .child(
                            field("Name").child(
                                inbox()
                                    .child(div().id("np-ic").size(px(24.)).ml(px(-4.)).rounded(px(6.)).flex().items_center().justify_center().hover(|s| s.bg(t.surface_hover)).child(pix(icon_id, 16.)).on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(n) = this.sidebar.np.as_mut() {
                                            n.picking = !n.picking;
                                        }
                                        cx.notify();
                                    })))
                                    .child(div().flex_1().child(Input::new(&name).appearance(false).font_family(MONO).text_size(px(13.)))),
                            ),
                        )
                        .when(picking, |d| {
                            d.child(div().mt(px(-4.)).mb(px(12.)).p(px(6.)).rounded(px(10.)).border_1().border_color(t.border).flex().flex_wrap().gap(px(2.)).children(icon_ids().map(|id| {
                                div()
                                    .id(SharedString::from(format!("npk-{id}")))
                                    .size(px(30.))
                                    .rounded(px(6.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(id == icon_id, |d| d.bg(t.surface_active).border_1().border_color(t.accent.opacity(0.6)))
                                    .hover(|s| s.bg(t.surface_hover))
                                    .child(pix(id, 18.))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(n) = this.sidebar.np.as_mut() {
                                            n.icon = id;
                                            n.picking = false;
                                        }
                                        cx.notify();
                                    }))
                            })))
                        })
                        .child(field("Label").child(div().flex().flex_wrap().gap(px(6.)).children(std::iter::once(None).chain(labels.into_iter().map(Some)).map(|l| {
                            let on = label == l.as_ref().map(|l| l.id.clone());
                            let lid = l.as_ref().map(|l| l.id.clone());
                            div()
                                .id(SharedString::from(format!("npl-{}", lid.clone().unwrap_or("none".into()))))
                                .h(px(26.))
                                .px(px(10.))
                                .rounded_full()
                                .border_1()
                                .border_color(if on { t.text_3 } else { t.border })
                                .when(on, |d| d.bg(t.surface_active))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(if on { t.text } else { t.text_2 })
                                .when_some(l.clone(), |d, l| d.child(div().size(px(8.)).rounded(px(2.)).bg(rgb(l.color))))
                                .child(l.map(|l| l.name).unwrap_or("None".into()))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(n) = this.sidebar.np.as_mut() {
                                        n.label = lid.clone();
                                    }
                                    cx.notify();
                                }))
                        }))))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .pt(px(12.))
                                .border_t_1()
                                .border_color(t.border)
                                .child(div().flex_1().text_size(px(11.5)).text_color(t.text_3).child(foot))
                                .child(btn("np-cancel", "Cancel", false, true).on_click(cx.listener(|this, _, _, cx| {
                                    this.sidebar.np = None;
                                    cx.notify();
                                })))
                                .child(btn("np-ok", "Create project", true, ok).on_click(cx.listener(|this, _, _, cx| this.create_project(cx)))),
                        ),
                )
                .into_any_element(),
        )
    }
}
