//! Mock content for the tool tabs (files, the worktree diff) and the tiny
//! syntax highlighter the mock uses (`hl()` in ade.html).

use crate::{theme::Theme, ui::MONO};
use gpui_kit::*;

pub const TIMELINE: &str = r#"import { useMemo } from "react";
import { groupItems, type Item } from "../../lib/timeline";
import { Divider } from "./Divider";

type Props = { items: Item[]; resumed?: boolean };

export function Timeline({ items, resumed }: Props) {
  const groups = useMemo(() => groupItems(items), [items]);

  return groups.map((g, i) => {
    // Dividers are only for "worked for" summaries
    const showDivider = g.kind === "worked-for";
    return (
      <section key={g.id} data-kind={g.kind}>
        {resumed && i === 0 && <ResumedHeader at={g.startedAt} />}
        {g.items.map(item => <Row key={item.id} item={item} />)}
        {showDivider && <Divider label={g.label} />}
      </section>
    );
  });
}"#;

pub const TEST: &str = r#"import { describe, it, expect } from "bun:test";
import { renderTimeline } from "./test-utils";

describe("timeline collapse", () => {
  it("collapses consecutive tool calls", () => {
    const t = renderTimeline(fixtures.tools(6));
    expect(t.groups).toHaveLength(1);
  });

  it("does not render a divider under resumed headers", () => {
    const t = renderTimeline(fixtures.resumed(), { resumed: true });
    expect(t.query("hr")).toHaveLength(0);
  });
});"#;

pub const LIB: &str = r#"export type Item = { id: string; kind: string; startedAt: number };

export function groupItems(items: Item[]) {
  const out = [];
  for (const item of items) {
    const last = out.at(-1);
    if (last && last.kind === item.kind) last.items.push(item);
    else out.push({ id: item.id, kind: item.kind, items: [item] });
  }
  return out;
}"#;

pub fn file_source(path: &str) -> String {
    match path {
        "src/components/chat/Timeline.tsx" => TIMELINE.into(),
        "src/components/chat/timeline.collapse.test.ts" => TEST.into(),
        "src/lib/timeline.ts" => LIB.into(),
        _ => format!("// {path}\n// (empty in this mock)\n"),
    }
}

pub struct Hunk {
    pub h: &'static str,
    pub from: u32,
    pub to: u32,
    pub lines: &'static [(char, &'static str)],
}
pub struct Change {
    pub path: &'static str,
    pub st: char,
    pub add: u32,
    pub del: u32,
    pub hunks: &'static [Hunk],
}
impl Change {
    pub fn name(&self) -> &'static str {
        self.path.rsplit('/').next().unwrap_or(self.path)
    }
    pub fn dir(&self) -> &'static str {
        self.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
    }
}

pub static CHANGES: &[Change] = &[
    Change {
        path: "src/components/chat/Timeline.tsx",
        st: 'M',
        add: 9,
        del: 3,
        hunks: &[Hunk {
            h: "@@ -8,14 +8,20 @@ export function Timeline({ items, resumed }: Props) {",
            from: 8,
            to: 8,
            lines: &[
                (' ', "  const groups = useMemo(() => groupItems(items), [items]);"),
                (' ', ""),
                ('-', "  return groups.map((g, i) => ("),
                ('+', "  return groups.map((g, i) => {"),
                ('+', "    // Dividers are only for \"worked for\" summaries"),
                ('+', "    const showDivider = g.kind === \"worked-for\";"),
                ('+', "    return ("),
                (' ', "      <section key={g.id} data-kind={g.kind}>"),
                ('-', "        {i === 0 && <ResumedHeader at={g.startedAt} />}"),
                ('+', "        {resumed && i === 0 && <ResumedHeader at={g.startedAt} />}"),
                (' ', "        {g.items.map(item => <Row key={item.id} item={item} />)}"),
                ('-', "        <Divider label={g.label} />"),
                ('+', "        {showDivider && <Divider label={g.label} />}"),
                (' ', "      </section>"),
                ('+', "    );"),
                ('+', "  });"),
            ],
        }],
    },
    Change {
        path: "src/components/chat/timeline.collapse.test.ts",
        st: 'A',
        add: 5,
        del: 0,
        hunks: &[Hunk {
            h: "@@ -9,3 +9,8 @@ describe(\"timeline collapse\", () => {",
            from: 9,
            to: 9,
            lines: &[
                (' ', "  });"),
                ('+', ""),
                ('+', "  it(\"does not render a divider under resumed headers\", () => {"),
                ('+', "    const t = renderTimeline(fixtures.resumed(), { resumed: true });"),
                ('+', "    expect(t.query(\"hr\")).toHaveLength(0);"),
                ('+', "  });"),
                (' ', "});"),
            ],
        }],
    },
    Change {
        path: "src/styles/timeline.css",
        st: 'M',
        add: 1,
        del: 4,
        hunks: &[Hunk {
            h: "@@ -31,7 +31,4 @@ .timeline section {",
            from: 31,
            to: 31,
            lines: &[
                (' ', ".timeline hr {"),
                ('-', "  margin: 18px 0;"),
                ('-', "  border-top: 1px solid var(--border);"),
                ('-', "  opacity: .6;"),
                ('-', "}"),
                ('+', "  margin: 18px 0; border-top: 1px solid var(--border); }"),
                (' ', ""),
            ],
        }],
    },
    Change {
        path: "src/components/chat/Divider.tsx",
        st: 'M',
        add: 2,
        del: 1,
        hunks: &[Hunk {
            h: "@@ -4,6 +4,7 @@ export function Divider({ label }: { label?: string }) {",
            from: 4,
            to: 4,
            lines: &[
                (' ', "  return ("),
                ('-', "    <hr className=\"divider\" />"),
                ('+', "    <hr className=\"divider\" aria-label={label} />"),
                ('+', "    // only rendered for worked-for groups"),
                (' ', "  );"),
            ],
        }],
    },
    Change {
        path: "src/components/chat/LegacyDivider.tsx",
        st: 'D',
        add: 0,
        del: 4,
        hunks: &[Hunk {
            h: "@@ -1,4 +0,0 @@",
            from: 1,
            to: 0,
            lines: &[
                ('-', "import { Divider } from \"./Divider\";"),
                ('-', ""),
                ('-', "/** @deprecated use Divider */"),
                ('-', "export const LegacyDivider = Divider;"),
            ],
        }],
    },
    Change {
        path: "CHANGELOG.md",
        st: 'A',
        add: 3,
        del: 0,
        hunks: &[Hunk {
            h: "@@ -0,0 +1,3 @@",
            from: 0,
            to: 1,
            lines: &[('+', "## Unreleased"), ('+', ""), ('+', "- Resumed headers no longer render a divider (#2150)")],
        }],
    },
];

const KEYWORDS: &[&str] = &[
    "import", "from", "export", "default", "function", "return", "const", "let", "if", "else", "for", "of", "new", "type", "interface", "extends", "true",
    "false", "null", "undefined", "await", "async", "describe", "it", "expect",
];

#[derive(Clone, Copy, PartialEq)]
enum Tok {
    Plain,
    Kw,
    Str,
    Comment,
    Num,
    Tag,
}

/// Port of the mock's `hl()` regex: comments, strings, keywords, numbers and
/// `<Component` tags. Returns mono-font text runs over `code`.
pub fn hl(code: &str, t: &Theme, base: Hsla) -> StyledText {
    let b = code.as_bytes();
    let mut toks: Vec<(usize, Tok)> = vec![]; // (end, kind)
    let mut i = 0;
    let push = |toks: &mut Vec<(usize, Tok)>, end: usize, k: Tok| {
        if let Some(last) = toks.last_mut() {
            if last.1 == k {
                last.0 = end;
                return;
            }
        }
        toks.push((end, k));
    };
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    while i < b.len() {
        let c = b[i];
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = code[i..].find('\n').map(|n| i + n).unwrap_or(b.len());
            push(&mut toks, end, Tok::Comment);
            i = end;
        } else if c == b'"' || c == b'\'' || c == b'`' {
            let mut j = i + 1;
            let mut found = false;
            while j < b.len() {
                if b[j] == b'\\' && c != b'`' {
                    j += 2;
                    continue;
                }
                if b[j] == b'\n' && c != b'`' {
                    break;
                }
                if b[j] == c {
                    found = true;
                    break;
                }
                j += 1;
            }
            if found {
                push(&mut toks, j + 1, Tok::Str);
                i = j + 1;
            } else {
                push(&mut toks, i + 1, Tok::Plain);
                i += 1;
            }
        } else if c == b'<' && {
            let k = if b.get(i + 1) == Some(&b'/') { i + 2 } else { i + 1 };
            b.get(k).is_some_and(|x| x.is_ascii_uppercase())
        } {
            let mut j = if b.get(i + 1) == Some(&b'/') { i + 2 } else { i + 1 };
            while j < b.len() && (word(b[j]) || b[j] == b'.') {
                j += 1;
            }
            push(&mut toks, j, Tok::Tag);
            i = j;
        } else if word(c) {
            let mut j = i;
            while j < b.len() && word(b[j]) {
                j += 1;
            }
            let w = &code[i..j];
            let k = if w.bytes().all(|x| x.is_ascii_digit()) {
                Tok::Num
            } else if KEYWORDS.contains(&w) {
                Tok::Kw
            } else {
                Tok::Plain
            };
            push(&mut toks, j, k);
            i = j;
        } else {
            // advance one UTF-8 char
            let n = code[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            push(&mut toks, i + n, Tok::Plain);
            i += n;
        }
    }
    let s_col: Hsla = if t.light { rgb(0x2f7a3a).into() } else { rgb(0x9fd49a).into() };
    let n_col: Hsla = if t.light { rgb(0xa8740a).into() } else { t.yellow };
    let mut start = 0;
    let runs = toks
        .into_iter()
        .map(|(end, k)| {
            let mut f = font(MONO);
            if k == Tok::Comment {
                f.style = FontStyle::Italic;
            }
            let color = match k {
                Tok::Plain => base,
                Tok::Kw => t.accent,
                Tok::Str => s_col,
                Tok::Comment => t.text_3,
                Tok::Num => n_col,
                Tok::Tag => t.blue,
            };
            let r = TextRun { len: end - start, font: f, color, background_color: None, underline: None, strikethrough: None };
            start = end;
            r
        })
        .collect::<Vec<_>>();
    if code.is_empty() {
        return StyledText::new(" ");
    }
    StyledText::new(code.to_string()).with_runs(runs)
}
