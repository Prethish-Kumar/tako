//! Mock data — mirrors the MOCK DATA section of the HTML mock.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrState {
    Open,
    Merged,
    Draft,
    Closed,
}

#[derive(Clone, Debug)]
pub struct Thread {
    pub id: usize,
    pub title: String,
    pub repo: String,
    pub pr: u32,
    pub pr_state: PrState,
    pub branch: String,
    pub running: bool,
    pub unread: bool,
    /// minutes ago (for the "5m / 1h / 2d" stamp)
    pub ago_min: u32,
}

impl Thread {
    pub fn wt(&self) -> &str {
        self.branch.rsplit('/').next().unwrap_or(&self.branch)
    }
}

pub fn ago(min: u32) -> String {
    if min < 60 {
        format!("{}m", min.max(1))
    } else if min < 60 * 24 {
        format!("{}h", min / 60)
    } else {
        format!("{}d", min / (60 * 24))
    }
}

fn t(id: usize, title: &str, repo: &str, pr: u32, st: PrState, branch: &str, ago_min: u32) -> Thread {
    Thread { id, title: title.into(), repo: repo.into(), pr, pr_state: st, branch: branch.into(), running: false, unread: false, ago_min }
}

pub fn threads() -> Vec<Thread> {
    use PrState::*;
    let mut v = vec![
        t(1, "Background agents panel", "tako", 2150, Open, "tako/background-agent-panel", 0),
        t(2, "Duplicate message on resume", "tako", 2149, Open, "tako/fix-duplicate-msg", 5),
        t(4, "Skill recognition failure", "tako", 2142, Open, "tako/debug-skill-recog", 53),
        t(7, "Inline images in messages", "tako", 2139, Open, "tako/inline-images", 60),
        t(3, "Multi-project worktree label", "tako", 2131, Merged, "tako/fix-multi-proj", 180),
        t(5, "Breadcrumb header w/o tabs", "tako", 2141, Draft, "tako/breadcrumb-header", 2 * 1440),
        t(9, "PR attachment logic", "tako", 2132, Open, "tako/pr-attachments", 5 * 1440),
        t(8, "Auto-reconnect remote host", "tako", 2133, Merged, "tako/auto-reconnect", 6 * 1440),
        t(6, "Retry on transient 403", "api", 2138, Merged, "api/retry-transient", 10 * 60),
        t(10, "Docs search ranking", "web", 91, Open, "web/docs-search", 13),
        t(11, "Landing page hero", "web", 88, Closed, "web/hero-refresh", 8 * 1440),
    ];
    v[0].running = true;
    v[1].unread = true;
    v
}

#[derive(Clone, Debug)]
pub struct Project {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub label: Option<&'static str>,
}

pub fn projects() -> Vec<Project> {
    vec![
        Project { id: "tako", name: "tako", icon: "octo", label: Some("work") },
        Project { id: "api", name: "api", icon: "robot", label: Some("work") },
        Project { id: "web", name: "web", icon: "planet", label: Some("oss") },
    ]
}

pub struct Label {
    pub id: &'static str,
    pub name: &'static str,
    pub color: u32,
}
pub const LABELS: &[Label] = &[
    Label { id: "work", name: "Work", color: 0x7aa7e8 },
    Label { id: "oss", name: "Open source", color: 0x5fc48a },
    Label { id: "personal", name: "Personal", color: 0xe2b75a },
];

/// Transcript blocks. `body` uses a tiny markup: lines starting with "$ " are
/// commands, "✓" ok, "✗" errors; `text` supports `code` spans with backticks.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    User { text: String, steer: bool },
    Text(String),
    Tool { label: String, body: String, running: bool, icon: &'static str },
    Thinking,
}

pub fn transcript(th: &Thread) -> Vec<Block> {
    use Block::*;
    let tool = |l: &str, b: &str| Tool { label: l.into(), body: b.into(), running: false, icon: "term" };
    if th.id == 1 {
        return vec![
            User { text: "One more thing — the divider under the resumed header shouldn't render. Dividers are only for “worked for” summaries.".into(), steer: true },
            Text("`fmt:check` broke after the rebase. Fixing that along with removing the divider under the “Resumed” headers. Checking formatting first.".into()),
            tool("Ran 6 commands", "$ bun run fmt:check\n✗ src/components/chat/Timeline.tsx\n$ bunx fmt --write src/components/chat/Timeline.tsx\n✓ formatted 1 file"),
            Text("Adding a check that resumed headers have no divider, reusing the existing collapse test:".into()),
            tool("Ran 3 commands", "$ bun test timeline.collapse\n✓ 14 passed (212ms)"),
            Text("Passes. Confirming the new assertion fails without the fix:".into()),
            tool("Ran cp src/components/chat/Timeline.tsx /tmp/backup.tsx && git stash …", "✗ resumed header renders divider\n✗   expected 0, received 1\n$ git stash pop"),
            Text("Fails without the fix, passes with it. Running full checks, then splitting commits: rebase formatting into the main commit, divider fix into its own.".into()),
            tool("Bash", "$ bun run check\n✓ unit 6153 passed\n✓ typecheck\n✓ lint"),
            Text("Unit tests (6153), typecheck, and lint pass. Force-pushing the rebased branch with `--force-with-lease` and updating the PR description:".into()),
            Tool { label: "Running git push -q --force-with-lease origin HEAD".into(), body: "$ git push -q --force-with-lease origin HEAD\n…".into(), running: true, icon: "term" },
            Thinking,
        ];
    }
    vec![
        User { text: format!("Can you take a look at “{}”?", th.title), steer: false },
        Text("Sure — exploring the relevant code first.".into()),
        tool("Read 4 files", "src/app.tsx\nsrc/lib/session.ts\nsrc/components/Sidebar.tsx\nsrc/hooks/useThread.ts"),
        Text(format!("Found the cause. Opened `#{}` on `{}` with the fix and a regression test.", th.pr, th.branch)),
    ]
}

pub fn agent_steps() -> Vec<Block> {
    vec![
        Block::Text("On it. Looking at the relevant files first.".into()),
        Block::Tool { label: "Read 3 files".into(), body: "src/components/Panel.tsx\nsrc/state/agents.ts\nsrc/styles/tokens.css".into(), running: false, icon: "term" },
        Block::Tool { label: "Edited src/components/Panel.tsx".into(), body: "✓ + 18   ✗ − 4".into(), running: false, icon: "term" },
        Block::Text("Done. The change is in place and the existing tests still pass.".into()),
    ]
}

#[derive(Clone, Debug)]
pub struct Change {
    pub path: &'static str,
    pub add: u32,
    pub del: u32,
    pub st: &'static str, // M | A | D
}

pub fn changes() -> Vec<Change> {
    vec![
        Change { path: "src/components/chat/Timeline.tsx", add: 9, del: 3, st: "M" },
        Change { path: "src/components/chat/timeline.collapse.test.ts", add: 5, del: 0, st: "A" },
        Change { path: "src/styles/timeline.css", add: 1, del: 4, st: "M" },
        Change { path: "src/components/chat/Divider.tsx", add: 2, del: 1, st: "M" },
        Change { path: "src/components/chat/LegacyDivider.tsx", add: 0, del: 4, st: "D" },
        Change { path: "CHANGELOG.md", add: 3, del: 0, st: "M" },
    ]
}
