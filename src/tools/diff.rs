//! Changes tab: file list + unified diff, "Viewed" checkboxes, Comment/Approve.
//! Also the single-file variant opened from the right panel.

use super::{cbtn, data::*, st, st_badge, checkbox, tr};
use crate::{app::TakoApp, chat::{Tab, TabKind}, theme::{ActiveTheme, Theme}, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct DiffState {
    pub scroll: ScrollHandle,
    pub folded: HashSet<usize>,
    /// Single-file mode: index into CHANGES.
    pub file: Option<usize>,
}

impl TakoApp {
    pub fn diff_toggle_viewed(&mut self, tab: usize, i: usize) {
        st(|s| {
            let p = CHANGES[i].path.to_string();
            let now = !s.viewed.contains(&p);
            if now {
                s.viewed.insert(p);
            } else {
                s.viewed.remove(&p);
            }
            let d = s.diff.entry(tab).or_default();
            if d.file.is_none() {
                if now {
                    d.folded.insert(i);
                } else {
                    d.folded.remove(&i);
                }
            }
        });
    }

    /// Open (or reuse) a single-file diff tab — the mock's `forgeOpenDiff`.
    pub fn open_diff_file(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(i) = CHANGES.iter().position(|c| c.path == path) else { return };
        let existing = st(|s| s.diff.iter().find(|(_, d)| d.file == Some(i)).map(|(id, _)| *id));
        if let Some(id) = existing.filter(|id| self.chat.tabs.iter().any(|t| t.id == *id)) {
            self.chat.active_tab = id;
            cx.notify();
            return;
        }
        // single-file diffs reuse the one you're on
        let act = self.chat.active_tab;
        let reuse = st(|s| s.diff.get(&act).is_some_and(|d| d.file.is_some()));
        let id = if reuse {
            self.rename_tab(act, CHANGES[i].name());
            act
        } else {
            self.open_tool_tab(TabKind::Diff, CHANGES[i].name(), cx)
        };
        st(|s| s.diff.entry(id).or_default().file = Some(i));
        cx.notify();
    }

    pub(super) fn render_diff(&mut self, tab: &Tab, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        let (scroll, folded, file, viewed) = st(|s| {
            let d = s.diff.entry(id).or_default();
            (d.scroll.clone(), d.folded.clone(), d.file, s.viewed.clone())
        });
        let is_viewed = |i: usize| viewed.contains(CHANGES[i].path);

        if let Some(fi) = file {
            let n = CHANGES.len();
            let nav = move |d: isize| {
                move |this: &mut TakoApp, _: &ClickEvent, _: &mut Window, cx: &mut Context<TakoApp>| {
                    let j = (fi as isize + d).rem_euclid(n as isize) as usize;
                    this.rename_tab(id, CHANGES[j].name());
                    st(|s| s.diff.entry(id).or_default().file = Some(j));
                    cx.notify();
                }
            };
            return div()
                .id(("df-single", id))
                .flex_1()
                .h_full()
                .overflow_y_scroll()
                .child(
                    div()
                        .max_w(px(1100.))
                        .w_full()
                        .mx_auto()
                        .px(px(18.))
                        .pt(px(14.))
                        .pb(px(40.))
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(file_card(fi, true, false, is_viewed(fi), id, t, cx))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .gap(px(8.))
                                .child(cbtn("df-prev", false, t).child(icon("left", 13., t.text)).child("Previous file").on_click(cx.listener(nav(-1))))
                                .child(cbtn("df-edit", false, t).child(icon("code", 13., t.text)).child("Open in editor").on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_file_in_editor(CHANGES[fi].path, cx)
                                })))
                                .child(cbtn("df-next", false, t).child("Next file").child(icon("right", 13., t.text)).on_click(cx.listener(nav(1)))),
                        ),
                )
                .into_any_element();
        }

        let adds: u32 = CHANGES.iter().map(|f| f.add).sum();
        let dels: u32 = CHANGES.iter().map(|f| f.del).sum();
        let num = |s: String, c: Hsla| div().flex_none().font_family(MONO).text_size(px(11.5)).text_color(c).child(s);

        let files = div()
            .id(("df-files", id))
            .w(px(260.))
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(t.border)
            .px(px(8.))
            .py(px(10.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .pt(px(4.))
                    .pb(px(10.))
                    .text_size(px(12.5))
                    .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(format!("{} files", CHANGES.len())))
                    .child(num(format!("+{adds}"), t.green))
                    .child(num(format!("−{dels}"), t.red)),
            )
            .children(CHANGES.iter().enumerate().map(|(i, f)| {
                let sc = scroll.clone();
                div()
                    .id(("df-f", i))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(8.))
                    .py(px(7.))
                    .rounded(px(7.))
                    .text_size(px(12.5))
                    .hover(|s| s.bg(t.surface_hover))
                    .when(is_viewed(i), |d| d.opacity(0.5))
                    .child(st_badge(f.st, t).mr(px(2.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().child(f.name()))
                            .child(div().truncate().text_size(px(11.)).text_color(t.text_3).child(f.dir())),
                    )
                    .child(num(format!("+{}", f.add), t.green))
                    .child(num(format!("−{}", f.del), t.red))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        sc.scroll_to_top_of_item(i);
                        cx.notify();
                    }))
            }))
            .child(div().flex_1().min_h(px(8.)))
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .pt(px(10.))
                    .px(px(4.))
                    .pb(px(2.))
                    .child(cbtn("df-comment", false, t).flex_1().justify_center().child("Comment"))
                    .child(cbtn("df-approve", true, t).flex_1().justify_center().child("Approve")),
            );

        let cards: Vec<AnyElement> =
            (0..CHANGES.len()).map(|i| file_card(i, false, folded.contains(&i), is_viewed(i), id, t, cx).into_any_element()).collect();
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .child(files)
            .child(
                div()
                    .id(("df-body", id))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .px(px(18.))
                    .pt(px(14.))
                    .pb(px(40.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .children(cards),
            )
            .into_any_element()
    }
}

fn file_card(i: usize, single: bool, folded: bool, viewed: bool, tab: usize, t: Theme, cx: &mut Context<TakoApp>) -> impl IntoElement {
    let f = &CHANGES[i];
    let num = |s: String, c: Hsla| div().flex_none().font_family(MONO).text_size(px(11.5)).text_color(c).child(s);
    let head = div()
        .id(("df-head", i))
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .py(px(8.))
        .font_family(MONO)
        .text_size(px(12.))
        .bg(t.surface)
        .when(!single, |d| {
            d.child(
                svg()
                    .path("icons/chev-d.svg")
                    .size(px(12.))
                    .flex_none()
                    .text_color(t.text)
                    .with_transformation(Transformation::rotate(radians(if folded { -std::f32::consts::FRAC_PI_2 } else { 0. }))),
            )
            .on_click(cx.listener(move |_, _, _, cx| {
                st(|s| {
                    let d = s.diff.entry(tab).or_default();
                    if !d.folded.remove(&i) {
                        d.folded.insert(i);
                    }
                });
                cx.notify();
            }))
        })
        .child(st_badge(f.st, t))
        .child(div().flex_1().min_w_0().truncate().child(f.path))
        .child(num(format!("+{}", f.add), t.green))
        .child(num(format!("−{}", f.del), t.red))
        .child(
            div()
                .id(("df-v", i))
                .flex()
                .items_center()
                .gap(px(5.))
                .ml(px(8.))
                .font_family(FONT)
                .text_size(px(11.5))
                .text_color(t.text_3)
                .child(checkbox(viewed, t))
                .child("Viewed")
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.diff_toggle_viewed(tab, i);
                    cx.notify();
                })),
        );

    let mut rows: Vec<AnyElement> = vec![];
    if !folded {
        for h in f.hunks {
            rows.push(div().px(px(12.)).py(px(4.)).font_family(MONO).text_size(px(11.5)).text_color(t.text_3).bg(tr(t.blue, 0.07)).whitespace_nowrap().overflow_hidden().text_ellipsis().child(h.h).into_any_element());
            let (mut o, mut n) = (h.from, h.to);
            for (k, l) in h.lines {
                let on = if *k == '+' { String::new() } else { o += 1; (o - 1).to_string() };
                let nn = if *k == '-' { String::new() } else { n += 1; (n - 1).to_string() };
                let (bg, sg, sgc) = match k {
                    '+' => (Some(tr(t.green, 0.11)), "+", t.green),
                    '-' => (Some(tr(t.red, 0.11)), "−", t.red),
                    _ => (None, "", t.text_3),
                };
                let ln = |s: String| div().w(px(40.)).flex_none().text_right().pr(px(8.)).text_color(t.text_3).opacity(0.6).child(s);
                rows.push(
                    div()
                        .flex()
                        .font_family(MONO)
                        .text_size(px(12.))
                        .line_height(relative(1.75))
                        .when_some(bg, |d, bg| d.bg(bg))
                        .child(ln(on))
                        .child(ln(nn))
                        .child(div().w(px(18.)).flex_none().text_color(sgc).child(sg))
                        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(hl(l, &t, t.text)))
                        .into_any_element(),
                );
            }
        }
    }
    div().flex_none().border_1().border_color(t.border).rounded(px(10.)).overflow_hidden().child(head).children(rows)
}
