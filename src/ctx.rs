//! Composer context: access · machine · project · branch · model chips, each
//! opening a popover menu (port of the mock's COMPOSER CONTEXT), plus the
//! app-wide toast.

use crate::{app::{TakoApp, View}, pix::pix, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    component::input::{Input, InputEvent, InputState},
    *,
};
use std::time::{Duration, Instant};

pub struct Access {
    pub id: &'static str,
    pub name: &'static str,
    pub sub: &'static str,
    pub icon: &'static str,
}
pub const ACCESS: &[Access] = &[
    Access { id: "full", name: "Full access", sub: "Edit files and run commands without asking", icon: "bolt" },
    Access { id: "ask", name: "Ask before edits", sub: "Approve each change and command", icon: "shield" },
    Access { id: "read", name: "Read-only", sub: "Plan and explain; no changes", icon: "lock" },
];

pub struct Env {
    pub id: &'static str,
    pub name: &'static str,
    pub short: Option<&'static str>,
    pub sub: &'static str,
    pub icon: &'static str,
    pub st: Option<&'static str>,
    pub vm: bool,
}
pub struct EnvGroup {
    pub group: &'static str,
    pub note: Option<&'static str>,
    pub items: &'static [Env],
}
pub const ENVS: &[EnvGroup] = &[
    EnvGroup { group: "This computer", note: None, items: &[Env { id: "local", name: "My MacBook", short: None, sub: "macOS · 12 cores · 36 GB", icon: "laptop", st: Some("ok"), vm: false }] },
    EnvGroup {
        group: "Remote servers",
        note: None,
        items: &[
            Env { id: "devbox", name: "devbox-01", short: None, sub: "SSH · us-east-1 · 32 GB", icon: "server", st: Some("ok"), vm: false },
            Env { id: "gpu", name: "gpu-runner", short: None, sub: "SSH · eu-west · 1× A100", icon: "server", st: Some("idle"), vm: false },
            Env { id: "staging", name: "staging-api", short: None, sub: "SSH · offline since 2h", icon: "server", st: Some("off"), vm: false },
        ],
    },
    EnvGroup {
        group: "Ephemeral VMs",
        note: Some("A clean machine for each session, deleted when it ends"),
        items: &[
            Env { id: "vm-s", name: "Ephemeral · Small", short: Some("VM · Small"), sub: "2 vCPU · 4 GB · boots in ~8s", icon: "cloud", st: None, vm: true },
            Env { id: "vm-m", name: "Ephemeral · Medium", short: Some("VM · Medium"), sub: "4 vCPU · 16 GB · boots in ~12s", icon: "cloud", st: None, vm: true },
            Env { id: "vm-l", name: "Ephemeral · Large", short: Some("VM · Large"), sub: "8 vCPU · 32 GB · boots in ~20s", icon: "cloud", st: None, vm: true },
        ],
    },
];
pub const MODELS: &[(&str, &str)] = &[
    ("Opus 5.5", "Most capable · best for large changes"),
    ("Sonnet 5.5", "Fast and strong · everyday work"),
    ("Haiku 5.5", "Fastest · small edits and questions"),
    ("Codex", "Alternative agent"),
];
pub const EFFORT: &[&str] = &["Low", "Medium", "High", "Max"];

pub fn env(id: &str) -> &'static Env {
    ENVS.iter().flat_map(|g| g.items.iter()).find(|e| e.id == id).unwrap_or(&ENVS[0].items[0])
}

pub struct CtxState {
    pub access: &'static str,
    pub env: &'static str,
    pub project: String,
    pub branch: String,
    pub model: &'static str,
    pub effort: &'static str,
    pub open: Option<&'static str>,
    pub opened_at: Instant,
    pub query: Option<Entity<InputState>>,
    pub vm_boot: bool,
    pub toast: Option<(String, Instant)>,
}

impl CtxState {
    pub fn new() -> Self {
        CtxState {
            access: "full",
            env: "local",
            project: "tako".into(),
            branch: "main".into(),
            model: "Opus 5.5",
            effort: "High",
            open: None,
            opened_at: Instant::now(),
            query: None,
            vm_boot: false,
            toast: None,
        }
    }
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

impl TakoApp {
    pub fn toast(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        let at = Instant::now();
        self.ctx.toast = Some((msg.into(), at));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(2100)).await;
            this.update(cx, |this, cx| {
                if this.ctx.toast.as_ref().is_some_and(|t| t.1 == at) {
                    this.ctx.toast = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn ctx_project(&self) -> String {
        if self.view == View::Home { self.ctx.project.clone() } else { self.thread().repo.clone() }
    }
    pub fn ctx_branch(&self) -> String {
        if self.view == View::Home { self.ctx.branch.clone() } else { self.thread().branch.clone() }
    }

    pub fn open_ctx(&mut self, k: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.ctx.open == Some(k) {
            self.ctx.open = None;
            cx.notify();
            return;
        }
        self.ctx.open = Some(k);
        self.ctx.opened_at = Instant::now();
        self.ctx.query = None;
        if k == "project" || k == "branch" {
            let q = cx.new(|cx| InputState::new(window, cx).placeholder(if k == "project" { "Switch project" } else { "Find or create a branch" }));
            cx.subscribe(&q, |this, q, ev: &InputEvent, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    let v = q.read(cx).value().trim().to_string();
                    let k = this.ctx.open;
                    match k {
                        Some("branch") if !v.is_empty() => this.ctx_set("branch", v, cx),
                        Some("project") => {
                            if let Some(p) = this.projects.iter().find(|p| p.name.to_lowercase().contains(&v.to_lowercase())) {
                                let id = p.id.to_string();
                                this.ctx_set("project", id, cx);
                            }
                        }
                        _ => {}
                    }
                }
                cx.notify();
            })
            .detach();
            q.update(cx, |s, cx| s.focus(window, cx));
            self.ctx.query = Some(q);
        }
        cx.notify();
    }

    pub fn ctx_set(&mut self, k: &str, v: String, cx: &mut Context<Self>) {
        match k {
            "access" => self.ctx.access = ACCESS.iter().find(|a| a.id == v).map(|a| a.id).unwrap_or("full"),
            "model" => self.ctx.model = MODELS.iter().find(|m| m.0 == v).map(|m| m.0).unwrap_or("Opus 5.5"),
            "effort" => {
                self.ctx.effort = EFFORT.iter().find(|e| **e == v).copied().unwrap_or("High");
                cx.notify();
                return; // the effort segment keeps the menu open
            }
            "env" => {
                let e = env(&v);
                self.ctx.env = e.id;
                if e.vm {
                    self.ctx.vm_boot = true;
                    let name = e.name;
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(Duration::from_millis(1800)).await;
                        this.update(cx, |this, cx| {
                            this.ctx.vm_boot = false;
                            this.toast(format!("{name} is ready"), cx);
                        })
                        .ok();
                    })
                    .detach();
                }
            }
            "project" => {
                if self.view == View::Home {
                    self.ctx.project = v;
                    self.ctx.branch = "main".into();
                } else if let Some(t) = self.threads.iter().filter(|t| t.repo == v).min_by_key(|t| t.ago_min).map(|t| t.id) {
                    self.open_thread(t, cx);
                } else {
                    self.ctx.project = v;
                    self.ctx.branch = "main".into();
                    self.view = View::Home;
                }
            }
            "branch" => {
                let existed = self.branches().contains(&v);
                if self.view == View::Home {
                    self.ctx.branch = v.clone();
                } else {
                    let id = self.active;
                    if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
                        t.branch = v.clone();
                    }
                }
                if !existed {
                    self.toast(format!("Created branch {v}"), cx);
                }
            }
            _ => {}
        }
        self.ctx.open = None;
        cx.notify();
    }

    fn branches(&self) -> Vec<String> {
        let p = self.ctx_project();
        let mut v: Vec<String> = vec!["main".into(), "develop".into(), "release/2.4".into()];
        for t in self.threads.iter().filter(|t| t.repo == p) {
            if !v.contains(&t.branch) {
                v.push(t.branch.clone());
            }
        }
        v
    }

    /// The chip strip under the composer.
    pub fn render_ctx_chips(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let open = self.ctx.open;
        let pop = std::cell::RefCell::new(self.render_ctx_pop(window, cx));
        let chip = |id: &'static str| {
            let on = open == Some(id);
            div()
                .id(id)
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.))
                .px(px(7.))
                .py(px(4.))
                .rounded(px(7.))
                .text_color(if on { t.text } else { t.text_2 })
                .when(on, |d| d.bg(t.surface_hover))
                .hover(|s| s.bg(t.surface_hover).text_color(t.text))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| this.open_ctx(id, window, cx)))
                .relative()
                .when(on, |d| d.children(pop.borrow_mut().take().map(|p| div().absolute().top_0().left_0().child(p))))
        };
        let sep = || div().flex().flex_none().items_center().child(icon("chev-r", 12., t.text_3));
        let a = ACCESS.iter().find(|a| a.id == self.ctx.access).unwrap();
        let e = env(self.ctx.env);
        let pid = self.ctx_project();
        let p = self.projects.iter().find(|p| p.id == pid).cloned();
        let branch = self.ctx_branch();
        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .px(px(10.))
            .py(px(7.))
            .text_size(px(13.))
            .overflow_hidden()
            .child(chip("access").child(icon(a.icon, 14., t.text_2)).child(a.name))
            .child(
                chip("env")
                    .child(icon(e.icon, 14., t.text_2))
                    .child(e.short.unwrap_or(e.name))
                    .when(self.ctx.vm_boot, |d| d.child(spinner("vm-boot", 12., t.text_3)))
                    .when(!self.ctx.vm_boot && e.vm, |d| d.child(div().size(px(6.)).rounded_full().bg(t.green))),
            )
            .child(sep())
            .child(chip("project").when_some(p.clone(), |d, p| d.child(pix(p.icon, 14.))).child(p.map(|p| p.name.to_string()).unwrap_or(pid)))
            .child(sep())
            .child(chip("branch").child(icon("branch", 14., t.text_2)).child(div().whitespace_nowrap().child(branch)))
            .child(div().flex_1().min_w(px(8.)))
            .child(
                chip("model")
                    .child(icon("spark", 14., t.accent))
                    .child(div().font_weight(FontWeight::SEMIBOLD).text_color(t.text).child(self.ctx.model))
                    .child(div().text_color(t.text_3).child(self.ctx.effort)),
            )
    }

    /// The open context popover (above the chip strip), or None.
    pub fn render_ctx_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let k = self.ctx.open?;
        let t = cx.t();
        let p = ((self.ctx.opened_at.elapsed().as_secs_f32() * 1000.) / 160.).min(1.);
        if p < 1. {
            window.request_animation_frame();
        }
        let row = |id: SharedString, on: bool, disabled: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(10.))
                .w_full()
                .px(px(10.))
                .py(px(8.))
                .rounded(px(8.))
                .when(disabled, |d| d.opacity(0.45))
                .when(!disabled, |d| d.hover(|s| s.bg(t.surface_hover)))
                .when(on, |d| d)
        };
        let txt = |name: AnyElement, sub: Option<String>| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).truncate().child(name))
                .when_some(sub, |d, s| d.child(div().text_size(px(11.5)).text_color(t.text_3).truncate().child(s)))
        };
        let head = |s: &'static str| div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(t.text_3).pt(px(10.)).px(px(10.)).pb(px(4.)).child(s.to_uppercase());
        let check = |on: bool| if on { Some(icon("check", 14., t.accent)) } else { None };
        let foot_btn = |id: &'static str, ic: Option<&'static str>, label: &'static str, msg: &'static str| {
            div()
                .id(id)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .px(px(8.))
                .py(px(7.))
                .rounded(px(7.))
                .text_size(px(12.5))
                .text_color(t.text_2)
                .hover(|s| s.bg(t.surface_hover).text_color(t.text))
                .when_some(ic, |d, i| d.child(icon(i, 13., t.text_2)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.ctx.open = None;
                    this.toast(msg, cx);
                }))
        };
        let foot = || div().flex().gap(px(4.)).border_t_1().border_color(t.border).mt(px(6.)).pt(px(6.));
        let q = self.ctx.query.as_ref().map(|q| q.read(cx).value().to_string()).unwrap_or_default();
        let search = |q: &Entity<InputState>| div().flex().items_center().gap(px(8.)).h(px(34.)).px(px(10.)).mt(px(2.)).mx(px(2.)).mb(px(6.)).rounded(px(8.)).bg(t.surface).child(icon("search", 14., t.text_3)).child(div().flex_1().child(Input::new(q).appearance(false).text_size(px(13.))));

        let mut body = div().flex().flex_col();
        match k {
            "access" => {
                body = body.child(head("Permissions"));
                for a in ACCESS {
                    let on = self.ctx.access == a.id;
                    body = body.child(
                        row(SharedString::from(format!("ac-{}", a.id)), on, false)
                            .child(icon(a.icon, 16., t.text_2))
                            .child(txt(a.name.into_any_element(), Some(a.sub.into())))
                            .children(check(on))
                            .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("access", a.id.into(), cx))),
                    );
                }
            }
            "env" => {
                for g in ENVS {
                    body = body.child(head(g.group)).when_some(g.note, |d, n| d.child(div().text_size(px(11.5)).text_color(t.text_3).px(px(10.)).pb(px(6.)).mt(px(-2.)).child(n)));
                    for e in g.items {
                        let on = self.ctx.env == e.id;
                        let off = e.st == Some("off");
                        let dot = e.st.map(|s| match s {
                            "ok" => t.green,
                            "idle" => t.yellow,
                            _ => t.text_3,
                        });
                        let mut r = row(SharedString::from(format!("env-{}", e.id)), on, off)
                            .child(icon(e.icon, 16., t.text_2))
                            .child(txt(e.name.into_any_element(), Some(e.sub.into())))
                            .when_some(dot, |d, c| d.child(div().size(px(6.)).rounded_full().bg(c)))
                            .children(check(on));
                        if !off {
                            r = r.on_click(cx.listener(move |this, _, _, cx| this.ctx_set("env", e.id.into(), cx)));
                        }
                        body = body.child(r);
                    }
                }
                body = body.child(foot().child(foot_btn("f-ssh", Some("plus"), "Add SSH server", "SSH setup would open here")).child(foot_btn("f-envs", None, "Manage machines", "Machine settings would open here")));
            }
            "project" => {
                if let Some(qe) = &self.ctx.query {
                    body = body.child(search(qe));
                }
                let cur = self.ctx_project();
                let all: Vec<_> = self.projects.iter().filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q.to_lowercase())).cloned().collect();
                if all.is_empty() {
                    body = body.child(div().p(px(10.)).text_size(px(12.5)).text_color(t.text_3).child("No projects match"));
                }
                for p in all {
                    let n = self.threads.iter().filter(|t| t.repo == p.id).count();
                    let on = cur == p.id;
                    let id = p.id.to_string();
                    body = body.child(
                        row(SharedString::from(format!("pr-{}", p.id)), on, false)
                            .child(div().size(px(22.)).rounded(px(6.)).bg(t.surface).flex().items_center().justify_center().child(pix(p.icon, 14.)))
                            .child(txt(p.name.into_any_element(), Some(format!("{n} worktree{}", if n == 1 { "" } else { "s" }))))
                            .children(check(on))
                            .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("project", id.clone(), cx))),
                    );
                }
                body = body.child(foot().child(foot_btn("f-open", Some("folder"), "Open folder…", "A folder picker would open here")).child(foot_btn("f-clone", Some("github"), "Clone repository…", "Clone from URL would open here")));
            }
            "branch" => {
                if let Some(qe) = &self.ctx.query {
                    body = body.child(search(qe));
                }
                let cur = self.ctx_branch();
                let all: Vec<String> = self.branches().into_iter().filter(|b| q.is_empty() || b.to_lowercase().contains(&q.to_lowercase())).collect();
                let qt = q.trim().to_string();
                if !qt.is_empty() && !all.contains(&qt) {
                    let v = qt.clone();
                    body = body.child(
                        row("br-new".into(), false, false)
                            .child(icon("plus", 16., t.text_2))
                            .child(txt(div().flex().gap(px(4.)).child("Create").child(div().font_family(MONO).text_size(px(12.5)).child(qt.clone())).into_any_element(), Some(format!("from {cur}"))))
                            .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("branch", v.clone(), cx))),
                    );
                }
                let mut list = div().id("br-list").flex().flex_col().max_h(px(260.)).overflow_y_scroll();
                for b in all {
                    let on = cur == b;
                    let v = b.clone();
                    list = list.child(
                        row(SharedString::from(format!("br-{b}")), on, false)
                            .child(icon("branch", 16., t.text_2))
                            .child(txt(div().font_family(MONO).text_size(px(12.5)).child(b.clone()).into_any_element(), if b == "main" { Some("default".into()) } else { None }))
                            .children(check(on))
                            .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("branch", v.clone(), cx))),
                    );
                }
                body = body.child(list);
            }
            _ => {
                body = body.child(head("Model"));
                for (m, sub) in MODELS {
                    let on = self.ctx.model == *m;
                    body = body.child(
                        row(SharedString::from(format!("m-{m}")), on, false)
                            .child(icon("spark", 16., t.accent))
                            .child(txt((*m).into_any_element(), Some((*sub).into())))
                            .children(check(on))
                            .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("model", (*m).into(), cx))),
                    );
                }
                body = body.child(head("Effort")).child(div().flex().gap(px(2.)).p(px(3.)).mt(px(2.)).mx(px(6.)).mb(px(6.)).rounded(px(8.)).bg(t.surface).children(EFFORT.iter().map(|e| {
                    let on = self.ctx.effort == *e;
                    div()
                        .id(SharedString::from(format!("ef-{e}")))
                        .flex_1()
                        .flex()
                        .justify_center()
                        .p(px(6.))
                        .rounded(px(6.))
                        .text_size(px(12.5))
                        .text_color(if on { t.text } else { t.text_3 })
                        .when(on, |d| d.bg(t.surface_active))
                        .child(*e)
                        .on_click(cx.listener(move |this, _, _, cx| this.ctx_set("effort", (*e).into(), cx)))
                })));
            }
        }
        let _ = leak;
        let e = ease_out(p);
        Some(
            deferred(
                anchored().anchor(Anchor::BottomLeft).child(
                    div()
                        .id("ctx-pop")
                        .occlude()
                        .mb(px(8.))
                        .w(px(320.))
                        .max_h(px(460.))
                        .overflow_y_scroll()
                        .p(px(6.))
                        .rounded(px(12.))
                        .border_1()
                        .border_color(t.border)
                        .bg(t.panel_solid)
                        .shadow_xl()
                        .opacity(e)
                        .child(body)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.ctx.open = None;
                            cx.notify();
                        })),
                ),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }

    pub fn render_toast(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (msg, at) = self.ctx.toast.clone()?;
        let t = cx.t();
        let ms = at.elapsed().as_secs_f32() * 1000.;
        let p = if ms < 250. { ease_snappy(ms / 250.) } else if ms > 1800. { 1. - ((ms - 1800.) / 300.).min(1.) } else { 1. };
        if ms < 250. || ms > 1800. {
            window.request_animation_frame();
        }
        Some(
            div()
                .absolute()
                .bottom(px(28.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .px(px(14.))
                        .py(px(9.))
                        .rounded(px(9.))
                        .bg(t.panel_solid)
                        .border_1()
                        .border_color(t.border)
                        .shadow_lg()
                        .text_size(px(13.))
                        .opacity(p)
                        .relative()
                        .top(px(10. * (1. - p)))
                        .child(msg),
                )
                .into_any_element(),
        )
    }
}
