//! Onboarding: "Tako", a four-beat pixel story in the deep sea (port of the mock's
//! ONBOARDING CSS + JS).
//!
//!   I    Meet Tako   a blue octopus rises out of the dark and waves
//!   II   Parallel    each arm reaches a task card that fills with lines
//!   III  Review      finished tasks fly back as pearls into a clam
//!   IV   Dive in     bubbles spell TAKO; then the real backdrop takes over
//!
//! * `story.rs` — the world (static layers + per-frame `px::Frame` recording).
//! * `wipe.rs`  — the pixel-dissolve wipe between the story and the live backdrop.
//! * this file  — the `OnbView` entity (60fps only while open), the copy/controls UI,
//!   the live final step and the scripted actions.
//!
//! App-side effects (scene id, corner radius, theme, view, closing) are queued and
//! applied from the view's own render/listeners, where the `TakoApp` entity isn't
//! leased.
mod story;
mod wipe;

use crate::{app::TakoApp, scene::px, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{cell::Cell, rc::Rc, sync::Arc, time::Instant};
use story::{Glows, Story};
use wipe::Wipe;

struct Step {
    k: &'static str,
    h: &'static str,
    p: &'static str,
    fin: bool,
}
const STEPS: [Step; 4] = [
    Step { k: "Tako", h: "Meet Tako.", p: "Run Claude Code and Codex agents side by side, all in one place.", fin: false },
    Step { k: "Parallel", h: "Every arm runs an agent.", p: "Many tasks at once, each in its own worktree.", fin: false },
    Step { k: "Review", h: "Every task ends in a pull request.", p: "Review the diff and merge it, from your desk or your phone.", fin: false },
    Step { k: "Dive in", h: "You're all set.", p: "Pick a home and start shipping.", fin: true },
];
const N: usize = STEPS.len();

/// The final step's scene pills (id, name, swatch).
const PILLS: [(&str, &str, u32); 6] = [("tako", "Tako", 0x3f72e6), ("aurora", "Aurora", 0x8b6cf0), ("neon", "Neon Rain", 0xe0369a), ("peaks", "Peaks", 0xea7a5c), ("glade", "Glade", 0x7cbc4a), ("valley", "Valley", 0xc8504a)];

/// Wipe glow per scene (the mock's GLOW).
fn glow_of(id: &str) -> [u8; 3] {
    match id {
        "aurora" => [190, 255, 222],
        "neon" => [255, 150, 230],
        "peaks" => [255, 200, 160],
        "glade" => [220, 248, 170],
        "valley" => [255, 190, 176],
        _ => [190, 225, 255],
    }
}
/// Accent + ink per scene (the mock's ACC).
fn acc_of(id: &str) -> (u32, u32) {
    match id {
        "aurora" => (0xb9a4ff, 0x1a0f40),
        "neon" => (0xff8ad0, 0x3a0626),
        "peaks" => (0xffb08f, 0x3a1206),
        "glade" => (0xc6ec8c, 0x18300a),
        "valley" => (0xffa99a, 0x3a0f0a),
        _ => (0x9cc8ff, 0x06163a),
    }
}

/// cubic-bezier(x1, y1, x2, y2) timing function.
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, p: f32) -> f32 {
    let p = p.clamp(0., 1.);
    let b = |t: f32, a: f32, c: f32| 3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * c + t * t * t;
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..20 {
        let m = (lo + hi) / 2.;
        if b(m, x1, x2) < p { lo = m } else { hi = m }
    }
    b((lo + hi) / 2., y1, y2)
}
/// CSS `ease`.
fn css_ease(p: f32) -> f32 {
    bezier(0.25, 0.1, 0.25, 1., p)
}
fn mix_rgb(a: u32, b: u32, f: f32) -> Rgba {
    let c = |v: u32, s: u32| ((v >> s) & 255) as f32 / 255.;
    let m = |s| c(a, s) + (c(b, s) - c(a, s)) * f;
    Rgba { r: m(16), g: m(8), b: m(0), a: 1. }
}
fn wa(a: f32) -> Hsla {
    hsla(0., 0., 1., a)
}
fn rgba_u(c: u32, a: f32) -> Hsla {
    Rgba { r: ((c >> 16) & 255) as f32 / 255., g: ((c >> 8) & 255) as f32 / 255., b: (c & 255) as f32 / 255., a }.into()
}

type Fx = Box<dyn FnOnce(&mut TakoApp, &mut Context<TakoApp>)>;

pub struct OnboardingState {
    /// The overlay is mounted (including while it fades out).
    pub open: bool,
    /// The overlay hides the app behind it (the mock's `body.onb-on .app { visibility: hidden }`):
    /// true from open until the fade-out starts.
    pub covers: bool,
    view: Option<Entity<OnbView>>,
}

impl OnboardingState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        OnboardingState { open: false, covers: false, view: None }
    }
}

impl TakoApp {
    fn onb_view(&mut self, cx: &mut Context<Self>) -> Entity<OnbView> {
        let app = cx.entity().downgrade();
        self.overlays.onboarding.view.get_or_insert_with(|| cx.new(|cx| OnbView::new(app, cx))).clone()
    }

    /// Open the intro at beat I and start it (the mock's `forgeOnboarding.open()`).
    pub fn open_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let v = self.onb_view(cx);
        self.overlays.onboarding.open = true;
        self.overlays.onboarding.covers = true;
        v.update(cx, |v, cx| v.open(window, cx));
        cx.notify();
    }

    /// Fade the intro out (the mock's `close()`).
    pub fn close_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(v) = self.overlays.onboarding.view.clone() {
            v.update(cx, |v, cx| v.close(window, cx));
        }
    }

    /// Whether the app behind should skip rendering its main content.
    pub fn onboarding_covers(&self) -> bool {
        self.overlays.onboarding.open && self.overlays.onboarding.covers
    }

    /// Returns the overlay element when visible (absolutely positioned over the window).
    pub fn render_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.overlays.onboarding.open {
            return None;
        }
        let v = self.onb_view(cx);
        if !v.read(cx).running {
            // opened by flipping the flag: start it properly
            self.overlays.onboarding.covers = true;
            v.update(cx, |v, cx| v.open(window, cx));
        }
        Some(div().absolute().inset_0().child(v.cached(StyleRefinement::default().size_full())).into_any_element())
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// * `onboarding` — open at beat I
/// * `onb:<0-3>`  — jump to a beat and run the world 150 × 1/30 s (the mock's `go(i)`)
/// * `onb:live`   — enter the live final step now (wipe from the story)
/// * `onb:close`  — close (fade out); `onb:next` / `onb:back` — the buttons
/// * `onbscene:<id>` — click a scene pill on the final step
pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    match k {
        "onboarding" => app.open_onboarding(window, cx),
        "onb" | "onbscene" => {
            if !app.overlays.onboarding.open {
                app.open_onboarding(window, cx);
            }
            let view = app.onb_view(cx);
            let (k, v) = (k.to_string(), v.to_string());
            view.update(cx, |o, cx| match (k.as_str(), v.as_str()) {
                ("onb", "live") => {
                    let (w, h) = (f32::from(window.viewport_size().width), f32::from(window.viewport_size().height));
                    o.enter_live(w * 0.62, h * 0.42, window);
                }
                ("onb", "close") => o.close(window, cx),
                ("onb", "next") => o.next(window, cx),
                ("onb", "back") => o.set_step(o.step.saturating_sub(1), false, window),
                ("onb", n) => o.go(n.parse().unwrap_or(0), window),
                (_, id) => o.pick_scene(id, None, window),
            });
            view.update(cx, |_, cx| cx.notify());
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------- the view

pub struct OnbView {
    app: WeakEntity<TakoApp>,
    focus: FocusHandle,
    pub running: bool,
    story: Story,
    glows: Option<Glows>,
    passes: Rc<Vec<px::Pass>>,
    step: usize,
    /// last setStep (copy animation start + the mock's `liveT`)
    step_at: Instant,
    prev_step: usize,
    live: bool,
    was_live: bool,
    /// live clock (for the backdrop ghost snapshots)
    live_t: f64,
    pick: &'static str,
    wipe: Option<Wipe>,
    acc_from: (u32, u32),
    acc_to: (u32, u32),
    acc_at: Instant,
    opened_at: Instant,
    closing: Option<Instant>,
    last: Option<Instant>,
    trash: Vec<Arc<RenderImage>>,
    fx: Vec<Fx>,
    saved_radius: Option<f32>,
    pills: Rc<[Cell<Option<Bounds<Pixels>>>; 6]>,
    snap: bool,
    prof: bool,
}

impl OnbView {
    fn new(app: WeakEntity<TakoApp>, cx: &mut Context<Self>) -> Self {
        let now = Instant::now();
        OnbView {
            app,
            focus: cx.focus_handle(),
            running: false,
            story: Story::new(),
            glows: None,
            passes: Rc::new(vec![]),
            step: 0,
            step_at: now,
            prev_step: 0,
            live: false,
            was_live: false,
            live_t: 0.,
            pick: "tako",
            wipe: None,
            acc_from: acc_of("tako"),
            acc_to: acc_of("tako"),
            acc_at: now,
            opened_at: now,
            closing: None,
            last: None,
            trash: vec![],
            fx: vec![],
            saved_radius: None,
            pills: Rc::new(Default::default()),
            snap: std::env::var_os("TAKO_SNAP").is_some(),
            prof: std::env::var_os("TAKO_ONB_PROF").is_some(),
        }
    }

    fn fx(&mut self, f: impl FnOnce(&mut TakoApp, &mut Context<TakoApp>) + 'static) {
        self.fx.push(Box::new(f));
    }
    /// Apply queued app-side effects (only call where the app entity isn't leased).
    fn flush(&mut self, cx: &mut Context<Self>) {
        if self.fx.is_empty() {
            return;
        }
        let fx = std::mem::take(&mut self.fx);
        self.app
            .update(cx, |a, cx| {
                for f in fx {
                    f(a, cx);
                }
                cx.notify();
            })
            .ok();
    }

    fn ensure_story(&mut self, window: &Window) {
        let vs = window.viewport_size();
        let old = std::mem::take(&mut self.story.made);
        if self.story.resize(f32::from(vs.width), f32::from(vs.height), window.scale_factor()) {
            self.trash.extend(old);
        } else {
            self.story.made = old;
        }
        if self.glows.is_none() {
            let mut b = px::Builder::new(1.);
            self.glows = Some(Glows::new(&mut b));
        }
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self.live {
            self.leave_live(false, window);
        }
        if let Some(w) = self.wipe.take() {
            self.trash.push(w.img);
        }
        self.story.reset();
        self.ensure_story(window);
        self.running = true;
        self.closing = None;
        self.opened_at = now;
        self.last = Some(now);
        self.pick = "tako";
        self.was_live = false;
        self.acc_from = acc_of("tako");
        self.acc_to = acc_of("tako");
        self.step = 0;
        self.set_step(0, true, window);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.closing.is_some() || !self.running {
            return;
        }
        if self.live {
            // freeze a snapshot of the live backdrop over everything while we fade
            self.start_wipe_from_scene(self.app_scene_hint(), 0., 0., self.pick, window);
            if let Some(w) = &mut self.wipe {
                w.freeze = true;
                w.p = 0.;
            }
            self.leave_live(false, window);
        }
        if self.was_live {
            self.fx(|a, _| a.view = crate::app::View::Home);
        }
        self.closing = Some(Instant::now());
        self.fx(|a, _| a.overlays.onboarding.covers = false);
        cx.notify();
    }

    /// The mock's `setStep(i, instant)`.
    fn set_step(&mut self, i: usize, instant: bool, window: &mut Window) {
        let i = i.min(N - 1);
        if i != self.step {
            self.prev_step = self.step;
        } else if instant {
            self.prev_step = i;
        }
        self.step = i;
        self.story.set_step(i, instant);
        if !STEPS[i].fin && self.live {
            self.leave_live(true, window);
        }
        self.step_at = Instant::now();
    }

    /// The mock's `go(i)`: jump and run the world 150 × 1/30 s.
    fn go(&mut self, i: usize, window: &mut Window) {
        self.ensure_story(window);
        if i == 0 {
            self.story.restart_rise();
        }
        self.set_step(i, true, window);
        for _ in 0..150 {
            self.story.update(1. / 30.);
        }
    }

    fn tint(&mut self, id: &str) {
        let now = Instant::now();
        self.acc_from = self.acc_now();
        self.acc_to = acc_of(id);
        self.acc_at = now;
    }
    /// Current accent (+ ink), mid-transition (0.8s ease).
    fn acc_now(&self) -> (u32, u32) {
        let f = css_ease(self.acc_at.elapsed().as_secs_f32() / 0.8);
        let to = |c: Rgba| ((c.r * 255.).round() as u32) << 16 | ((c.g * 255.).round() as u32) << 8 | (c.b * 255.).round() as u32;
        (to(mix_rgb(self.acc_from.0, self.acc_to.0, f)), to(mix_rgb(self.acc_from.1, self.acc_to.1, f)))
    }

    fn start_wipe(&mut self, src: px::Layer, unit: f32, ox: f32, oy: f32, scene: &str, window: &Window) {
        let t0 = Instant::now();
        let (vw, vh) = vsize(window);
        if let Some(w) = self.wipe.take() {
            self.trash.push(w.img);
        }
        self.wipe = Some(Wipe::new(src, unit, window.scale_factor(), vw, vh, ox, oy, glow_of(scene)));
        if self.prof {
            eprintln!("onb wipe start: {:?}", t0.elapsed());
        }
    }

    /// Snapshot of a backdrop scene. The Backdrop's own recorded frame is private to
    /// `scene/`, so build a throwaway copy of the scene at the window size and draw one
    /// frame of it (art-res only, nothing is uploaded).
    fn start_wipe_from_scene(&mut self, id: &str, ox: f32, oy: f32, glow: &str, window: &Window) {
        let t0 = Instant::now();
        let (vw, vh) = vsize(window);
        let inf = crate::scene::info(id);
        let unit = inf.px as f32;
        let (w, h) = ((vw / unit).ceil() as usize, (vh / unit).ceil() as usize);
        let s = inf.px * window.scale_factor() as f64;
        let vp = px::Vp::new(w, h, s, inf.shift);
        let mut scene = (inf.make)();
        let mut b = px::Builder::new(1.);
        let t = self.live_t + 2.;
        scene.build(&mut b, &vp, t);
        let mut f = px::Frame::new(s);
        scene.draw(&vp, t, &mut f);
        let layer = wipe::snapshot(&f.finish(), w, h, [4, 12, 38]);
        if self.prof {
            eprintln!("onb ghost {id} {w}x{h}: {:?}", t0.elapsed());
        }
        self.start_wipe(layer, unit, ox, oy, glow, window);
    }

    /// The mock's `enterLive(ox, oy)`: dissolve the story into the real backdrop.
    fn enter_live(&mut self, ox: f32, oy: f32, window: &mut Window) {
        if self.live {
            return;
        }
        self.ensure_story(window);
        if self.passes.is_empty() {
            self.record();
        }
        let t0 = Instant::now();
        let layer = wipe::snapshot(&self.passes, self.story.w as usize, self.story.h as usize, [4, 12, 38]);
        if self.prof {
            eprintln!("onb story composite {}x{}: {:?}", self.story.w, self.story.h, t0.elapsed());
        }
        let pick = self.pick;
        self.start_wipe(layer, self.story.s as f32, ox, oy, pick, window);
        self.live = true;
        self.was_live = true;
        self.live_t = 0.;
        let saved = self.saved_radius.is_none();
        self.fx(move |a, _| {
            a.scene.id = crate::scene::info(pick).id;
            a.scene.corner_radius = 0.;
        });
        if saved {
            // remember the card radius to restore it when leaving the live state
            self.saved_radius = Some(12.);
        }
        self.tint(pick);
    }

    /// The mock's `leaveLive(withWipe)`.
    fn leave_live(&mut self, with_wipe: bool, window: &mut Window) {
        if !self.live {
            return;
        }
        if with_wipe {
            let (vw, vh) = vsize(window);
            let id = self.app_scene_hint();
            self.start_wipe_from_scene(id, vw / 2., vh / 2., "tako", window);
        }
        self.live = false;
        let r = self.saved_radius.take().unwrap_or(12.);
        self.fx(move |a, _| a.scene.corner_radius = r);
        self.tint("tako");
    }

    /// The scene currently on the backdrop while live = the last pick.
    fn app_scene_hint(&self) -> &'static str {
        crate::scene::info(self.pick).id
    }

    /// A scene pill was clicked (the mock's `onbScenes` click → `switchLive`).
    fn pick_scene(&mut self, id: &str, at: Option<(f32, f32)>, window: &mut Window) {
        let Some(i) = PILLS.iter().position(|p| p.0 == id) else { return };
        let id = PILLS[i].0;
        if id == self.pick && self.live {
            return;
        }
        let (vw, vh) = vsize(window);
        let (ox, oy) = at.or_else(|| self.pills[i].get().map(|b| (f32::from(b.center().x), f32::from(b.center().y)))).unwrap_or((vw * 0.62, vh * 0.42));
        let old = self.pick;
        self.pick = id;
        if !self.live {
            self.enter_live(ox, oy, window);
            return;
        }
        self.start_wipe_from_scene(old, ox, oy, id, window);
        self.fx(move |a, _| a.scene.id = crate::scene::info(id).id);
        self.tint(id);
    }

    fn cta(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pick = self.pick;
        self.fx(move |a, cx| {
            a.scene.id = crate::scene::info(pick).id;
            if pick == "tako" {
                let ts = cx.global_mut::<crate::theme::ThemeState>();
                ts.pal = "auto".into();
                ts.apply();
            }
        });
        self.close(window, cx);
    }

    fn next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if STEPS[self.step].fin {
            self.cta(window, cx);
        } else {
            self.set_step(self.step + 1, false, window);
        }
    }

    fn record(&mut self) {
        let Some(gl) = &self.glows else { return };
        let mut f = px::Frame::new(self.story.s * self.story.key.2 as f64 / 100.);
        self.story.draw(&mut f, gl);
        self.passes = Rc::new(f.finish());
    }
}

fn vsize(window: &Window) -> (f32, f32) {
    let v = window.viewport_size();
    (f32::from(v.width), f32::from(v.height))
}

/// One word / line rising in (the mock's `onbWord`: opacity + translateY(12px); the
/// blur(8px) has no GPUI equivalent).
fn rise(el: Div, since: f32, delay: f32, dur: f32) -> Div {
    let p = ease_snappy((since - delay) / dur);
    el.relative().top(px(12. * (1. - p))).opacity(p)
}

impl Render for OnbView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.flush(cx);
        let now = Instant::now();
        let raw = self.last.map(|l| (now - l).as_secs_f64()).unwrap_or(0.);
        self.last = Some(now);
        // Offscreen snapshots only draw at capture time: run the elapsed wall time in
        // 1/60 s steps so the story is where it would be.
        let steps: Vec<f64> = if self.snap {
            let mut left = raw.min(30.);
            let mut v = vec![];
            while left > 1e-6 {
                v.push(left.min(1. / 60.));
                left -= 1. / 60.;
            }
            v
        } else {
            vec![raw.min(0.05)]
        };
        let dt = if self.snap { raw.min(30.) } else { raw.min(0.05) };
        let (vw, vh) = vsize(window);

        // closing: fade out, then unmount
        if let Some(c) = self.closing {
            if c.elapsed().as_secs_f32() >= 0.9 {
                self.running = false;
                self.closing = None;
                if let Some(w) = self.wipe.take() {
                    self.trash.push(w.img);
                }
                self.passes = Rc::new(vec![]);
                self.fx(|a, _| {
                    a.overlays.onboarding.open = false;
                    a.overlays.onboarding.covers = false;
                });
                if let Some(app) = self.app.upgrade() {
                    let f = app.read(cx).focus.clone();
                    window.focus(&f, cx);
                }
                self.flush(cx);
                let trash = std::mem::take(&mut self.trash);
                return div().size_full().child(canvas(
                    move |_, window, _| {
                        for im in trash {
                            window.drop_image(im).ok();
                        }
                    },
                    |_, _, _, _| {},
                ))
                .into_any_element();
            }
        }

        let t0 = Instant::now();
        self.ensure_story(window);
        // mouse parallax target
        let m = window.mouse_position();
        self.story.tmx = ((f32::from(m.x) / vw - 0.5) * 2.).clamp(-1., 1.) as f64;
        self.story.tmy = ((f32::from(m.y) / vh - 0.5) * 2.).clamp(-1., 1.) as f64;
        if self.live {
            self.live_t += dt;
        } else {
            for d in &steps {
                self.story.update(*d);
            }
            self.record();
        }
        let t_frame = t0.elapsed();
        if let Some(w) = &mut self.wipe {
            if w.advance(dt, self.snap) {
                let w = self.wipe.take().unwrap();
                self.trash.push(w.img);
            }
        }
        if STEPS[self.step].fin && !self.live && self.wipe.is_none() && self.closing.is_none() && self.step_at.elapsed().as_secs_f32() > 2.6 {
            self.enter_live(vw * 0.62, vh * 0.42, window);
            if self.snap {
                // offscreen: the wipe would have started at 2.6s
                let late = self.step_at.elapsed().as_secs_f64() - 2.6;
                if let Some(w) = &mut self.wipe {
                    w.start -= std::time::Duration::from_secs_f64(late.max(0.));
                }
            }
        }
        self.flush(cx);

        // ---------------------------------------------------------- layers
        let unit = self.story.s as f32;
        if self.prof {
            eprintln!("onb render: live={} wipe={:?} closing={}", self.live, self.wipe.as_ref().map(|w| (w.p, w.freeze)), self.closing.is_some());
        }
        let fade = match self.closing {
            Some(c) => 1. - bezier(0.6, 0., 0.4, 1., c.elapsed().as_secs_f32() / 0.9),
            None => css_ease(self.opened_at.elapsed().as_secs_f32() / 1.1),
        };
        // onbOut: the mock scales the overlay to 1.04 while it fades. The story and the
        // frozen wipe are our own drawing, so zoom them around the window centre.
        let zoom = match self.closing {
            Some(c) => 1. + 0.04 * bezier(0.6, 0., 0.4, 1., c.elapsed().as_secs_f32() / 0.9),
            None => 1.,
        };
        let zb = move |b: Bounds<Pixels>| -> Bounds<Pixels> {
            if zoom == 1. {
                return b;
            }
            let (w, h) = (b.size.width * zoom, b.size.height * zoom);
            Bounds { origin: point(b.origin.x - (w - b.size.width) / 2., b.origin.y - (h - b.size.height) / 2.), size: size(w, h) }
        };
        let trash = std::mem::take(&mut self.trash);
        let mut root = div()
            .id("onb")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if this.closing.is_some() {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "escape" => this.close(window, cx),
                    "right" | "enter" | "space" => this.next(window, cx),
                    "left" => this.set_step(this.step.saturating_sub(1), false, window),
                    _ => return,
                }
                cx.stop_propagation();
                this.flush(cx);
                cx.notify();
            }))
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(rgb(0x040c26))
            .font_family(FONT)
            .text_color(white())
            .line_height(relative(1.21))
            .opacity(fade)
            .child(
                canvas(
                    move |_, window, _| {
                        for im in trash {
                            window.drop_image(im).ok();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );
        if self.live && self.closing.is_none() {
            if let Some(app) = self.app.upgrade() {
                let bd = app.update(cx, |a, cx| a.render_backdrop(window, cx).into_any_element());
                root = root.child(bd);
            }
        } else if !self.wipe.as_ref().is_some_and(|w| w.freeze) {
            for i in 0..self.passes.len() {
                let ps = self.passes.clone();
                root = root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .when(ps[i].opacity < 1., |d| d.opacity(ps[i].opacity))
                        .child(canvas(|_, _, _| (), move |b, _, window, _| px::paint_ops(&ps[i].ops, zb(b), unit * zoom, window)).size_full()),
                );
            }
        }
        if let Some(w) = self.wipe.clone() {
            let prof = self.prof;
            root = root.child(
                canvas(|_, _, _| (), move |b, _, window, _| {
                    let t = Instant::now();
                    w.paint(zb(b), window);
                    if prof {
                        eprintln!("onb wipe paint: {:?}", t.elapsed());
                    }
                })
                .absolute()
                .size_full(),
            );
        }
        if self.prof && !self.live {
            let n: usize = self.passes.iter().map(|p| p.ops.len()).sum();
            eprintln!("onb story frame: {:?}, {} passes, {} ops", t_frame, self.passes.len(), n);
        }
        let ui = self.render_ui(window, cx);
        window.request_animation_frame();
        root.child(ui).into_any_element()
    }
}

// ---------------------------------------------------------------- copy + controls

impl OnbView {
    fn render_ui(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let s = &STEPS[self.step];
        let since = self.step_at.elapsed().as_secs_f32();
        let (acc, ink) = self.acc_now();
        let live = self.live;

        // .onb-shade: three linear gradients (piecewise, GPUI gradients have 2 stops)
        let (c, a) = if live { (0x000000u32, [0.62, 0.22, 0.5, 0.14, 0.32]) } else { (0x03091c, [0.8, 0.3, 0.6, 0.2, 0.35]) };
        let lg = |deg: f32, a0: f32, a1: f32| linear_gradient(deg, linear_color_stop(rgba_u(c, a0), 0.), linear_color_stop(rgba_u(c, a1), 1.));
        let (b1, b2, l1, l2) = if live { (0.18, 0.34, 0.42, 0.6) } else { (0.16, 0.3, 0.4, 0.58) };
        let shade = div()
            .absolute()
            .inset_0()
            .child(div().absolute().left_0().right_0().bottom_0().h(relative(b1)).bg(lg(0., a[0], a[1])))
            .child(div().absolute().left_0().right_0().bottom(relative(b1)).h(relative(b2 - b1)).bg(lg(0., a[1], 0.)))
            .child(div().absolute().top_0().bottom_0().left_0().w(relative(l1)).bg(lg(90., a[2], a[3])))
            .child(div().absolute().top_0().bottom_0().left(relative(l1)).w(relative(l2 - l1)).bg(lg(90., a[3], 0.)))
            .child(div().absolute().left_0().right_0().top_0().h(relative(0.14)).bg(lg(180., a[4], 0.)));

        // header: brand + skip
        let logo = canvas(
            |_, _, _| (),
            |b, _, window, _| {
                let cell = b.size.width / 10.;
                for (y, row) in crate::scene::octo::MARK.iter().enumerate() {
                    for (x, ch) in row.chars().enumerate() {
                        let col = match ch {
                            'b' => 0x3f72e6,
                            'h' => 0xa8c8ff,
                            'w' => 0xffffff,
                            'k' => 0x0a1430,
                            _ => continue,
                        };
                        let o = point(b.origin.x + cell * x as f32, b.origin.y + cell * y as f32);
                        window.paint_quad(fill(Bounds::new(o, size(cell, cell)), rgb(col)));
                    }
                }
            },
        )
        .size(px(24.))
        .flex_none();
        let top = div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .flex()
            .items_center()
            .justify_between()
            .pt(px(22.))
            .pl(px(28.))
            .pr(px(24.))
            .child(div().flex().items_center().gap(px(10.)).text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child(logo).child("Tako"))
            .child(
                div()
                    .id("onb-skip")
                    .text_size(px(13.))
                    .text_color(wa(0.62))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(|d| d.text_color(white()).bg(wa(0.08)))
                    .child("Skip intro")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close(window, cx);
                        this.flush(cx);
                    })),
            );

        // copy block: kicker, h1, p, each word rising in
        let words = |text: &str, size: f32, lh: f32, delay0: f32, per: f32, gap: f32| {
            div().flex().flex_wrap().text_size(px(size)).line_height(px(size * lh)).gap_x(px(gap)).children(
                text.split(' ').enumerate().map(|(i, w)| rise(div().child(w.to_string()), since, delay0 + per * i as f32, 0.8)),
            )
        };
        let kick = rise(
            div().font_family(PIXEL).text_size(px(11.)).line_height(px(14.)).text_color(rgb(acc)).mb(px(14.)).child(tracked(&s.k.to_uppercase(), 11., 0.12)),
            since,
            0.05,
            0.7,
        );
        let h1 = words(s.h, 52., 1.05, 0.15, 0.055, 52. * 0.26).font_weight(FontWeight::SEMIBOLD);
        let p = words(s.p, 17., 1.5, 0.52, 0.03, 17. * 0.26).mt(px(14.)).max_w(px(520.)).text_color(wa(0.7));
        let mut copy = div().absolute().left(px(64.)).bottom(px(118.)).max_w(px(680.)).pr(px(24.)).child(kick).child(h1).child(p);
        if s.fin {
            copy = copy.child(rise(self.render_final(since, acc, ink, cx), since, 0.9, 0.8));
        }

        // footer: dots + back / continue
        let dot_w = |st: usize, j: usize| if j == st { 28. } else { 8. };
        let dot_c = |st: usize, j: usize| -> Rgba {
            if j == st {
                rgb(acc)
            } else if j < st {
                Rgba { r: 156. / 255., g: 200. / 255., b: 1., a: 0.55 }
            } else {
                Rgba { r: 1., g: 1., b: 1., a: 0.22 }
            }
        };
        let fw = ease_snappy(since / 0.4);
        let fc = css_ease(since / 0.3);
        let (ps, cs) = (self.prev_step, self.step);
        let dots = div().flex().gap(px(8.)).children((0..N).map(|j| {
            let (c0, c1) = (dot_c(ps, j), dot_c(cs, j));
            let lerp = |a: f32, b: f32| a + (b - a) * fc;
            let col = Rgba { r: lerp(c0.r, c1.r), g: lerp(c0.g, c1.g), b: lerp(c0.b, c1.b), a: lerp(c0.a, c1.a) };
            div()
                .id(("onb-dot", j))
                .h(px(8.))
                .w(px(dot_w(ps, j) + (dot_w(cs, j) - dot_w(ps, j)) * fw))
                .bg(col)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_step(j, false, window);
                    this.flush(cx);
                    cx.notify();
                }))
        }));
        let back_dis = self.step == 0;
        let back = div()
            .id("onb-back")
            .size(px(46.))
            .rounded(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .bg(wa(0.08))
            .when(back_dis, |d| d.opacity(0.3))
            .when(!back_dis, |d| d.cursor_pointer().hover(|d| d.bg(wa(0.14))))
            .child(icon("left", 18., wa(0.75)))
            .on_click(cx.listener(|this, _, window, cx| {
                if this.step > 0 {
                    this.set_step(this.step - 1, false, window);
                    this.flush(cx);
                    cx.notify();
                }
            }));
        let next = (!s.fin).then(|| {
            btn("onb-next", "Continue", 0x9cc8ff, 0x06163a).on_click(cx.listener(|this, _, window, cx| {
                this.next(window, cx);
                this.flush(cx);
                cx.notify();
            }))
        });
        let foot = div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_between()
            .pl(px(64.))
            .pr(px(28.))
            .pb(px(36.))
            .child(dots)
            .child(div().flex().items_center().gap(px(8.)).child(back).children(next));

        div().absolute().inset_0().child(shade).child(top).child(copy).child(foot).into_any_element()
    }

    /// The final step's scene pills + "Dive in".
    fn render_final(&mut self, since: f32, acc: u32, ink: u32, cx: &mut Context<Self>) -> Div {
        let pick = self.pick;
        let pills = div().flex().gap(px(4.)).p(px(4.)).rounded(px(12.)).bg(wa(0.08)).children(PILLS.iter().enumerate().map(|(i, &(id, name, col))| {
            let on = id == pick;
            let cell = self.pills.clone();
            div()
                .id(SharedString::from(format!("onb-scene-{id}")))
                .relative()
                .flex()
                .items_center()
                .gap(px(7.))
                .px(px(12.))
                .py(px(8.))
                .rounded(px(9.))
                .text_size(px(13.5))
                .line_height(px(16.))
                .text_color(if on { white() } else { wa(0.7) })
                .when(on, |d| d.bg(wa(0.14)))
                .hover(|d| d.text_color(white()))
                .cursor_pointer()
                .child(canvas(move |b, _, _| cell[i].set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
                .child(div().size(px(10.)).rounded(px(3.)).bg(rgb(col)).when(on, |d| {
                    d.shadow(vec![
                        BoxShadow { color: wa(0.85), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false },
                        BoxShadow { color: wa(0.9), offset: point(px(0.), px(0.)), blur_radius: px(10.), spread_radius: px(0.), inset: false },
                    ])
                }))
                .child(name)
                .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                    let at = this.pills[i].get().map(|b| (f32::from(b.center().x), f32::from(b.center().y))).or_else(|| {
                        let p = ev.position();
                        Some((f32::from(p.x), f32::from(p.y)))
                    });
                    this.pick_scene(id, at, window);
                    this.flush(cx);
                    cx.notify();
                }))
        }));
        // CTA pulse: box-shadow 0 0 0 0→10px, .45→0, 2.4s ease-in-out, after 2s
        let pt = since - 2.;
        let mut cta = btn("onb-cta", "Dive in", acc, ink);
        if pt > 0. {
            let ph = (pt % 2.4) / 2.4;
            let u = bezier(0.42, 0., 0.58, 1., if ph < 0.5 { ph * 2. } else { 2. - ph * 2. });
            cta = cta.shadow(vec![BoxShadow { color: rgba_u(0x9cc8ff, 0.45 * (1. - u)), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(10. * u), inset: false }]);
        }
        let cta = cta.on_click(cx.listener(|this, _, window, cx| {
            this.cta(window, cx);
            this.flush(cx);
        }));
        div().mt(px(26.)).flex().flex_wrap().items_center().gap(px(14.)).child(pills).child(cta)
    }
}

/// `.onb-cta` / `.onb-next`.
fn btn(id: &'static str, label: &'static str, bg: u32, ink: u32) -> Stateful<Div> {
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(46.))
        .pl(px(22.))
        .pr(px(20.))
        .rounded(px(12.))
        .bg(rgb(bg))
        .text_color(rgb(ink))
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|d| d.top(px(-1.)))
        .child(label)
        .child(icon("chev-r", 16., rgb(ink).into()))
}
