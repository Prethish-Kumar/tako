# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; the label pulses
  between text-3 and text colour instead.

## Scenes (pixel backdrop)
- Additive (`globalCompositeOperation = "lighter"`) blending isn't available: rays, aurora
  curtains, lake glow, cabin-window glow, neon sign glows and floor spills are drawn
  source-over at the mock's alpha (Tako's light rays get ×1.25 alpha to compensate).
  Glows read slightly dimmer/flatter than the mock where they overlap bright pixels.
- GPUI applies opacity per element, so any image drawn with a global alpha becomes its
  own opacity "pass" (a child canvas); aurora columns are grouped by their 8 alpha levels.
- Aurora curtains are drawn as 1-art-px-wide sprite slices of a ray texture stretched on
  the GPU (rows pre-repeated ×8 so linear filtering stays close to nearest-neighbour);
  the mock resamples nearest-neighbour at device resolution.
- GPUI content masks are rectangular: the card's rounded corners are faked by painting
  `t.bg` corner wedges over the backdrop (`SceneState.corner_radius`, default 12px).
- Hidden/offscreen windows never present frames: `scripts/snap.sh … "scenetest:pump;scene:aurora"`
  shows the backdrop alone and pumps frames by hand so animation/fades progress.
- Peaks, Glade and Valley aren't ported yet; their pills exist but render Tako.
- The caption next to the pills is implemented but hidden, as in the mock
  (`.scene-switch .cap { display: none !important }`); `SceneState.show_caption` turns it on.
- Auto theme: the frame is software-composited at art resolution (no additive glows) and
  bilinearly sampled to the 64×40 probe; Tako yields accent ≈ #2a5a9e (mock ≈ #6593d8,
  same hue) — after accent tuning the resulting theme matches closely.
- `prefers-reduced-motion` isn't honoured (always animates).
