# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

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
