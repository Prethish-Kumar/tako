//! Sidebar: new chat / new project, worktrees grouped by project with
//! pixel icons and labels, and the smooth fold animation.

use gpui_kit::prelude::FluentBuilder as _;
use crate::{
    app::TakoApp,
    data::{self, PrState, Thread},
    pix::pix,
    theme::ActiveTheme,
    ui::*,
};
use gpui_kit::*;
use std::{cell::Cell, collections::{HashMap, HashSet}, rc::Rc};

pub struct Fold {
    pub opening: bool,
    pub tween: Tween,
    pub from: f32,
}

#[derive(Default)]
pub struct SidebarState {
    pub collapsed: HashSet<String>,
    pub folds: HashMap<String, Fold>,
    pub heights: HashMap<String, Rc<Cell<f32>>>,
}

impl TakoApp {
    pub fn fold_project(&mut self, pid: &str, cx: &mut Context<Self>) {
        let s = &mut self.sidebar;
        let h = s.heights.entry(pid.to_string()).or_default().get();
        let shut = s.collapsed.contains(pid);
        // Current visual height if a fold is mid-flight, so reversing is seamless.
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

        // finish folds
        let mut animating = false;
        let done: Vec<String> = self.sidebar.folds.iter().filter(|(_, f)| f.tween.done()).map(|(k, _)| k.clone()).collect();
        for k in done {
            let f = self.sidebar.folds.remove(&k).unwrap();
            if !f.opening {
                self.sidebar.collapsed.insert(k);
            }
        }
        if !self.sidebar.folds.is_empty() {
            animating = true;
        }

        let mut groups: Vec<AnyElement> = vec![];
        for p in self.projects.clone() {
            let list: Vec<Thread> = self.threads.iter().filter(|t| t.repo == p.id).cloned().collect();
            groups.push(self.project_block(&p, list, cx).into_any_element());
        }
        if animating {
            window.request_animation_frame();
        }

        div()
            .w(px(300.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(44.))
            .border_r_1()
            .border_color(t.border)
            .child(sb_row("sb-new", "plus", "New chat").on_click(cx.listener(|this, _, _, cx| this.go(crate::app::View::Home, cx))))
            .child(sb_row("sb-proj", "folder", "New project"))
            .child(
                div()
                    .mx(px(8.))
                    .mt(px(1.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id("sb-group")
                            .flex_1()
                            .h(px(27.))
                            .px(px(12.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.5))
                            .text_color(t.text_2)
                            .hover(|s| s.bg(t.surface_hover))
                            .child(icon("list", 14., t.text_3))
                            .child("Grouped by project")
                            .child(div().flex_1())
                            .child(icon("sort", 14., t.text_3)),
                    )
                    .child(div().id("sb-sel").size(px(28.)).ml(px(2.)).rounded(px(7.)).flex().items_center().justify_center().hover(|s| s.bg(t.surface_hover)).child(icon("checks", 14., t.text_3))),
            )
            .child(div().id("threads").flex_1().min_h_0().overflow_y_scroll().pt(px(8.)).pb(px(12.)).children(groups))
    }

    fn project_block(&mut self, p: &data::Project, list: Vec<Thread>, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let pid = p.id.to_string();
        let shut = self.sidebar.collapsed.contains(&pid);
        let fold = self.sidebar.folds.get(&pid);
        let natural = self.sidebar.heights.entry(pid.clone()).or_default().clone();
        let label = p.label.and_then(|l| data::LABELS.iter().find(|x| x.id == l));
        // chevron: 0 = open, -90° = shut
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
        let in_chats = self.view == crate::app::View::Chats;
        let pid2 = pid.clone();
        let pid3 = pid.clone();
        let n = list.len();
        let rows_p = fold.map(|f| (f.opening, f.tween.clone()));

        div()
            .flex()
            .flex_col()
            .mb(px(6.))
            .child(
                div()
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
                    .child(div().size(px(22.)).rounded(px(6.)).bg(t.surface).flex().items_center().justify_center().child(pix(p.icon, 14.)))
                    .child(
                        div()
                            .id(SharedString::from(format!("pjn-{pid}")))
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(p.name)
                            .on_click(cx.listener(move |this, _, _, cx| this.fold_project(&pid2, cx))),
                    )
                    .when_some(label, |d, l| {
                        let c: Hsla = rgb(l.color).into();
                        d.child(div().text_size(px(10.5)).px(px(6.)).py(px(1.)).rounded(px(4.)).text_color(c).bg(c.opacity(0.14)).child(l.name))
                    })
                    .child(div().flex_1())
                    .child(div().text_size(px(11.5)).text_color(t.text_3).child(n.to_string()))
                    .child(div().size(px(22.)).rounded(px(5.)).flex().items_center().justify_center().invisible().group_hover("pjh", |s| s.visible()).child(icon("dots", 13., t.text_3)))
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
                            .on_click(cx.listener(move |this, _, _, cx| this.fold_project(&pid3, cx))),
                    ),
            )
            .when(show_list, |d| {
                let rows: Vec<AnyElement> = list
                    .iter()
                    .enumerate()
                    .map(|(i, th)| {
                        let el = self.wt_row(th, active == th.id && in_chats, cx).into_any_element();
                        // rows fade/lift: closing bottom-first, opening top-first
                        match &rows_p {
                            Some((opening, tw)) => {
                                let ms = tw.start.elapsed().as_secs_f32() * 1000.;
                                let (delay, dur) = if *opening { (30. + i as f32 * 22., 260.) } else { ((n - 1 - i) as f32 * 12., 160.) };
                                let p = ((ms - delay) / dur).clamp(0., 1.);
                                let (o, y) = if *opening { let e = ease_snappy(p); (e, -6. * (1. - e)) } else { let e = p * p; (1. - e, -4. * e) };
                                div().opacity(o).relative().top(px(y)).child(el).into_any_element()
                            }
                            None => el,
                        }
                    })
                    .collect();
                d.child(
                    div()
                        .when_some(list_h, |d, h| d.h(px(h)).overflow_hidden())
                        .child(div().relative().flex_none().flex().flex_col().gap(px(1.)).mt(px(2.)).children(rows).child(measurer(natural.clone()))),
                )
            })
    }

    fn wt_row(&mut self, th: &Thread, active: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = th.id;
        let pr_c = match th.pr_state {
            PrState::Open => t.green,
            PrState::Merged => rgb(0xb48ae8).into(),
            PrState::Closed => t.red,
            PrState::Draft => t.text_3,
        };
        div()
            .id(("wt", id))
            .mx(px(8.))
            .pl(px(14.))
            .pr(px(10.))
            .py(px(7.))
            .rounded(px(8.))
            .flex()
            .flex_col()
            .gap(px(3.))
            .when(active, |d| d.bg(t.surface_active))
            .when(!active, |d| d.hover(|s| s.bg(t.surface_hover)))
            .on_click(cx.listener(move |this, _, _, cx| this.open_thread(id, cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().min_w_0().text_size(px(13.)).truncate().child(th.title.clone()))
                    .when(th.running, |d| d.child(spinner(("wt-spin", id), 11., t.accent)))
                    .when(!th.running && th.unread, |d| d.child(div().size(px(7.)).rounded_full().bg(t.accent)))
                    .when(!th.running, |d| d.child(div().text_size(px(11.5)).text_color(t.text_3).child(data::ago(th.ago_min)))),
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
}
