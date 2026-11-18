# Tako — GPUI port of the Tako ADE mock

A native Rust recreation of the Tako desktop mock (`ref/ade.html`) built with
[GPUI](https://www.gpui.rs) via `gpui-kit` 0.10 (GPUI `gpui-pre` 0.3.12 +
gpui-component).

```bash
cargo run            # opens the app at 1440×900
```

What's here: window chrome and rail, the sidebar (projects, labels, fold
animation, multi-select, New project), chat sessions with tabs and the smooth
feed, the composer and its context menus, the right-hand source-control panel,
session tool tabs (Changes, Browser, Terminal, Editor, Markdown, Whiteboard,
Emulator, ⌘T launcher), Home with the pixel-art backdrop scenes and PR heatmap,
Automations, Plugins, Settings, Stats, the profile/theme/usage popovers, the
signed-out screen, the theme engine (presets + Auto from the backdrop) and the
onboarding story.

- `DEV.md` — architecture, idioms and the offscreen snapshot tool.
- `GAPS.md` — everything GPUI couldn't match 1:1, and what was done instead.
- `ref/` — the HTML mock and reference screenshots.
