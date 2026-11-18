# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.
- CSS collapses adjacent vertical margins; GPUI doesn't. The feed computes the
  gap between blocks itself (max of the neighbours' margins) to match.
- Inline `code` can't be a padded, smaller-font span inside one text run (GPUI
  runs can't change size/padding), so paragraphs with code are laid out as
  wrapped word items with real chips. Long code spans move to the next line
  as a whole instead of breaking mid-word like the browser does.
- GPUI's default line-height is 1.618; the browser's `normal` for Inter is
  ~1.21. The app root sets 1.21 so un-styled text matches.

## Window chrome
- The mock draws fake traffic lights; the GPUI window uses the real macOS ones
  (transparent titlebar, lights at 16,16).

## Snapshots
- Hidden windows never redraw on their own, so the snapshot mode forces
  `window.draw()` twice before `render_to_image()` (the second pass lets
  measured heights settle).

## Home
- The heatmap card's frosted glass (`backdrop-filter: blur(18px) saturate(1.3)`) can't be done;
  it is a flat `panel-solid` at 74% alpha.
- `rise` entrance: opacity + 16px lift only, the `blur(8px)` → 0 part is dropped.
- The light sweep over the grid uses `mix-blend-mode: overlay` in the mock; here it is a plain
  7% white gradient band (painted as two 2-stop gradients, since GPUI gradients have 2 stops).
- Cell tooltips: the mock uses the native `title` tooltip; here a small custom tip is drawn
  above the hovered cell. "Today" is UTC days, not local midnight.
- Placeholder "Ask anything, @ to mention…" is owned by chat.rs (shared composer input).

## Right panel
- No letter-spacing in GPUI: the uppercase section headers (`.06em`) are set without tracking.
- Underline offset (`text-underline-offset: 3px`) isn't configurable; origin/main's underline sits tighter.
- "Discard all" skips the mock's `confirm()` dialog.
- The 2px badge ring (`box-shadow: 0 0 0 2px var(--bg)`) is a padded bg-coloured wrapper.

## Settings / theme picker / usage
- Popovers and menus are solid (`--panel-solid`), not blurred. That matches the mock's
  always-on "flat" style, so nothing is lost.
- Conic-gradient orbs (Auto) and hard-stop preset swatches are drawn on the CPU into
  cached `RenderImage`s, because GPUI only has two-stop linear gradients. The Settings
  "Auto" orb follows the live palette, so a theme transition draws about 50 cached frames.
- Toggles (`.tgl`) jump straight to the new state instead of sliding over 0.2s.
  Popovers fade in and slide 6px but don't scale from .98.
- The theme popover anchors to `ThemePopState::anchor` (the palette button's window bounds).
  The titlebar doesn't record those yet, so it falls back to the empty-layout spot
  (top 44, right 70). In Chats layout the button sits elsewhere.
- The `<select>` controls are a custom anchored menu (checkmark on the current value),
  not the native OS popup.
- Model / Effort / Access defaults and "Open on startup" are stored in `SettingsState`.
  They aren't wired to the composer context or to app start yet.
- Labels are a settings-local list seeded from `data::LABELS` plus the mock's Client /
  Maintenance. The sidebar still reads `data::LABELS`.
- "Reset this mockup" resets settings, usage and theme to their seeds instead of reloading.
