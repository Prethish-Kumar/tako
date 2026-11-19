//! Root view: window chrome, rail, and the layout that hosts every page.

use gpui_kit::prelude::FluentBuilder as _;
use crate::{
    chat::ChatState,
    data::{self, Project, Thread},
    home::HomeState,
    overlays::OverlayState,
    pages::PagesState,
    right::RightState,
    scene::SceneState,
    sidebar::SidebarState,
    theme::{ActiveTheme, ThemeState},
    ui::*,
};
use gpui_kit::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Home,
    Chats,
    Automations,
    Plugins,
    Settings,
    Stats,
}

actions!(tako, [NextWorktree, PrevWorktree, Dismiss, OpenSettings, ToggleSidebar, ToggleRight, NewChat, FocusSearch, NewTab, CloseTab, NextTab, PrevTab]);

#[derive(Clone, PartialEq, gpui_kit::Action)]
#[action(namespace = tako, no_json)]
pub struct GoTab(pub usize);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("cmd-.", ToggleRight, None),
        KeyBinding::new("cmd-n", NewChat, None),
        KeyBinding::new("cmd-k", FocusSearch, None),
        KeyBinding::new("cmd-t", NewTab, None),
        KeyBinding::new("escape", Dismiss, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-shift-]", NextTab, None),
        KeyBinding::new("cmd-shift-[", PrevTab, None),
        KeyBinding::new("cmd-]", NextTab, None),
        KeyBinding::new("cmd-[", PrevTab, None),
        // Text inputs bind ⌘] / ⌘[ to indent/outdent; switching tabs wins, even while typing.
        KeyBinding::new("cmd-]", NextTab, Some("Input")),
        // ⌘↓ / ⌘↑ step through the worktrees in the sidebar (also while typing; inputs
        // would otherwise use them for start / end of text).
        KeyBinding::new("cmd-down", NextWorktree, None),
        KeyBinding::new("cmd-up", PrevWorktree, None),
        KeyBinding::new("cmd-down", NextWorktree, Some("Input")),
        KeyBinding::new("cmd-up", PrevWorktree, Some("Input")),
        KeyBinding::new("cmd-[", PrevTab, Some("Input")),
    ]);
    for i in 1..=9 {
        cx.bind_keys([KeyBinding::new(Box::leak(format!("cmd-{i}").into_boxed_str()), GoTab(i), None)]);
    }
    // last, so the terminal's Escape outranks the global Dismiss binding
    crate::tools::pty::bind_keys(cx);
}

/// Page-switch motion (the mock's VIEW TRANSITIONS): ~300ms, offsets + opacity only.
/// Home → Chats: the composer glides from Home's centre to the chat's bottom, the side
/// panels slide in and the feed fades up. Back to Home: the composer glides back while the
/// backdrop and stats fade in. Other pages fade up.
pub struct NavFx {
    last: View,
    pub from: View,
    pub at: Option<std::time::Instant>,
    /// Where the composer was laid out on Home / in the chat (unshifted), refreshed every frame.
    pub slot_home: std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    pub slot_chat: std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    /// The composer slot the transition started from.
    pub glide: Option<Bounds<Pixels>>,
}
impl NavFx {
    fn new(v: View) -> Self {
        NavFx { last: v, from: v, at: None, slot_home: Default::default(), slot_chat: Default::default(), glide: None }
    }
    /// Eased 0 → 1 progress of the running switch for one element; 1 when nothing runs.
    pub fn p(&self, delay_ms: f32, dur_ms: f32) -> f32 {
        match self.at {
            None => 1.,
            Some(at) => ease_snappy(((at.elapsed().as_secs_f32() * 1000. - delay_ms) / dur_ms).clamp(0., 1.)),
        }
    }
    /// A switch from `from` into the current view is running.
    pub fn entering_from(&self, from: View) -> bool {
        self.at.is_some() && self.from == from
    }
    /// (dx, dy, visible) to add to the composer of the view being entered so it starts where
    /// the old one was. `home` says which composer is asking.
    pub fn composer_offset(&self, home: bool) -> (f32, f32, bool) {
        let (Some(_), Some(g)) = (self.at, self.glide) else { return (0., 0., true) };
        if home == (self.last != View::Home) {
            return (0., 0., true);
        }
        let Some(cur) = (if home { &self.slot_home } else { &self.slot_chat }).get() else { return (0., 0., false) };
        let k = 1. - self.p(0., 340.);
        let cx = |b: Bounds<Pixels>| f32::from(b.origin.x) + f32::from(b.size.width) / 2.;
        ((cx(g) - cx(cur)) * k, f32::from(g.origin.y - cur.origin.y) * k, true)
    }
}

pub struct TakoApp {
    pub view: View,
    pub nav: NavFx,
    pub sidebar_open: bool,
    pub right_open: bool,
    pub threads: Vec<Thread>,
    pub projects: Vec<Project>,
    pub active: usize,
    pub focus: FocusHandle,
    // feature state, one per module
    pub sidebar: SidebarState,
    pub chat: ChatState,
    pub home: HomeState,
    pub right: RightState,
    pub pages: PagesState,
    pub overlays: OverlayState,
    pub scene: SceneState,
    pub ctx: crate::ctx::CtxState,
    animating_theme: bool,
    labels_seen: Vec<(String, String, u32)>,
    pub persist_last: Option<crate::persist::Saved>,
    pub persist_at: Option<std::time::Instant>,
    pub persist_json: String,
    fps: Entity<crate::fps::Fps>,
}

impl TakoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let threads = data::threads();
        let mut s = TakoApp {
            view: View::Home,
            nav: NavFx::new(View::Home),
            sidebar_open: true,
            right_open: true,
            projects: data::projects(),
            active: 1,
            focus: cx.focus_handle(),
            sidebar: SidebarState::default(),
            chat: ChatState::new(window, cx),
            home: HomeState::new(window, cx),
            right: RightState::new(window, cx),
            pages: PagesState::new(window, cx),
            overlays: OverlayState::new(window, cx),
            scene: SceneState::new(),
            ctx: crate::ctx::CtxState::new(),
            threads,
            animating_theme: false,
            labels_seen: vec![],
            persist_last: None,
            persist_at: None,
            persist_json: String::new(),
            fps: cx.new(|_| crate::fps::Fps::new()),
        };
        s.chat.load(&s.threads, s.active);
        s.restore_state(cx);
        // Shortcuts dispatch along the focused element's path; start with the app root focused.
        window.focus(&s.focus, cx);
        if s.pages.settings.startup == "chats" {
            s.view = View::Chats; // Settings → General → Open on startup
        }
        if std::env::var("TAKO_SNAP").is_err() {
            let marker = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/tako/onboarded");
            if !marker.exists() || std::env::var("TAKO_ONBOARDING").is_ok() {
                if let Some(dir) = marker.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&marker, "1");
                s.open_onboarding(window, cx);
            }
        }
        crate::snap::setup(&mut s, window, cx);
        s
    }

    /// Scripted actions for snapshots / tests.
    pub fn run_action(&mut self, a: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (k, v) = a.split_once(':').unwrap_or((a, ""));
        match k {
            "send" => {
                let inp = if self.view == View::Home { self.home_input() } else { self.chat.input.clone() };
                inp.update(cx, |s, cx| s.set_value(v.to_string(), window, cx));
                self.send(window, cx);
            }
            "fold" => self.fold_project(v, cx),
            "open" => self.open_thread(v.parse().unwrap_or(1), cx),
            "theme" => self.overlays.theme.open = true,
            "profile" => self.overlays.profile.open = true,
            "tab" => {
                let i: usize = v.parse().unwrap_or(0);
                if let Some(id) = self.chat.tabs.get(i).map(|t| t.id) {
                    self.activate_tab(id, cx)
                }
            }
            "newtab" => self.open_tab(crate::chat::TabKind::NewTab, "New Tab", cx),
            "nosidebar" => self.sidebar_open = false,
            "dump" => {
                let tab = self.chat.tabs.iter().find(|t| t.id == self.chat.active_tab).map(|t| t.name.clone()).unwrap_or_default();
                eprintln!(
                    "STATE {v}: view={:?} thread={} tab={tab} collapsed={:?} theme_open={} profile_open={} ctx_open={:?} scene={} pal={} sidebar={} right={} np={}",
                    self.view,
                    self.active,
                    self.sidebar.collapsed,
                    self.overlays.theme.open,
                    self.overlays.profile.open,
                    self.ctx.open,
                    self.scene.id,
                    cx.global::<ThemeState>().pal,
                    self.sidebar_open,
                    self.right_open,
                    self.sidebar.np.is_some()
                );
            }
            "search" => {
                self.toggle_search(window, cx);
                if !v.is_empty() {
                    if let Some(s) = self.sidebar.search.clone() {
                        s.update(cx, |s, cx| s.set_value(v.to_string(), window, cx));
                    }
                }
            }
            "group" => self.sidebar.group_by_label = v == "label",
            "ctx" => {
                let k: &'static str = match v { "access" => "access", "env" => "env", "project" => "project", "branch" => "branch", _ => "model" };
                self.open_ctx(k, window, cx);
            }
            "toast" => self.toast(v.to_string(), cx),
            "select" => {
                self.set_sel_mode(true, cx);
                for id in v.split(',').filter_map(|s| s.parse().ok()) {
                    self.sidebar.sel.insert(id);
                }
            }
            "pjmenu" => self.sidebar.menu = Some(v.to_string()),
            "pjpick" => self.sidebar.picker = Some(v.to_string()),
            "rename" => self.start_rename(v, window, cx),
            "newproject" => self.open_new_project(window, cx),
            "archive" => {
                let ids: Vec<usize> = v.split(',').filter_map(|s| s.parse().ok()).collect();
                self.archive_threads(ids, "Archived", cx);
            }
            "noright" => self.right_open = false,
            _ => self.run_ext_action(k, v, window, cx),
        }
        cx.notify();
    }

    pub fn thread(&self) -> &Thread {
        self.threads.iter().find(|t| t.id == self.active).unwrap_or(&self.threads[0])
    }

    pub fn open_thread(&mut self, id: usize, cx: &mut Context<Self>) {
        self.active = id;
        self.view = View::Chats;
        if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
            t.unread = false;
        }
        let threads = self.threads.clone();
        self.chat.load(&threads, id);
        cx.notify();
    }

    pub fn go(&mut self, v: View, cx: &mut Context<Self>) {
        self.view = v;
        cx.notify();
    }

    /// The mock has one label list for the whole app; the sidebar, Settings and
    /// Automations each keep a copy, so propagate whichever one changed.
    fn sync_labels(&mut self) {
        let side: Vec<(String, String, u32)> = self.sidebar.labels.iter().map(|l| (l.id.clone(), l.name.clone(), l.color)).collect();
        let set: Vec<(String, String, u32)> = self.pages.settings.labels.iter().map(|l| (l.id.clone(), l.name.clone(), l.c)).collect();
        let aut: Vec<(String, String, u32)> = self.pages.auto.labels.iter().map(|l| (l.id.clone(), l.name.clone(), l.c)).collect();
        let pick = if self.labels_seen.is_empty() {
            side.clone() // start from the sidebar's (the mock's) defaults
        } else if side != self.labels_seen {
            side.clone()
        } else if set != self.labels_seen {
            set.clone()
        } else if aut != self.labels_seen {
            aut.clone()
        } else {
            return;
        };
        self.sidebar.labels = pick.iter().map(|(id, name, c)| crate::sidebar::LabelDef { id: id.clone(), name: name.clone(), color: *c }).collect();
        self.pages.settings.labels = pick.iter().map(|(id, name, c)| crate::pages::settings::Lbl { id: id.clone(), name: name.clone(), c: *c }).collect();
        self.pages.auto.labels = pick.iter().map(|(id, name, c)| crate::pages::automations::Lbl { id: id.clone(), name: name.clone(), c: *c }).collect();
        self.labels_seen = pick;
    }

    /// Theme colour transitions + anything else that animates app-wide.
    fn tick_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let anim = cx.global_mut::<ThemeState>().tick();
        if anim {
            self.animating_theme = true;
            window.request_animation_frame();
        } else if self.animating_theme {
            self.animating_theme = false;
        }
    }

    fn render_titlebar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let full = self.is_empty_layout() || !self.sidebar_open;
        let tb_btn = |id: &'static str, ic: &'static str| {
            div()
                .id(id)
                .p(px(6.))
                .rounded(px(6.))
                .text_color(t.text_3)
                .hover(|s| s.bg(t.surface_hover).text_color(t.text))
                .child(icon(ic, 16., t.text_3))
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .h(px(44.))
            .when(full, |d| d.right_0())
            .when(!full, |d| d.w(px(52. + 300.)))
            .flex()
            .items_center()
            .gap(px(12.))
            .pl(px(14. + 64.))
            .pr(px(14.))
            .child(tb_btn("tb-sidebar", "sidebar").on_click(cx.listener(|this, _, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            })))
            .child(tb_btn("tb-left", "left"))
            .child(tb_btn("tb-right", "right"))
            .when(self.is_empty_layout(), |d| d.child(div().flex_1()).child(self.render_tb_right(cx)))
    }

    pub fn render_tb_right(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let b = |id: &'static str, ic: &'static str| {
            div().id(id).p(px(6.)).rounded(px(6.)).hover(|s| s.bg(t.surface_hover)).child(icon(ic, 16., t.text_3))
        };
        div()
            .flex()
            .items_center()
            .gap(px(2.))
            .child(b("tb-panel", "panel-r").on_click(cx.listener(|this, _, _, cx| {
                this.right_open = !this.right_open;
                cx.notify();
            })))
    }

    fn render_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let view = self.view;
        let btn = |id: &'static str, ic: &'static str, active: bool| {
            div()
                .id(id)
                .size(px(36.))
                .rounded(px(9.))
                .flex()
                .items_center()
                .justify_center()
                .when(active, |d| d.bg(t.surface_active))
                .when(!active, |d| d.hover(|s| s.bg(t.surface_hover)))
                .child(icon(ic, 19., if active { t.text } else { t.text_3 }))
        };
        div()
            .w(px(52.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .pt(px(54.))
            .pb(px(10.))
            .child(btn("rail-home", "home", view == View::Home).on_click(cx.listener(|this, _, _, cx| this.go(View::Home, cx))))
            .child(btn("rail-chats", "inbox", view == View::Chats).on_click(cx.listener(|this, _, _, cx| this.go(View::Chats, cx))))
            .child(btn("rail-auto", "bolt", view == View::Automations).on_click(cx.listener(|this, _, _, cx| this.go(View::Automations, cx))))
            .child(btn("rail-plugins", "plug", view == View::Plugins).on_click(cx.listener(|this, _, _, cx| this.go(View::Plugins, cx))))
            .child(div().flex_1())
            .child(
                btn("rail-theme", "palette", self.overlays.theme.open)
                    .relative()
                    // record the button's bounds so the theme popover anchors beside it
                    .child({
                        let anchor = self.overlays.theme.anchor.clone();
                        canvas(move |bounds, _, _| anchor.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overlays.theme.open = !this.overlays.theme.open;
                        cx.notify();
                    })),
            )
            .child(btn("rail-settings", "gear", view == View::Settings).on_click(cx.listener(|this, _, _, cx| this.go(View::Settings, cx))))
            .child(self.render_usage_ring(cx))
            .child(
                div()
                    .id("avatar")
                    .size(px(30.))
                    .rounded_full()
                    .bg(t.accent)
                    // the mock's .on ring while the Stats page is open
                    .when(view == View::Stats, |d| {
                        d.shadow(vec![
                            BoxShadow { color: t.bg, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false },
                            BoxShadow { color: t.accent.opacity(0.7), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(4.), inset: false },
                        ])
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(white())
                    .child("P")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overlays.profile.open = !this.overlays.profile.open;
                        cx.notify();
                    })),
            )
    }

    /// The chat pane's backdrop bleed: the scene's pre-blurred still, under the theme bg
    /// at (1 - bleed) opacity. Not live — GPUI has no backdrop-filter (see scene/blur.rs).
    fn render_bleed(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = cx.t();
        let bleed = cx.global::<ThemeState>().bleed;
        if bleed <= 0.001 {
            return None;
        }
        let im = crate::scene::blur::blurred(self.scene.id, t.light);
        Some(
            div()
                .absolute()
                .inset_0()
                .child(img(ImageSource::Render(im)).absolute().inset_0().size_full().object_fit(ObjectFit::Cover))
                .child(div().absolute().inset_0().bg(t.bg.opacity(1. - bleed)))
                .into_any_element(),
        )
    }

    pub fn is_empty_layout(&self) -> bool {
        self.view != View::Chats
    }
}

impl Render for TakoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick_theme(window, cx);
        self.sync_labels();
        self.persist_tick(cx);
        let t = cx.t();
        let view = self.view;
        let empty = self.is_empty_layout();
        // While the onboarding covers the window the app behind isn't drawn (the mock
        // hides .app), so the shared backdrop entity is only rendered once.
        let covered = self.onboarding_covers();
        if view != self.nav.last {
            let from = self.nav.last;
            self.nav.last = view;
            if !covered && !reduced_motion() {
                self.nav.from = from;
                self.nav.at = Some(std::time::Instant::now());
                self.nav.glide = match from {
                    View::Home => self.nav.slot_home.get(),
                    View::Chats => self.nav.slot_chat.get(),
                    _ => None,
                };
                self.home.fast = view == View::Home;
            }
        }
        if let Some(at) = self.nav.at {
            if at.elapsed().as_millis() > 650 {
                self.nav.at = None;
            } else {
                window.request_animation_frame();
            }
        }
        // panels slide in from their side when the chat layout appears
        let into_chats = view == View::Chats && self.nav.at.is_some() && self.nav.from != View::Chats;
        let (pl, pr) = if into_chats { (self.nav.p(0., 280.), self.nav.p(40., 280.)) } else { (1., 1.) };
        let page_p = if matches!(view, View::Automations | View::Plugins | View::Settings | View::Stats) { self.nav.p(0., 260.) } else { 1. };
        let show_sidebar = !covered && !empty && self.sidebar_open;
        let show_right = !covered && !empty && self.right_open;

        let main: AnyElement = if covered { div().into_any_element() } else { match view {
            View::Home => self.render_home(window, cx).into_any_element(),
            View::Chats => self.render_chat(window, cx).into_any_element(),
            View::Automations => self.render_automations(window, cx).into_any_element(),
            View::Plugins => self.render_plugins(window, cx).into_any_element(),
            View::Settings => self.render_settings(window, cx).into_any_element(),
            View::Stats => self.render_stats(window, cx).into_any_element(),
        } };

        div()
            .id("tako")
            .track_focus(&self.focus)
            .key_context("Tako")
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleRight, _, cx| {
                this.right_open = !this.right_open;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &NewChat, _, cx| this.go(View::Home, cx)))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| this.toggle_search(window, cx)))
            .on_action(cx.listener(|this, _: &NewTab, _, cx| this.tab_key("t", false, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.go(View::Settings, cx)))
            .on_action(cx.listener(|this, _: &NextWorktree, _, cx| this.step_worktree(1, cx)))
            .on_action(cx.listener(|this, _: &PrevWorktree, _, cx| this.step_worktree(-1, cx)))
            // Escape closes whatever menu / popover / dialog is open (the mock's Escape handlers).
            .on_action(cx.listener(|this, _: &Dismiss, _, cx| {
                this.ctx.open = None;
                this.overlays.theme.open = false;
                this.overlays.profile.open = false;
                this.sidebar.menu = None;
                this.sidebar.picker = None;
                this.sidebar.new_label = None;
                this.sidebar.np = None;
                if this.sidebar.sel_mode {
                    this.set_sel_mode(false, cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CloseTab, _, cx| this.tab_key("w", false, cx)))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| this.tab_key("]", true, cx)))
            .on_action(cx.listener(|this, _: &PrevTab, _, cx| this.tab_key("[", true, cx)))
            .on_action(cx.listener(|this, a: &GoTab, _, cx| this.tab_key(&a.0.to_string(), false, cx)))
            .size_full()
            .relative()
            .bg(t.bg)
            .text_color(t.text)
            .font_family(FONT)
            .text_size(px(14.))
            // Browsers use line-height: normal (~1.2 for Inter); GPUI defaults to 1.618.
            .line_height(relative(1.21))
            .flex()
            .when(!covered, |d| d.child(self.render_rail(cx)))
            .when(show_sidebar, |d| d.child(div().h_full().flex_none().relative().left(px(-14. * (1. - pl))).opacity(pl).child(self.render_sidebar(window, cx))))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .relative()
                    .when(view == View::Home, |d| d.pt(px(44.)).pr(px(12.)).pb(px(12.)))
                    .when(view == View::Chats && !covered, |d| d.children(self.render_bleed(cx)))
                    .child(div().size_full().relative().top(px(8. * (1. - page_p))).opacity(page_p).child(main)),
            )
            .when(show_right, |d| d.child(div().h_full().flex_none().relative().left(px(14. * (1. - pr))).opacity(pr).child(self.render_right(window, cx))))
            .when(!covered, |d| d.child(self.render_titlebar(window, cx)))
            .children(self.render_overlays(window, cx))
            .children(self.render_new_project(window, cx))
            .children(self.render_toast(window, cx))
            .child(self.fps.clone())
    }
}
