//! Plugins page (port of the mock's PLUGINS section): marketplace with
//! Discover / Installed tabs, category chips, featured cards, a popular grid
//! and a detail sheet that slides in from the right.
use super::automations::{btn, hexc, ib, mix, transparent, Btn};
use crate::{app::TakoApp, theme::ActiveTheme, ui::*};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{cell::Cell, collections::HashSet, rc::Rc};

const CATS: [&str; 8] = ["All", "Testing", "Code review", "Deploy", "Security", "Design", "Data", "Writing"];

struct Plugin {
    id: &'static str,
    name: &'static str,
    by: &'static str,
    cat: &'static str,
    c: u32,
    m: &'static str,
    rating: f32,
    installs: u32,
    featured: bool,
    verified: bool,
    tag: &'static str,
    desc: &'static str,
    adds: &'static [&'static str],
}

macro_rules! p {
    ($id:expr, $name:expr, $by:expr, $cat:expr, $c:expr, $m:expr, $r:expr, $i:expr, $f:expr, $v:expr, $tag:expr, $desc:expr, [$($a:expr),*]) => {
        Plugin { id: $id, name: $name, by: $by, cat: $cat, c: $c, m: $m, rating: $r, installs: $i, featured: $f, verified: $v, tag: $tag, desc: $desc, adds: &[$($a),*] }
    };
}

const PLUGINS: &[Plugin] = &[
    p!("preview", "Deploy Preview", "Northwind Labs", "Deploy", 0x4f8ef7, "DP", 4.9, 48200, true, true, "A live preview URL for every worktree",
       "Spins up an isolated preview environment whenever an agent opens a worktree, and posts the link in the chat and on the PR.", ["1 MCP server", "2 commands", "1 skill"]),
    p!("flaky", "Flaky Hunter", "Testbench", "Testing", 0xe2795a, "FH", 4.8, 31700, true, true, "Find, quarantine and fix flaky tests",
       "Re-runs suspicious tests in parallel, scores their flakiness, quarantines the worst offenders and hands each one to an agent to fix.", ["3 commands", "1 skill"]),
    p!("sweep", "Security Sweep", "Halyard", "Security", 0x5fc48a, "SS", 4.7, 26100, true, true, "Secrets and dependency scanning on every run",
       "Checks each change for leaked secrets and vulnerable dependencies before the agent commits, and explains any findings in plain language.", ["1 MCP server", "1 hook"]),
    p!("notes", "Release Notes", "Inkwell", "Writing", 0xc77dff, "RN", 4.6, 19400, false, false, "Draft release notes from merged PRs",
       "Groups merged pull requests by area and writes release notes in your project's voice.", ["1 command", "1 skill"]),
    p!("schema", "Schema Guard", "Tablewise", "Data", 0xe2b75a, "SG", 4.7, 12800, false, true, "Review migrations before they ship",
       "Flags locking migrations, missing indexes and destructive changes, and suggests safer rollouts.", ["1 hook", "1 skill"]),
    p!("design", "Token Lint", "Studio Fold", "Design", 0xec7aa0, "TL", 4.5, 9300, false, false, "Keep UI changes on your design tokens",
       "Catches hard-coded colours, spacing and type in UI diffs and offers the matching token.", ["1 command"]),
    p!("reviewer", "Second Opinion", "Tako", "Code review", 0x7aa7e8, "SO", 4.8, 41200, false, true, "A reviewer agent on every PR",
       "Runs an independent review agent over each pull request and leaves inline comments ranked by severity.", ["1 skill", "1 hook"]),
    p!("tests", "Test Writer", "Testbench", "Testing", 0x4fb3c8, "TW", 4.4, 15600, false, false, "Fill coverage gaps in changed files",
       "Reads the diff, finds untested branches and writes focused tests that follow your existing patterns.", ["2 commands"]),
    p!("perf", "Perf Budget", "Northwind Labs", "Code review", 0x9fd49a, "PB", 4.3, 6900, false, false, "Fail PRs that blow the bundle budget",
       "Builds before and after, compares bundle sizes and blocks merges that exceed your budget.", ["1 hook"]),
    p!("scribe", "Incident Scribe", "Halyard", "Writing", 0xe5675f, "IS", 4.6, 7800, false, false, "Postmortems from alerts and timelines",
       "Collects the alert, deploys and chat from an incident and drafts a blameless postmortem.", ["1 MCP server", "1 skill"]),
    p!("i18n", "i18n Sync", "Babel & Co", "Writing", 0xb48ae8, "iS", 4.2, 5400, false, false, "Keep translation files in step",
       "Adds missing keys to every locale when strings change and drafts translations for review.", ["1 command"]),
    p!("mock", "API Mocker", "Stubly", "Data", 0xcfc9c1, "AM", 4.4, 11200, false, false, "Mock servers from your OpenAPI spec",
       "Starts a local mock of any API in your spec so agents can test integrations without real credentials.", ["1 MCP server"]),
];

fn fmt(n: u32) -> String {
    if n >= 1000 {
        let k = n as f32 / 1000.;
        if n >= 10000 { format!("{:.0}k", k) } else { format!("{:.1}k", k) }
    } else {
        n.to_string()
    }
}

pub struct PluginsState {
    installed: HashSet<&'static str>,
    installed_order: Vec<&'static str>,
    tab_installed: bool,
    cat: &'static str,
    search: Entity<InputState>,
    detail: Option<&'static str>,
    sheet_tween: Option<Tween>,
    sheet_focus: FocusHandle,
    grid_w: Rc<Cell<f32>>,
}

impl PluginsState {
    pub fn new(window: &mut Window, cx: &mut Context<TakoApp>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search plugins"));
        cx.subscribe_in(&search, window, |_: &mut TakoApp, _, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        })
        .detach();
        PluginsState {
            installed: HashSet::from(["reviewer"]),
            installed_order: vec!["reviewer"],
            tab_installed: false,
            cat: "All",
            search,
            detail: None,
            sheet_tween: None,
            sheet_focus: cx.focus_handle(),
            grid_w: Rc::new(Cell::new(1040.)),
        }
    }

    fn toggle(&mut self, id: &'static str) {
        if !self.installed.remove(id) {
            self.installed.insert(id);
            self.installed_order.push(id);
        } else {
            self.installed_order.retain(|x| *x != id);
        }
    }
}

/// `.pl-tile` — the initials badge.
fn tile(p: &Plugin, big: bool, t: &crate::theme::Theme) -> Div {
    let c = hexc(p.c);
    let sz = if big { 48. } else { 36. };
    div()
        .size(px(sz))
        .flex_none()
        .rounded(px(if big { 12. } else { 9. }))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(if big { 16. } else { 13. }))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c)
        .bg(if big { t.panel_solid } else { mix(c, transparent(), 0.16) })
        .border_1()
        .border_color(mix(c, transparent(), 0.30))
        .child(p.m)
}

/// `.pl-name` — name (+ verified badge) over "publisher · category".
fn name_block(p: &Plugin, size: f32, t: &crate::theme::Theme) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(size))
                .font_weight(FontWeight::SEMIBOLD)
                .whitespace_nowrap()
                .overflow_hidden()
                .child(p.name)
                .when(p.verified, |d| {
                    d.child(div().size(px(13.)).flex_none().rounded_full().bg(t.blue).flex().items_center().justify_center().child(icon("check", 9., t.bg)))
                }),
        )
        .child(div().text_size(px(12.)).text_color(t.text_3).child(format!("{} · {}", p.by, p.cat)))
}

/// The dotted pattern of `.pl-fart` (radial-gradient dots on a 14px grid).
fn dots(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |b, _, window, _| {
            let mut y = b.origin.y + px(7.);
            while y < b.origin.y + b.size.height {
                let mut x = b.origin.x + px(7.);
                while x < b.origin.x + b.size.width {
                    window.paint_quad(fill(Bounds::new(point(x - px(1.15), y - px(1.15)), size(px(2.3), px(2.3))), color).corner_radii(px(1.15)));
                    x += px(14.);
                }
                y += px(14.);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl TakoApp {
    fn pl_open(&mut self, id: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let s = &mut self.pages.plugins;
        s.detail = Some(id);
        s.sheet_tween = Some(Tween::new(280.));
        let f = s.sheet_focus.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    fn inst_btn(&self, p: &'static Plugin, key: &str, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = cx.t();
        let on = self.pages.plugins.installed.contains(p.id);
        let id = p.id;
        btn(SharedString::from(format!("inst-{key}-{}", p.id)), if on { Btn::On } else { Btn::Plain }, true, false, cx)
            .when(on, |d| d.child(icon("check", 14., t.green)).child("Installed"))
            .when(!on, |d| d.child("Install"))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.pages.plugins.toggle(id);
                cx.notify();
            }))
    }

    fn foot(&self, p: &'static Plugin, key: &str, cx: &mut Context<Self>) -> Div {
        let t = cx.t();
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .text_size(px(12.))
            .text_color(t.text_3)
            .child(format!("★ {}", p.rating))
            .child(format!("{} installs", fmt(p.installs)))
            .child(div().flex_1())
            .child(self.inst_btn(p, key, cx))
    }

    fn pl_card(&self, p: &'static Plugin, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let id = p.id;
        div()
            .id(SharedString::from(format!("plc-{}", p.id)))
            .flex_1()
            .min_w_0()
            .border_1()
            .border_color(t.border)
            .rounded(px(12.))
            .bg(t.panel_solid)
            .px(px(16.))
            .py(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .cursor_pointer()
            .hover(|s| s.border_color(mix(t.text, t.border, 0.18)))
            .child(div().flex().items_center().gap(px(12.)).child(tile(p, false, &t)).child(name_block(p, 14., &t)))
            .child(div().text_size(px(13.)).line_height(relative(1.5)).text_color(t.text_2).child(p.tag))
            .child(div().flex_1())
            .child(self.foot(p, "g", cx))
            .on_click(cx.listener(move |this, _, window, cx| this.pl_open(id, window, cx)))
            .into_any_element()
    }

    fn pl_feature(&self, p: &'static Plugin, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let c = hexc(p.c);
        let id = p.id;
        div()
            .id(SharedString::from(format!("plf-{}", p.id)))
            .flex_1()
            .min_w_0()
            .relative()
            .border_1()
            .border_color(t.border)
            .rounded(px(12.))
            .bg(t.panel_solid)
            .overflow_hidden()
            .flex()
            .flex_col()
            .cursor_pointer()
            .hover(move |s| s.border_color(mix(c, t.border, 0.45)).top(px(-2.)))
            .child(
                div()
                    .relative()
                    .h(px(112.))
                    .flex_none()
                    .bg(mix(c, t.panel_solid, 0.14))
                    .border_b_1()
                    .border_color(t.border)
                    .child(dots(mix(c, transparent(), 0.35)))
                    .child(
                        div()
                            .size_full()
                            .p(px(16.))
                            .flex()
                            .items_end()
                            .justify_between()
                            .child(tile(p, true, &t))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(t.text_2)
                                    .bg(t.panel_solid)
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(t.border)
                                    .child(p.adds.join(" · ")),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .pt(px(14.))
                    .px(px(16.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(name_block(p, 14., &t))
                    .child(div().text_size(px(13.)).line_height(relative(1.5)).text_color(t.text_2).child(p.tag))
                    .child(div().flex_1())
                    .child(self.foot(p, "f", cx)),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.pl_open(id, window, cx)))
            .into_any_element()
    }

    pub fn render_plugins(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let s = &self.pages.plugins;
        let q = s.search.read(cx).value().to_lowercase();
        let mut list: Vec<&'static Plugin> = PLUGINS
            .iter()
            .filter(|p| (s.cat == "All" || p.cat == s.cat) && (q.is_empty() || format!("{}{}{}{}", p.name, p.tag, p.by, p.cat).to_lowercase().contains(&q)))
            .collect();
        if s.tab_installed {
            list = PLUGINS.iter().filter(|p| s.installed.contains(p.id)).collect();
        }
        let show_featured = !s.tab_installed && s.cat == "All" && q.is_empty();
        if show_featured {
            list.retain(|p| !p.featured);
        }

        let tab = |id: &'static str, on: bool| {
            div()
                .id(id)
                .relative()
                .py(px(10.))
                .text_size(px(13.5))
                .flex()
                .items_center()
                .text_color(if on { t.text } else { t.text_3 })
                .cursor_pointer()
                .hover(|s| s.text_color(t.text))
                .when(on, |d| d.child(div().absolute().left_0().right_0().bottom(px(-1.)).h(px(2.)).rounded(px(2.)).bg(t.text)))
        };
        let tabs = div()
            .flex()
            .gap(px(22.))
            .mt(px(24.))
            .border_b_1()
            .border_color(t.border)
            .child(tab("pl-discover", !s.tab_installed).child("Discover").on_click(cx.listener(|this, _, _, cx| {
                this.pages.plugins.tab_installed = false;
                cx.notify();
            })))
            .child(
                tab("pl-installed", s.tab_installed)
                    .child("Installed")
                    .child(div().ml(px(7.)).text_size(px(11.5)).font_weight(FontWeight::MEDIUM).text_color(t.text_3).child(s.installed.len().to_string()))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pages.plugins.tab_installed = true;
                        cx.notify();
                    })),
            );
        let cats = (!s.tab_installed).then(|| {
            div().flex().flex_wrap().gap(px(6.)).mt(px(18.)).children(CATS.iter().enumerate().map(|(i, c)| {
                let on = *c == s.cat;
                let c: &'static str = c;
                div()
                    .id(("pl-cat", i))
                    .px(px(12.))
                    .py(px(6.))
                    .rounded_full()
                    .text_size(px(12.5))
                    .border_1()
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.text).text_color(t.bg).border_color(transparent()))
                    .when(!on, |d| d.text_color(t.text_2).border_color(t.border).hover(|s| s.text_color(t.text)))
                    .child(c)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pages.plugins.cat = c;
                        cx.notify();
                    }))
            }))
        });
        let h2 = |s: &'static str| div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).mt(px(32.)).mb(px(12.)).child(s);

        let featured = show_featured.then(|| {
            let cards: Vec<AnyElement> = PLUGINS.iter().filter(|p| p.featured).map(|p| self.pl_feature(p, cx)).collect();
            div().child(h2("Featured")).child(div().flex().gap(px(14.)).children(cards)).child(h2("Popular"))
        });

        // auto-fill grid, minmax(250px, 1fr), gap 12
        let w = s.grid_w.get();
        let cols = (((w + 12.) / 262.).floor() as usize).max(1);
        let grid_w = s.grid_w.clone();
        let content: AnyElement = if list.is_empty() {
            div()
                .py(px(48.))
                .text_center()
                .text_color(t.text_3)
                .child(if s.tab_installed { "Nothing installed yet. Browse Discover to add your first plugin." } else { "No plugins match your search." })
                .into_any_element()
        } else {
            let mut rows: Vec<AnyElement> = vec![];
            for chunk in list.chunks(cols) {
                let mut cells: Vec<AnyElement> = chunk.iter().map(|p| self.pl_card(p, cx)).collect();
                for _ in chunk.len()..cols {
                    cells.push(div().flex_1().min_w_0().into_any_element());
                }
                rows.push(div().flex().gap(px(12.)).children(cells).into_any_element());
            }
            div().relative().mt(px(4.)).flex().flex_col().gap(px(12.)).child(measurer_w(grid_w)).children(rows).into_any_element()
        };

        let wrap = div()
            .max_w(px(1120.))
            .w_full()
            .mx_auto()
            .pt(px(32.))
            .px(px(40.))
            .pb(px(72.))
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(24.))
                    .child(
                        div()
                            .child(div().text_size(px(28.)).font_weight(FontWeight::SEMIBOLD).child("Plugins"))
                            .child(div().mt(px(6.)).text_color(t.text_2).child("Give your agents new tools, skills and integrations.")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .w(px(300.))
                            .h(px(36.))
                            .px(px(12.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(if s.search.read(cx).focus_handle(cx).is_focused(window) { mix(t.accent, t.border, 0.5) } else { t.border })
                            .bg(t.panel_solid)
                            .child(icon("search", 15., t.text_3))
                            .child(div().flex_1().min_w_0().child(Input::new(&s.search).appearance(false).bordered(false).p_0().text_size(px(13.5)))),
                    ),
            )
            .child(tabs)
            .children(cats)
            .children(featured)
            .child(content);

        let sheet = s.detail.and_then(|id| PLUGINS.iter().find(|p| p.id == id)).map(|p| self.pl_sheet(p, window, cx));

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().h(px(44.)).flex_none())
            .child(
                div()
                    .id("pl-page")
                    .flex_1()
                    .min_h_0()
                    .border_t_1()
                    .border_l_1()
                    .border_color(t.border)
                    .rounded_tl(px(12.))
                    .bg(t.bg)
                    .overflow_y_scroll()
                    .child(wrap),
            )
            .children(sheet)
    }

    fn pl_sheet(&self, p: &'static Plugin, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.t();
        let s = &self.pages.plugins;
        let e = s.sheet_tween.as_ref().map(|tw| tw.raw()).unwrap_or(1.);
        if e < 1. {
            window.request_animation_frame();
        }
        let slide = ease_snappy(e);
        let fade = (e * 280. / 200.).min(1.);
        let vp = window.viewport_size();
        let on = s.installed.contains(p.id);
        let id = p.id;
        let h4 = |s: &'static str| div().mt(px(10.)).text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(t.text_3).child(s.to_uppercase());
        let stat = |b: String, sm: &'static str, first: bool| {
            div()
                .flex_1()
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .when(!first, |d| d.border_l_1().border_color(t.border))
                .child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(14.)).child(b))
                .child(div().text_size(px(11.5)).text_color(t.text_3).child(sm))
        };
        let version = format!("v2.{}.0", (p.m.as_bytes()[0] as u32) % 9);
        let has = |k: &str| p.adds.iter().any(|a| a.contains(k));
        let mut perms = vec!["Read files in worktrees"];
        if has("MCP") {
            perms.push("Network access to its own server");
        }
        if has("hook") {
            perms.push("Run before commits");
        }
        let sheet = div()
            .id("pl-sheet")
            .track_focus(&s.sheet_focus)
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(-24. * (1. - slide)))
            .w(px(420.))
            .opacity(slide)
            .bg(t.panel_solid)
            .border_l_1()
            .border_color(t.border)
            .pt(px(28.))
            .px(px(28.))
            .pb(px(40.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(14.))
            .occlude()
            .child(div().flex().items_center().gap(px(12.)).child(tile(p, true, &t)).child(name_block(p, 18., &t)))
            .child(
                div()
                    .flex()
                    .border_1()
                    .border_color(t.border)
                    .rounded(px(10.))
                    .child(stat(format!("★ {}", p.rating), "Rating", true))
                    .child(stat(fmt(p.installs), "Installs", false))
                    .child(stat(version, "Version", false)),
            )
            .child(
                btn("pl-sheet-inst", if on { Btn::On } else { Btn::Primary }, false, false, cx)
                    .w_full()
                    .h(px(36.))
                    .when(on, |d| d.child(icon("check", 14., t.green)).child("Installed · Remove"))
                    .when(!on, |d| d.child("Install for all worktrees"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pages.plugins.toggle(id);
                        cx.notify();
                    })),
            )
            .child(h4("About"))
            .child(div().text_color(t.text_2).line_height(relative(1.6)).child(p.desc))
            .child(h4("Adds"))
            .child(div().flex().flex_col().gap(px(8.)).children(p.adds.iter().map(|a| {
                let ic = if a.contains("MCP") {
                    "globe"
                } else if a.contains("hook") {
                    "bolt"
                } else if a.contains("skill") {
                    "spark"
                } else {
                    "term"
                };
                div().flex().items_center().gap(px(10.)).child(icon(ic, 15., t.accent)).child(*a)
            })))
            .child(h4("Permissions"))
            .child(div().flex().flex_col().gap(px(8.)).children(perms.into_iter().map(|x| {
                div().flex().items_start().text_color(t.text_2).child(div().mt(px(8.)).ml(px(2.)).mr(px(9.)).size(px(5.)).rounded_full().bg(t.text_3).flex_none()).child(x)
            })))
            .child(div().absolute().top(px(16.)).right(px(16.)).child(ib("pl-x", "x", 28., 16., cx).on_click(cx.listener(|this, _, _, cx| {
                this.pages.plugins.detail = None;
                cx.notify();
            }))))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" {
                    this.pages.plugins.detail = None;
                    cx.notify();
                }
            }));
        let overlay = div()
            .w(vp.width)
            .h(vp.height)
            .relative()
            .child(
                div()
                    .id("pl-scrim")
                    .absolute()
                    .size_full()
                    .bg(hsla(0., 0., 0., 0.35 * fade))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pages.plugins.detail = None;
                        cx.notify();
                    })),
            )
            .child(sheet);
        deferred(anchored().position(point(px(0.), px(0.))).child(overlay)).with_priority(2).into_any_element()
    }
}

/// Records the laid-out width of its parent (for the auto-fill grid).
fn measurer_w(cell: Rc<Cell<f32>>) -> impl IntoElement {
    canvas(move |b, _, _| cell.set(f32::from(b.size.width)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

/// Scripted actions (TAKO_ACT): `plugins:installed`, `plugins:discover`,
/// `plugins:cat:<Category>`, `plugins:search:<q>`, `plugins:open:<id>`,
/// `plugins:install:<id>`.
pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k != "plugins" {
        return false;
    }
    let (a, b) = v.split_once(':').unwrap_or((v, ""));
    let find = |b: &str| PLUGINS.iter().find(|p| p.id == b).map(|p| p.id);
    match a {
        "installed" => app.pages.plugins.tab_installed = true,
        "discover" => app.pages.plugins.tab_installed = false,
        "cat" => {
            if let Some(c) = CATS.iter().find(|c| c.eq_ignore_ascii_case(b)) {
                app.pages.plugins.cat = c;
            }
        }
        "search" => {
            let s = app.pages.plugins.search.clone();
            s.update(cx, |s, cx| s.set_value(b.to_string(), window, cx));
        }
        "open" => {
            if let Some(id) = find(b) {
                app.pl_open(id, window, cx);
            }
        }
        "install" => {
            if let Some(id) = find(b) {
                app.pages.plugins.toggle(id);
            }
        }
        _ => return false,
    }
    cx.notify();
    true
}
