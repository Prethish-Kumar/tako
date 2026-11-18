# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

## Automations
- `<select>`s are `.au-sel` chips that open a custom menu (styled like `.au-menu`)
  instead of the native macOS popup; the `<input type=time>` is a chip with a
  half-hour menu (no free typing of minutes).
- Labels are a local copy of the mock's `forgeLabels` defaults (incl. Client,
  Maintenance); new labels aren't shared with the sidebar yet (`data::LABELS` is const).
- Nothing is persisted (the mock uses localStorage); the Advanced chevron rotates
  without the .2s transition.

## Plugins
- `.pl-fcard:hover` lifts by 2px instantly (no transform transition); the dot
  pattern is painted as small rounded quads, not a radial-gradient.
- Installed plugins aren't persisted between launches.
