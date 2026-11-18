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
