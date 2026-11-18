# Tako (GPUI port) — developer notes

A 1:1 recreation of the HTML mock (`ref/ade.html`, desktop only) in Rust with
GPUI (`gpui-pre` 0.3.12) + gpui-component via `gpui-kit` 0.10.

## Run / snapshot
- `cargo run` opens the app (1440×900).
- `scripts/snap.sh /tmp/x.png chats "open:2;tab:2" 1500` renders the window
  **offscreen** to a PNG and exits (uses GPUI's `render_to_image`; no window
  appears, no screen-recording permission needed). Read the PNG to check your work.
- Actions (`TAKO_ACT`, `;`-separated): `send:text`, `open:<thread id>`, `fold:<project>`,
  `tab:<n>`, `theme`, `profile`, `nosidebar`, `noright`, `wait:<ms>`, `scene:<id>`,
  plus anything a module handles in its own `act()` function.
- Reference screenshots of the mock at 1440×900 (downscaled to 800×500) are in `ref/`.
  The mock source is `ref/ade.html` — CSS (top) and JS sections are labelled with
  banner comments (`grep -n "^   [A-Z]" ref/ade.html`). Port behaviour and sizes
  from the source; measure px values from the CSS, not from the JPGs.

## Layout of the code
- `app.rs` — root view, window chrome, rail, `View` enum, `run_action`.
- `theme.rs` — tokens + theme engine (`cx.t()` gives the current `Theme`).
  Colours: `t.bg, t.panel_solid, t.surface(_hover/_active), t.border, t.text(_2/_3),
  t.accent(_2), t.green/red/yellow/blue, t.composer_card/foot, t.hero_*`.
- `ui.rs` — `icon(name, size, color)` (SVGs in `assets/icons/`, same names as the
  mock's `#i-*` sprite without the `i-`), `spinner`, `rich_text` (backtick code
  spans), `measurer` + `Tween` + `ease_snappy` (= the mock's cubic-bezier(.2,.8,.2,1))
  for height animations, fonts: `FONT` (Inter), `MONO` (JetBrains Mono),
  `SERIF` (Instrument Serif), `PIXEL` (Silkscreen).
- `pix.rs` — 8×8 pixel project icons: `pix("octo", 14.)`.
- `sidebar.rs`, `chat.rs` — done (reference for style/idioms).
- Module stubs to fill: `home.rs`, `right.rs`, `tools.rs`, `scene/`,
  `pages/{automations,plugins,settings,stats}.rs`,
  `overlays/{theme_pop,profile,usage,onboarding}.rs`.
  Each owns a `…State` struct (held by `TakoApp`), its `render_*` fn(s) and an
  `act()` hook for scripted actions.

## GPUI idioms used here
- `use gpui_kit::prelude::FluentBuilder as _;` for `.when()`/`.when_some()`.
- Interactive elements need `.id(...)` before `.on_click(cx.listener(|this, _, window, cx| …))`.
- Animate by reading a `Tween` in render and calling `window.request_animation_frame()`
  while it runs; repeat animations with `.with_animation(id, Animation::new(d).repeat(), |el, p| …)`.
- Custom drawing: `canvas(prepaint, |bounds, _, window, cx| window.paint_quad(fill(bounds, color)))`,
  `PathBuilder::stroke(px(w))` + `window.paint_path`.
- Images from pixels: `RenderImage::new(vec![image::Frame::new(rgba_image)])` (expects **BGRA**),
  shown with `img(ImageSource::Render(Arc<RenderImage>))` or `window.paint_image`.
  GPUI samples images **linearly**, so upscale pixel art with nearest-neighbour on the CPU first.
- Text inputs: `gpui_kit::component::input::{InputState, Input, TextareaState, Textarea}`.
- Things GPUI can't do (backdrop blur, gradient-clipped text, CSS filters…): approximate,
  and note it in `GAPS.md` (append a bullet under your module's heading).
