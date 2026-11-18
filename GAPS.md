# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

## Profile menu, Stats, signed-out
- Profile popover: no `backdrop-filter: blur(24px) saturate(1.4)`; the 94% panel colour is used as-is.
  The open transition (fade + 6px slide) is ported, the scale(.98) and the close transition are not
  (the menu disappears immediately). Offscreen snapshots skip the entrance (they draw a single frame).
- Stats KPI glow (`radial-gradient(closest-side, accent 14%, transparent)`): approximated with 18
  stacked translucent rounded quads (a stadium rather than an exact ellipse).
- No `letter-spacing` in GPUI: headings/numbers (-0.03em … -0.01em) and the uppercase menu
  headers (.08em) render with default tracking. `font-variant-numeric: tabular-nums` is done via the
  `tnum` OpenType feature.
- Heatmap hover `transform: scale(1.35)`: drawn as an enlarged deferred copy of the cell with the
  outline on top (no layout transform / transition). The tooltip is drawn above the cell (not clamped
  to the window's right edge like the mock).
- Scope dropdown and tooltip are `deferred`, so they are not clipped by the page's scroll viewport.
- Token-mix segments don't animate `flex-basis` on scope/range changes.
- Signed-out scene is painted with GPUI quads/paths at 4px art pixels instead of a pixelated canvas;
  the 3-stop water gradient is two 2-stop quads; light rays are anti-aliased (not pixel-snapped).
  Sleeping Tako is a simple placeholder until `scene/octo.rs` lands (see TODO(octo) in
  overlays/profile.rs). Primary button hover is a lighter fill instead of `filter: brightness(1.08)`.
