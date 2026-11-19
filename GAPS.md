# Where GPUI differs from the HTML mock

Running list of things that could not be matched 1:1, and what was done instead.

## Summary — the recurring GPUI limits

Almost every gap below comes from one of these:

1. **Additive blending — solved on macOS with a patched renderer.** GPUI's pipelines only
   alpha-blend, so `vendor/gpui-pre-apple` (wired via `[patch.crates-io]`) adds an additive
   sprite pipeline (One, One) and repurposes the sprite `grayscale` bit as "additive". The
   scenes and onboarding mark the mock's `lighter` draws (`Frame::additive`), so rays, aurora,
   sign glows, spills and glow sprites add light like the mock. Windows/Linux use GPUI's other
   renderer and fall back to normal blending. The onboarding wipe's glowing cells are quads
   (not sprites), so they still blend normally.

2. **No CSS `filter: blur()` or `transform: scale()` on elements.** The mock runs in its
   *flat* style, which sets `backdrop-filter: none` on every element, so the mock has no
   frosted panels to match. What remains is the brief `blur(8px)`→0 on a few entrance
   animations (word rises, `rise`) and small popover scale-ins; those are opacity/slide only.
3. **No gradient-clipped text, letter-spacing or balanced wrapping in the text engine.**
   Worked around: the "Thinking" shimmer colours each glyph from the moving band, and the
   uppercase labels are tracked by laying characters out one by one (`ui::tracked`). Large
   headings keep default tracking, and `text-wrap: balance` isn't available.
4. **No rounded clip masks.** GPUI clips to rectangles, so the rounded scene card paints its
   corners in the page colour — visually identical to the mock's `border-radius` clip.
5. **No web view (by choice).** The Browser tab redraws the mock's fixed preview page natively,
   which looks the same. A `wry` system web view was prototyped (it builds: GPUI exposes the
   window handle) but left out for now — it adds binary size and a native view that sits above
   GPUI popovers.
6. **Linear image sampling.** Pixel art is upscaled nearest-neighbour on the CPU first, which
   costs GPU memory (~30–50 MB per scene at 2×) and a 20–70 ms build on scene switch/resize.
7. **Layout differences from CSS** (no margin collapsing, different default line-height,
   no inline padded spans) — handled in code so spacing matches the mock.
8. **Native controls are custom.** Selects, time pickers and toggles are drawn menus/knobs.

Persistence and reduced motion are ported: state the mock keeps in localStorage (theme,
scene, sidebar layout, projects, labels, composer context, settings, automations, installed
plugins) is saved to `~/.config/tako/state.json`, the intro's "seen" flag to
`~/.config/tako/onboarded`, and macOS *Reduce motion* makes tweens jump to their end and the
backdrop hold still. Embedding a real web view (e.g. `wry`) was considered for the Browser
tab, but the mock itself only shows a fixed srcdoc page, which is redrawn natively; a native
child view would also sit above every GPUI popover.

## Chat
- "Thinking" shimmer: GPUI can't clip a moving gradient to text; each glyph is coloured from the moving band instead (it used to pulse
  between text-3 and text colour).
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
- The heatmap card: the mock's flat style disables its `backdrop-filter`, so a solid
  translucent panel is a match, not a gap.
- `rise` entrance: opacity + 16px lift only, the `blur(8px)` → 0 part is dropped.
- The light sweep over the grid uses `mix-blend-mode: overlay` in the mock; here it is a plain
  7% white gradient band (painted as two 2-stop gradients, since GPUI gradients have 2 stops).
- Cell tooltips: the mock uses the native `title` tooltip; here a small custom tip is drawn
  above the hovered cell. "Today" is UTC days, not local midnight.
- Placeholder "Ask anything, @ to mention…" is owned by chat.rs (shared composer input).

## Right panel
- Letter-spacing: uppercase section headers (`.06em`) are tracked via `ui::tracked`.
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

## Profile menu, Stats, signed-out
- Profile popover: the mock's flat style disables its `backdrop-filter`, so the solid panel matches.
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
- The caption next to the pills is implemented but hidden, as in the mock
  (`.scene-switch .cap { display: none !important }`); `SceneState.show_caption` turns it on.
- Auto theme: the frame is software-composited at art resolution (no additive glows) and
  bilinearly sampled to the 64×40 probe; Tako yields accent ≈ #2a5a9e (mock ≈ #6593d8,
  same hue) — after accent tuning the resulting theme matches closely.
- Reduced motion: honoured via macOS *Reduce motion* (scenes hold a still frame).

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

## Automations
- `<select>`s are `.au-sel` chips that open a custom menu (styled like `.au-menu`)
  instead of the native macOS popup; the `<input type=time>` is a chip with a
  half-hour menu (no free typing of minutes).
- Labels are a local copy of the mock's `forgeLabels` defaults (incl. Client,
  Maintenance); new labels aren't shared with the sidebar yet (`data::LABELS` is const).
- Automations persist in `~/.config/tako/state.json`; the Advanced chevron rotates
  without the .2s transition.

## Plugins
- `.pl-fcard:hover` lifts by 2px instantly (no transform transition); the dot
  pattern is painted as small rounded quads, not a radial-gradient.
- Installed plugins persist between launches (state.json).
## Scenes: Peaks / Glade / Valley
- Valley's light/dark palette: `Scene::build` gets no `cx`, so the Backdrop calls
  `scene::valley::set_light` and keys Valley's build on the theme, rebuilding when it flips
  (the mock rebuilds on `data-theme` changes).
- Valley's lake reflection: the mock copies the canvas back into itself row by row. Here the
  sky and the three ranges each have a mirrored copy, pre-mixed with the water at `waterAlpha`,
  drawn as row strips with the same wobble and per-layer parallax. Clouds, stars and birds
  aren't mirrored. In the mock they only reach the lake's bottom rows now and then.
- Glade's sun shafts use `screen` in the mock and are drawn source-over here. The rays are
  near-white, so the difference is small.
## Onboarding (src/overlays/onboarding*)
- Fade-in/out is opacity only: the mock's `onbOut` also does `scale(1.04)` +
  `blur(6px)`, and every word's `onbWord` rise drops its `blur(8px)` (GPUI has no
  transforms/filters on divs). `text-wrap: balance` and the h1's `letter-spacing:
  -0.035em` / kicker `.12em` aren't available, so lines are a touch wider. The h1 is
  fixed at 52px (the clamp(30px, 4.2vw, 52px) max at 1440 wide).
- `globalCompositeOperation = "lighter"` (rays, every glow sprite, the wipe's glow
  cells) is source-over: rays get a 1.25× alpha boost, glows use pre-baked radial
  sprites at 32 alpha levels (GPUI can only apply opacity per element).
- Story → live dissolve: the story's last recorded frame is composited on the CPU
  (`px::composite`, ~1ms) and uploaded once; each frame the still-covered 7px blocks
  are drawn as row-merged cropped sprites of it, glowing cells as quads.
- Live → story / scene → scene / close-while-live wipes need the *backdrop's* last
  frame, but `scene::Backdrop` keeps its recorded passes private, so the overlay
  builds a throwaway copy of the scene at window size and draws one frame of it
  (~16–30ms hitch on the click). Its animated details (snow, fish…) are from a
  different moment than the live backdrop. Fix: expose the Backdrop's last passes.
- Scene pills switch the backdrop, which also runs its own 0.9s cross-fade under the
  wipe (the mock's home scene switch does the same).
- The intro opens on first launch (flag in `~/.config/tako/onboarded`) and on Replay.
- Reduce motion: tweens jump to their end; the story itself still animates.
