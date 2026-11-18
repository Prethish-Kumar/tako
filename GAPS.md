# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

## Tool tabs (src/tools/)
- Browser: GPUI has no web view, so the localhost:3000 preview (the mock's
  `srcdoc` page) is drawn natively with divs; non-localhost URLs show the mock's
  "Live pages load here in the real app" placeholder. The CSS grid
  `auto-fill minmax(220px,1fr)` is computed by hand from the stage width.
- Editor: the mock's `contenteditable` highlighted code can't be both highlighted
  and editable here. The code is shown highlighted; clicking it swaps in a plain
  mono textarea (no highlighting) until it loses focus. "Ln/Col" stays static.
- Changes: `.df-head` is not `position: sticky` (GPUI has no sticky positioning).
  Clicking a file in the list jumps (no smooth scroll). Native checkboxes are
  drawn as small squares.
- Whiteboard: the dot grid (`radial-gradient` background) is painted as 2px dots;
  strokes use lyon's default butt caps/miter joins with painted round end caps
  (GPUI doesn't re-export `LineCap`), so very sharp highlighter corners can spike.
- Emulator: the stage's `radial-gradient` glow is approximated with stacked
  translucent ellipses; the `rise` boot animation drops the blur(8px) part.
- Terminal: output is styled from a small `{class:text}` markup instead of HTML.
- ⌘T launcher: rows `rise` without the blur.
