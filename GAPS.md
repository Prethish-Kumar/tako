# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

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
