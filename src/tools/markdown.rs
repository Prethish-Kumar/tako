//! Markdown tab: Preview / Split / Source, live (the mock's `VIEWS.markdown`).

use super::{checkbox, data::hl, seg, st};
use crate::{app::TakoApp, chat::Tab, theme::{ActiveTheme, Theme}, ui::*};
use gpui_kit::{
    component::input::{InputEvent, Textarea, TextareaState},
    prelude::FluentBuilder as _,
    *,
};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum MdMode {
    #[default]
    Preview,
    Split,
    Source,
}
impl MdMode {
    pub fn parse(v: &str) -> Self {
        match v {
            "split" => MdMode::Split,
            "source" => MdMode::Source,
            _ => MdMode::Preview,
        }
    }
}

#[derive(Default)]
pub struct MdState {
    pub file: Option<String>,
    pub mode: MdMode,
    pub text: String,
    pub src: Option<Entity<TextareaState>>,
}

impl TakoApp {
    fn ensure_md(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) -> Entity<TextareaState> {
        if let Some(e) = st(|s| s.md.get(&id).and_then(|m| m.src.clone())) {
            return e;
        }
        let th = self.thread().clone();
        let src = format!(
            "# {title}\n\nPlan the agent is working from. Edit it and the agent picks up changes on its next step.\n\n## Goals\n- [x] Only render dividers for **worked for** summaries\n- [x] Regression test for resumed headers\n- [ ] Update the PR description with before/after screenshots\n\n## Notes\n> Resumed headers shouldn't carry a divider — it reads like the conversation ended.\n\nThe fix lives in `Timeline.tsx`:\n\n```tsx\nconst showDivider = g.kind === \"worked-for\";\n```\n\n## Done when\n1. `bun run check` passes\n2. PR #{pr} is green and ready for review\n",
            title = th.title,
            pr = th.pr
        );
        let ta = cx.new(|cx| TextareaState::new(window, cx));
        ta.update(cx, |s, cx| s.set_value(src.clone(), window, cx));
        cx.subscribe_in(&ta, window, move |_: &mut TakoApp, ta, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let v = ta.read(cx).value().to_string();
                st(|s| s.md.get_mut(&id).map(|m| m.text = v));
                cx.notify();
            }
        })
        .detach();
        st(|s| {
            let m = s.md.entry(id).or_default();
            m.text = src;
            m.src = Some(ta.clone());
        });
        ta
    }

    pub(super) fn render_markdown(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        let ta = self.ensure_md(id, window, cx);
        let (file, mode, text) = st(|s| {
            let m = &s.md[&id];
            (m.file.clone().unwrap_or_else(|| "PLAN.md".into()), m.mode, m.text.clone())
        });
        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .px(px(14.))
            .py(px(8.))
            .border_b_1()
            .border_color(t.border)
            .text_size(px(12.5))
            .text_color(t.text_2)
            .child(div().flex().items_center().gap(px(7.)).child(icon("doc", 16., t.text_2)).child(file))
            .child(seg("md-m", &[("Preview", None), ("Split", None), ("Source", None)], mode as usize, t, move |i, b| {
                b.on_click(move |_, window, _| {
                    st(|s| s.md.get_mut(&id).map(|m| m.mode = [MdMode::Preview, MdMode::Split, MdMode::Source][i]));
                    window.refresh();
                })
            }));

        let source = div()
            .id(("md-src", id))
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .px(px(32.))
            .py(px(26.))
            .font_family(MONO)
            .text_size(px(13.))
            .text_color(t.text_2)
            .line_height(relative(1.7))
            .when(mode == MdMode::Split, |d| d.max_w(relative(0.5)).border_r_1().border_color(t.border))
            .child(Textarea::new(&ta).appearance(false).bordered(false).p_0().text_size(px(13.)));

        let preview = div()
            .id(("md-out", id))
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .child(div().max_w(px(760.)).mx_auto().px(px(40.)).pt(px(26.)).pb(px(60.)).line_height(relative(1.65)).children(render_md(&text, id, t, cx)));

        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(bar)
            .child(div().flex_1().min_h_0().flex().when(mode != MdMode::Preview, |d| d.child(source)).when(mode != MdMode::Source, |d| d.child(preview)))
    }
}

/// Inline markdown: `code`, **bold**, *italic*, [link](url).
fn inline(s: &str, color: Hsla, t: Theme) -> StyledText {
    inline_s(s, color, t, false)
}

fn inline_s(s: &str, color: Hsla, t: Theme, italic: bool) -> StyledText {
    #[derive(Clone, Copy, PartialEq)]
    enum K {
        P,
        Code,
        Bold,
        It,
        Link,
    }
    let mut segs: Vec<(String, K)> = vec![];
    let mut rest = s;
    let push = |segs: &mut Vec<(String, K)>, s: &str, k: K| {
        if !s.is_empty() {
            segs.push((s.to_string(), k));
        }
    };
    loop {
        let cands = [
            rest.find('`').map(|i| (i, K::Code)),
            rest.find("**").map(|i| (i, K::Bold)),
            rest.find('*').filter(|&i| !rest[i..].starts_with("**")).map(|i| (i, K::It)),
            rest.find('[').map(|i| (i, K::Link)),
        ];
        let Some((i, k)) = cands.into_iter().flatten().min_by_key(|(i, _)| *i) else { break };
        let (open, close) = match k {
            K::Code => ("`", "`"),
            K::Bold => ("**", "**"),
            K::It => ("*", "*"),
            _ => ("[", "]("),
        };
        let after = &rest[i + open.len()..];
        let Some(j) = after.find(close).filter(|&j| j > 0) else {
            push(&mut segs, &rest[..i + open.len()], K::P);
            rest = after;
            continue;
        };
        push(&mut segs, &rest[..i], K::P);
        if k == K::Link {
            let tail = &after[j + 2..];
            let Some(e) = tail.find(')') else {
                push(&mut segs, &rest[i..], K::P);
                rest = "";
                break;
            };
            push(&mut segs, &after[..j], K::Link);
            rest = &tail[e + 1..];
        } else {
            let inner = &after[..j];
            if k == K::Code {
                // the chip's 5px side padding
                segs.push((format!("\u{2009}{inner}\u{2009}"), K::Code));
            } else {
                push(&mut segs, inner, k);
            }
            rest = &after[j + close.len()..];
        }
    }
    push(&mut segs, rest, K::P);
    let mut text = String::new();
    let runs = segs
        .iter()
        .map(|(s, k)| {
            text.push_str(s);
            let mut f = font(if *k == K::Code { MONO } else { FONT });
            match k {
                K::Bold => f.weight = FontWeight::BOLD,
                K::It => f.style = FontStyle::Italic,
                _ => {}
            }
            if italic {
                f.style = FontStyle::Italic;
            }
            TextRun {
                len: s.len(),
                font: f,
                color: if *k == K::Link { t.accent } else if *k == K::Bold { t.text } else { color },
                background_color: (*k == K::Code).then_some(t.surface_active),
                underline: (*k == K::Link).then(|| UnderlineStyle { thickness: px(1.), color: Some(t.accent), wavy: false }),
                strikethrough: None,
            }
        })
        .collect::<Vec<_>>();
    if text.is_empty() {
        return StyledText::new("");
    }
    StyledText::new(text).with_runs(runs)
}

fn render_md(src: &str, tab: usize, t: Theme, cx: &mut Context<TakoApp>) -> Vec<AnyElement> {
    let mut out: Vec<AnyElement> = vec![];
    let mut code: Option<String> = None;
    let mut task_i = 0usize;
    let mut ol_n = 0usize;
    for line in src.split('\n') {
        if line.starts_with("```") {
            if let Some(c) = code.take() {
                let c = c.trim_end_matches('\n').to_string();
                out.push(
                    div()
                        .bg(t.surface)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(9.))
                        .px(px(14.))
                        .py(px(12.))
                        .font_family(MONO)
                        .text_size(px(12.5))
                        .line_height(relative(1.6))
                        .children(c.split('\n').map(|l| div().whitespace_nowrap().child(hl(l, &t, t.text))))
                        .into_any_element(),
                );
            } else {
                code = Some(String::new());
            }
            continue;
        }
        if let Some(c) = code.as_mut() {
            c.push_str(line);
            c.push('\n');
            continue;
        }
        if !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ol_n = 0;
        }
        if let Some(h) = line.strip_prefix("# ") {
            out.push(div().font_family(SERIF).text_size(px(36.)).line_height(relative(1.65)).mb(px(10.)).child(inline(h, t.text, t)).into_any_element());
        } else if let Some(h) = line.strip_prefix("## ").or_else(|| line.strip_prefix("### ")) {
            out.push(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).mt(px(26.)).mb(px(8.)).child(inline(h, t.text, t)).into_any_element());
        } else if let Some(rest) = line.strip_prefix("- [").filter(|r| r.len() > 2 && &r[2..3] == " " && (r.starts_with("x]") || r.starts_with(" ]"))) {
            let on = rest.starts_with('x');
            let i = task_i;
            task_i += 1;
            out.push(
                div()
                    .id(("md-task", i))
                    .flex()
                    .items_center()
                    .pl(px(4.))
                    .text_color(t.text_2)
                    .child(checkbox(on, t).mr(px(6.)).ml(px(3.)))
                    .child(" ")
                    .child(inline(&rest[3..], t.text_2, t))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        // ticking a box in preview edits the source
                        let ta = st(|s| s.md.get(&tab).and_then(|m| m.src.clone()));
                        let Some(ta) = ta else { return };
                        let v = ta.read(cx).value().to_string();
                        let mut k = 0;
                        let nv = v
                            .split('\n')
                            .map(|l| {
                                if l.starts_with("- [x]") || l.starts_with("- [ ]") {
                                    k += 1;
                                    if k - 1 == i {
                                        return if l.starts_with("- [x]") { l.replacen("- [x]", "- [ ]", 1) } else { l.replacen("- [ ]", "- [x]", 1) };
                                    }
                                }
                                l.to_string()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        ta.update(cx, |s, cx| s.set_value(nv.clone(), window, cx));
                        st(|s| s.md.get_mut(&tab).map(|m| m.text = nv));
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        } else if let Some(li) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            out.push(
                div()
                    .relative()
                    .text_color(t.text_2)
                    .child(div().absolute().left(px(-14.)).top_0().child("•"))
                    .child(inline(li, t.text_2, t))
                    .into_any_element(),
            );
        } else if let Some((n, li)) = line.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            let _ = n;
            ol_n += 1;
            out.push(
                div()
                    .relative()
                    .text_color(t.text_2)
                    .child(div().absolute().left(px(-18.)).top_0().child(format!("{ol_n}.")))
                    .child(inline(li, t.text_2, t))
                    .into_any_element(),
            );
        } else if let Some(q) = line.strip_prefix("> ") {
            out.push(
                div()
                    .my(px(12.))
                    .px(px(14.))
                    .py(px(4.))
                    .border_l_2()
                    .border_color(t.accent)
                    .italic()
                    .text_color(t.text_2)
                    .child(inline_s(q, t.text_2, t, true))
                    .into_any_element(),
            );
        } else if line.trim().is_empty() {
        } else {
            out.push(div().text_color(t.text_2).child(inline(line, t.text_2, t)).into_any_element());
        }
    }
    out
}
