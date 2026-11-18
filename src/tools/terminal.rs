//! Terminal tab: a small fake shell with history (the mock's `VIEWS.terminal`).

use super::{hex, mix, st};
use crate::{app::TakoApp, chat::Tab, theme::{ActiveTheme, Theme}, ui::*};
use gpui_kit::{
    component::input::{Input, InputEvent, InputState},
    prelude::FluentBuilder as _,
    *,
};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub enum Tc {
    Plain,
    Dim,
    Ok,
    Err,
    Y,
    D,
    Ps,
    Br,
    Pr,
}

#[derive(Clone)]
pub struct TLine {
    /// (text, colour class, bold)
    pub segs: Vec<(String, Tc, bool)>,
    pub echo: bool,
}

pub struct TermState {
    pub lines: Vec<TLine>,
    pub input: Entity<InputState>,
    pub hist: Vec<String>,
    pub hi: usize,
    pub busy: bool,
    pub scroll: ScrollHandle,
}

/// Parse the mini markup used for canned output: `{d:src/}`, `{b:help}`, … Text
/// outside braces gets `base`.
fn parse(s: &str, base: Tc) -> Vec<(String, Tc, bool)> {
    let mut out = vec![];
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        if i > 0 {
            out.push((rest[..i].to_string(), base, false));
        }
        let Some(j) = rest[i..].find('}') else { break };
        let inner = &rest[i + 1..i + j];
        let (k, txt) = inner.split_once(':').unwrap_or(("", inner));
        let (c, b) = match k {
            "dim" => (Tc::Dim, false),
            "ok" => (Tc::Ok, false),
            "err" => (Tc::Err, false),
            "y" => (Tc::Y, false),
            "d" => (Tc::D, false),
            "b" => (base, true),
            _ => (base, false),
        };
        out.push((txt.to_string(), c, b));
        rest = &rest[i + j + 1..];
    }
    if !rest.is_empty() {
        out.push((rest.to_string(), base, false));
    }
    out
}

fn line(s: &str, base: Tc) -> TLine {
    TLine { segs: parse(s, base), echo: false }
}

impl TakoApp {
    fn prompt(&self) -> Vec<(String, Tc, bool)> {
        let th = self.thread();
        vec![
            (format!("~/{}", th.repo), Tc::Ps, false),
            (" ".into(), Tc::Plain, false),
            (th.wt().to_string(), Tc::Br, false),
            (" ".into(), Tc::Plain, false),
            ("❯".into(), Tc::Pr, false),
        ]
    }

    pub(super) fn ensure_term(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        if st(|s| s.term.contains_key(&id)) {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe_in(&input, window, move |this: &mut TakoApp, inp, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                if st(|s| s.term.get(&id).is_some_and(|t| t.busy)) {
                    return;
                }
                let c = inp.read(cx).value().trim().to_string();
                inp.update(cx, |s, cx| s.set_value("", window, cx));
                this.term_run(id, c, cx);
            }
        })
        .detach();
        let branch = self.thread().branch.clone();
        let first = line(&format!("Last login on ttys004 · worktree {{b:{branch}}} · type {{b:help}}"), Tc::Dim);
        input.update(cx, |s, cx| s.focus(window, cx));
        st(|s| s.term.insert(id, TermState { lines: vec![first], input, hist: vec![], hi: 0, busy: false, scroll: ScrollHandle::new() }));
    }

    fn term_print(id: usize, l: TLine) {
        st(|s| {
            if let Some(t) = s.term.get_mut(&id) {
                t.lines.push(l);
                t.scroll.scroll_to_bottom();
            }
        });
    }

    pub(super) fn term_run(&mut self, id: usize, cmd: String, cx: &mut Context<Self>) {
        let th = self.thread().clone();
        let mut echo = self.prompt();
        echo.push((format!(" {cmd}"), Tc::Plain, false));
        Self::term_print(id, TLine { segs: echo, echo: true });
        st(|s| {
            if let Some(t) = s.term.get_mut(&id) {
                if !cmd.is_empty() {
                    t.hist.push(cmd.clone());
                }
                t.hi = t.hist.len();
            }
        });
        if cmd.is_empty() {
            return;
        }
        if cmd == "clear" {
            st(|s| s.term.get_mut(&id).map(|t| t.lines.clear()));
            return;
        }
        if let Some(e) = cmd.strip_prefix("echo ") {
            return Self::term_print(id, TLine { segs: vec![(e.to_string(), Tc::Plain, false)], echo: false });
        }
        let br = &th.branch;
        let out: Option<String> = match cmd.as_str() {
            "help" => Some("commands: {b:ls} · {b:git status} · {b:git log} · {b:bun test} · {b:pwd} · {b:whoami} · {b:echo} · {b:clear}".into()),
            "ls" => Some("{d:src/}  {d:tests/}  {d:node_modules/}  package.json  bun.lockb  PLAN.md  README.md  tsconfig.json".into()),
            "pwd" => Some(format!("/Users/you/.tako/worktrees/{br}")),
            "whoami" => Some("you (with 1 agent in this worktree)".into()),
            "git status" => Some(format!(
                "On branch {br}\nChanges not staged for commit:\n  {{err:modified:   src/components/chat/Timeline.tsx}}\n  {{err:modified:   src/styles/timeline.css}}\nUntracked files:\n  {{err:src/components/chat/timeline.collapse.test.ts}}"
            )),
            "git log" => Some("{y:a91f3c2} Only render dividers for worked-for summaries\n{y:7d02e18} Format Timeline after rebase\n{y:c44b0a9} Add resumed header to timeline".into()),
            _ => None,
        };
        if let Some(o) = out {
            for l in o.lines() {
                Self::term_print(id, line(l, Tc::Plain));
            }
            return;
        }
        let is_test = ["bun ", "npm ", "pnpm "].iter().any(|p| cmd.starts_with(p)) && (cmd.ends_with(" test") || cmd.contains(" test") || cmd.contains("run test"));
        if is_test {
            st(|s| s.term.get_mut(&id).map(|t| t.busy = true));
            let lines = [
                "bun test v1.2.4",
                "",
                "{ok:✓} timeline collapse › collapses consecutive tool calls {dim:[3.1ms]}",
                "{ok:✓} timeline collapse › does not render a divider under resumed headers {dim:[1.8ms]}",
                "{ok:✓} session › resumes from snapshot {dim:[12ms]}",
                "",
                "{ok: 6153 pass}",
                " 0 fail",
                "{dim:Ran 6153 tests across 412 files. [4.21s]}",
            ];
            cx.spawn(async move |this, cx| {
                for (i, l) in lines.iter().enumerate() {
                    // the mock waits 140–300ms per line
                    let ms = 140 + ((i * 97) % 160) as u64;
                    cx.background_executor().timer(Duration::from_millis(ms)).await;
                    let l = if l.is_empty() { "\u{a0}" } else { l };
                    Self::term_print(id, line(l, Tc::Plain));
                    this.update(cx, |_, cx| cx.notify()).ok();
                }
                st(|s| s.term.get_mut(&id).map(|t| t.busy = false));
            })
            .detach();
            return;
        }
        let w = cmd.split(' ').next().unwrap_or("");
        Self::term_print(id, TLine { segs: vec![(format!("zsh: command not found: {w}"), Tc::Err, false)], echo: false });
    }

    pub(super) fn render_terminal(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let id = tab.id;
        self.ensure_term(id, window, cx);
        let (lines, input, scroll) = st(|s| {
            let ts = &s.term[&id];
            (ts.lines.clone(), ts.input.clone(), ts.scroll.clone())
        });
        let (bg, fg) = if t.light { (hex(0x1c1a1d), hex(0xe9e6e2)) } else { (mix(hsla(0., 0., 0., 1.), t.bg, 0.35), t.text) };
        let color = |c: Tc| match c {
            Tc::Plain => fg,
            Tc::Dim => hex(0x8a847f),
            Tc::Ok | Tc::Pr => t.green,
            Tc::Err => t.red,
            Tc::Y => t.yellow,
            Tc::D | Tc::Ps => t.blue,
            Tc::Br => t.accent,
        };
        let render_line = |l: &TLine| -> StyledText {
            let mut s = String::new();
            let runs = l
                .segs
                .iter()
                .map(|(txt, c, b)| {
                    s.push_str(txt);
                    let mut f = font(MONO);
                    if *b {
                        f.weight = FontWeight::SEMIBOLD;
                    }
                    TextRun { len: txt.len(), font: f, color: color(*c), background_color: None, underline: None, strikethrough: None }
                })
                .collect::<Vec<_>>();
            if s.is_empty() {
                return StyledText::new("\u{a0}");
            }
            StyledText::new(s).with_runs(runs)
        };
        let prompt = render_line(&TLine { segs: self.prompt(), echo: false });
        let inp = input.clone();
        let inp2 = input.clone();
        div()
            .id(("term", id))
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(bg)
            .text_color(fg)
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .px(px(18.))
            .py(px(14.))
            .font_family(MONO)
            .text_size(px(12.5))
            .line_height(relative(1.65))
            .cursor_text()
            .on_mouse_up(MouseButton::Left, move |_, window, cx| inp.update(cx, |s, cx| s.focus(window, cx)))
            .capture_key_down(cx.listener(move |_, ev: &KeyDownEvent, window, cx| {
                let k = ev.keystroke.key.as_str();
                let m = &ev.keystroke.modifiers;
                if k == "k" && (m.platform || m.control) {
                    st(|s| s.term.get_mut(&id).map(|t| t.lines.clear()));
                    cx.stop_propagation();
                    cx.notify();
                } else if k == "up" || k == "down" {
                    let v = st(|s| {
                        let t = s.term.get_mut(&id)?;
                        t.hi = if k == "up" { t.hi.saturating_sub(1) } else { (t.hi + 1).min(t.hist.len()) };
                        Some(t.hist.get(t.hi).cloned().unwrap_or_default())
                    });
                    if let Some(v) = v {
                        inp2.update(cx, |s, cx| s.set_value(v, window, cx));
                    }
                    cx.stop_propagation();
                }
            }))
            .children(lines.iter().map(|l| div().when(l.echo, |d| d.mt(px(6.))).child(render_line(l))))
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(div().flex_none().child(prompt))
                    .child(div().flex_1().ml(px(8.)).child(term_input(&input, fg, t))),
            )
    }
}

fn term_input(input: &Entity<InputState>, fg: Hsla, _t: Theme) -> impl IntoElement {
    div().text_color(fg).font_family(MONO).text_size(px(12.5)).child(Input::new(input).appearance(false).bordered(false).p_0().h(px(20.)))
}
