//! Pixel backdrop scenes (Tako, Aurora, Neon Rain, Peaks, Glade, Valley).
//! Port of the mock's PIXEL BACKDROP engine.
//!
//! * Each scene (`Scene` trait, one file each) paints its static layers once per
//!   (size, scene) on the CPU at art resolution (`px` = 3 logical px per art px); the
//!   `Builder` upscales them nearest-neighbour to device pixels and uploads them as
//!   `RenderImage`s.
//! * Every frame the scene records its animated details (rects, image draws) into a
//!   `Frame`; those become GPUI quads / sprites, with layers slid by depth for the mouse
//!   parallax (snapped to device pixels).
//! * The engine lives in its own cached view (`Backdrop`) so its 60fps redraws don't
//!   re-render the rest of the app.
use crate::{app::TakoApp, theme::ActiveTheme};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{cell::Cell, rc::Rc, sync::Arc, time::Instant};

pub mod aurora;
pub mod extract;
pub mod glade;
pub mod neon;
pub mod octo;
pub mod peaks;
pub mod px;
pub mod tako;
pub mod valley;

use px::{Builder, Frame, Pass, Vp};

/// One backdrop scene. `t` is the engine clock (seconds), `vp` the viewport/parallax.
pub trait Scene {
    /// Paint static layers (upload them with `b.img(...)`) and seed the living details.
    fn build(&mut self, b: &mut Builder, vp: &Vp, t: f64);
    fn update(&mut self, vp: &Vp, t: f64, dt: f64);
    fn draw(&self, vp: &Vp, t: f64, f: &mut Frame);
}

pub struct SceneInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// one-line caption shown next to the pills
    pub cap: &'static str,
    /// swatch gradient (from, to)
    pub swatch: (u32, u32),
    /// logical px per art px
    pub px: f64,
    /// parallax range in art px (x, y)
    pub shift: (f64, f64),
    pub make: fn() -> Box<dyn Scene>,
}

/// Registry, in pill order.
pub static SCENES: [SceneInfo; 6] = [
    SceneInfo { id: "tako", name: "Tako", cap: "Tako is here. Ready when you are.", swatch: (0x3a86e0, 0x06102e), px: 3., shift: (9., 3.), make: tako::new },
    SceneInfo { id: "aurora", name: "Aurora", cap: "Clear skies. Your agents work through the night.", swatch: (0x3fd8a0, 0xa77cf0), px: 3., shift: (9., 3.), make: aurora::new },
    SceneInfo { id: "neon", name: "Neon Rain", cap: "The city never sleeps. Neither do your agents.", swatch: (0xe0369a, 0x34e0f0), px: 3., shift: (9., 3.), make: neon::new },
    SceneInfo { id: "peaks", name: "Peaks", cap: "Every lit window is an agent at work.", swatch: (0x6e3483, 0xffe7a8), px: 3., shift: (9., 3.), make: peaks::new },
    SceneInfo { id: "glade", name: "Glade", cap: "Quiet focus. Nothing needs you yet.", swatch: (0xb4e0ea, 0x2f6e34), px: 3., shift: (9., 3.), make: glade::new },
    SceneInfo { id: "valley", name: "Valley", cap: "The day's work, shipping at sundown.", swatch: (0xc8504a, 0xffc58a), px: 4., shift: (7., 2.), make: valley::new },
];

pub fn info(id: &str) -> &'static SceneInfo {
    SCENES.iter().find(|s| s.id == id).unwrap_or(&SCENES[0])
}

pub struct SceneState {
    pub id: &'static str,
    /// The mock hides the caption (`.scene-switch .cap { display: none !important }`);
    /// flip this to show it next to the pills.
    pub show_caption: bool,
    /// Radius of the card the backdrop sits in; its corners are masked with `t.bg`
    /// because GPUI content masks are rectangular.
    pub corner_radius: f32,
    backdrop: Option<Entity<Backdrop>>,
}

impl SceneState {
    pub fn new() -> Self {
        SceneState { id: "tako", show_caption: false, corner_radius: 12., backdrop: None }
    }
}

impl TakoApp {
    /// The animated backdrop that fills the Home card (and the onboarding's last step).
    /// Absolutely positioned to fill the nearest positioned ancestor.
    pub fn render_backdrop(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = cx.entity().downgrade();
        let bd = self.scene.backdrop.get_or_insert_with(|| cx.new(|_| Backdrop::new(app))).clone();
        let (id, r) = (self.scene.id, self.scene.corner_radius);
        bd.update(cx, |b, cx| {
            if b.want != id || b.radius != r {
                b.want = id;
                b.radius = r;
                cx.notify();
            }
        });
        div().absolute().inset_0().child(bd.cached(StyleRefinement::default().size_full()))
    }

    /// The scene switcher pills + caption (top-right of Home).
    pub fn render_scene_switch(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let cur = self.scene.id;
        let cap = info(cur).cap;
        let pills = div().flex().gap(px(2.)).p(px(3.)).rounded(px(10.)).bg(t.panel_solid).border_1().border_color(t.border).children(SCENES.iter().map(|s| {
            let active = s.id == cur;
            let id = s.id;
            div()
                .id(SharedString::from(format!("scene-{id}")))
                .flex()
                .items_center()
                .gap(px(7.))
                .pl(px(7.))
                .pr(px(10.))
                .py(px(4.))
                .rounded(px(7.))
                .text_size(px(12.))
                .line_height(px(15.))
                .text_color(if active { t.text } else { t.hero_sub })
                .when(active, |d| d.bg(t.surface_active))
                .hover(|d| d.text_color(t.text))
                .cursor_pointer()
                .child(
                    div()
                        .size(px(12.))
                        .rounded(px(3.))
                        .bg(linear_gradient(160., linear_color_stop(rgb(s.swatch.0), 0.), linear_color_stop(rgb(s.swatch.1), 1.)))
                        .border_1()
                        .border_color(hsla(0., 0., 1., 0.2)),
                )
                .child(s.name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.scene.id = id;
                    cx.notify();
                }))
        }));
        let caption = div()
            .text_size(px(12.))
            .line_height(px(15.))
            .text_color(t.hero_sub)
            .px(px(11.))
            .py(px(5.))
            .rounded_full()
            .bg(t.panel_solid)
            .border_1()
            .border_color(t.border)
            .child(cap)
            .with_animation(SharedString::from(format!("scene-cap-{cur}")), Animation::new(std::time::Duration::from_millis(900)).with_easing(crate::ui::ease_snappy), |d, p| {
                d.opacity(p).ml(px(8. * (1. - p)))
            });
        div().flex().items_center().gap(px(12.)).when(self.scene.show_caption, |d| d.child(caption)).child(pills)
    }
}

pub fn act(app: &mut TakoApp, k: &str, v: &str, window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k == "scene" {
        app.scene.id = info(v).id;
        cx.notify();
        return true;
    }
    if k == "scenetest" {
        // Dev hook: show just the backdrop in a Home-sized card (window root replaced;
        // the app entity is kept alive so later scripted actions still reach it).
        let _ = v;
        let me = cx.entity();
        window.replace_root(cx, |_, cx| {
            cx.observe(&me, |_, _, cx| cx.notify()).detach();
            SceneTest { app: me }
        });
        // Offscreen snapshot windows never present frames, so drive the loop by hand.
        if v == "pump" {
            window
                .spawn(cx, async move |cx| loop {
                    cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                    if cx.update(|window, cx| {
                        let t0 = Instant::now();
                        window.refresh();
                        let _ = window.draw(cx);
                        if std::env::var_os("TAKO_SCENE_PROF").is_some() {
                            eprintln!("scene window draw: {:?}", t0.elapsed());
                        }
                    })
                    .is_err()
                    {
                        break;
                    }
                })
                .detach();
        }
        return true;
    }
    false
}

struct SceneTest {
    app: Entity<TakoApp>,
}
impl Render for SceneTest {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let (bd, sw) = self.app.update(cx, |a, cx| (a.render_backdrop(window, cx).into_any_element(), a.render_scene_switch(window, cx).into_any_element()));
        div().size_full().bg(t.bg).font_family(crate::ui::FONT).text_color(t.text).pt(px(44.)).pl(px(56.)).pr(px(12.)).pb(px(12.)).child(
            div().size_full().relative().rounded(px(12.)).child(bd).child(div().absolute().top(px(14.)).right(px(14.)).child(sw)),
        )
    }
}

// ------------------------------------------------------------------ engine

struct Live {
    info: &'static SceneInfo,
    scene: Box<dyn Scene>,
    vp: Vp,
    imgs: Vec<Arc<RenderImage>>,
    key: (usize, usize, u32),
}

struct Fade {
    passes: Rc<Vec<Pass>>,
    a: f64,
    /// seconds for the fade-out
    dur: f64,
    /// the faded build's size in art pixels (for anchoring)
    wh: (f64, f64),
    imgs: Vec<Arc<RenderImage>>,
}

/// The backdrop view: owns the live scene, the cross-fade, the clock and the parallax.
pub struct Backdrop {
    app: WeakEntity<TakoApp>,
    want: &'static str,
    radius: f32,
    live: Option<Live>,
    fade: Option<Fade>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    t: f64,
    last: Option<Instant>,
    mx: f64,
    my: f64,
    passes: Rc<Vec<Pass>>,
    extract_at: Option<f64>,
    trash: Vec<Arc<RenderImage>>,
    /// A size change waiting to settle before the scene is rebuilt.
    pending: Option<((usize, usize, u32), Instant)>,
    /// Already built the display-size cover for this resize (don't build again until it settles).
    grown: bool,
}

impl Backdrop {
    fn new(app: WeakEntity<TakoApp>) -> Self {
        Backdrop {
            app,
            want: "tako",
            radius: 12.,
            live: None,
            fade: None,
            bounds: Rc::new(Cell::new(None)),
            t: 0.,
            last: None,
            mx: 0.,
            my: 0.,
            passes: Rc::new(vec![]),
            extract_at: None,
            trash: vec![],
            pending: None,
            grown: false,
        }
    }

    /// Build (or rebuild) the wanted scene for the measured size.
    fn ensure_built(&mut self, sf: f32, light: bool, disp: Option<(f32, f32)>) -> bool {
        let Some(b) = self.bounds.get() else { return false };
        let inf = info(self.want);
        let unit = inf.px as f32;
        let (w, h) = ((f32::from(b.size.width) / unit).ceil() as usize, (f32::from(b.size.height) / unit).ceil() as usize);
        if w < 4 || h < 4 {
            return false;
        }
        // Valley repaints for the light/dark theme (the mock rebuilds on data-theme changes)
        let lit = inf.id == "valley" && light;
        let key = (w, h, (sf * 100.) as u32 + if lit { 1_000_000 } else { 0 });
        if inf.id == "valley" {
            valley::set_light(light);
        }
        // What to build and how to hand over from the old build.
        let (mut bw, mut bh) = (w, h);
        let mut fade_dur = 0.9;
        let mut settle = true;
        if let Some(l) = &self.live {
            if l.info.id == inf.id && l.key == key {
                self.pending = None;
                return false;
            }
            // Same scene, new size (window resize / maximize / fullscreen). The art is never
            // scaled: it's drawn at its native pixel size, anchored bottom-centre, so a resize
            // only crops or reveals. Rebuilding every frame flickered and stretching jittered.
            if l.info.id == inf.id && l.key.2 == key.2 {
                let settled = matches!(self.pending, Some((k, since)) if k == key && since.elapsed().as_millis() >= 150);
                if settled {
                    fade_dur = 0.3;
                } else {
                    if !matches!(self.pending, Some((k, _)) if k == key) {
                        self.pending = Some((key, Instant::now()));
                    }
                    if self.grown || (l.vp.w as usize >= w && l.vp.h as usize >= h) {
                        return false; // the current build still covers the card
                    }
                    // Growing past the build: build once at the display size so it covers any
                    // size the window can reach, crossfade to it, and settle later.
                    if let Some((dw, dh)) = disp {
                        bw = bw.max((dw / unit).ceil() as usize);
                        bh = bh.max((dh / unit).ceil() as usize);
                    }
                    fade_dur = 0.25;
                    settle = false;
                    self.grown = true;
                }
            }
        }
        if settle {
            self.pending = None;
            self.grown = false;
        }
        let key = (bw, bh, key.2);
        let (w, h) = (bw, bh);
        let scene_changed = self.live.as_ref().map(|l| l.info.id != inf.id).unwrap_or(true);
        if let Some(old) = self.live.take() {
            if !self.passes.is_empty() {
                // snapshot the last frame and fade it out over the new build
                if let Some(f) = self.fade.take() {
                    self.trash.extend(f.imgs);
                }
                let wh = (old.vp.w, old.vp.h);
                self.fade = Some(Fade { passes: std::mem::take(&mut self.passes), a: 1., dur: fade_dur, wh, imgs: old.imgs });
            } else {
                self.trash.extend(old.imgs);
            }
        }
        let s = inf.px * sf as f64;
        let mut vp = Vp::new(w, h, s, inf.shift);
        vp.mx = self.mx;
        vp.my = self.my;
        let mut scene = (inf.make)();
        let mut bld = Builder::new(s as f32);
        let t0 = Instant::now();
        scene.build(&mut bld, &vp, self.t);
        if std::env::var_os("TAKO_SCENE_PROF").is_some() {
            eprintln!("scene {} build {}x{}: {:?}", inf.id, w, h, t0.elapsed());
        }
        self.live = Some(Live { info: inf, scene, vp, imgs: bld.made, key });
        // the mock samples the backdrop ~1.85s after a build; a resize of the same scene
        // keeps the palette (re-sampling would shift every colour after a resize)
        if scene_changed {
            self.extract_at = Some(self.t + 1.85);
        }
        true
    }

    fn sample_theme(&mut self, cx: &mut Context<Self>) {
        let Some(l) = &self.live else { return };
        let (w, h) = (l.vp.w as usize, l.vp.h as usize);
        let rgb = px::composite(&self.passes, w, h);
        let spec = extract::extract(&rgb, w, h);
        if std::env::var_os("TAKO_SCENE_PROF").is_some() {
            eprintln!("scene auto theme: {spec:?}");
        }
        if let Some(spec) = spec {
            cx.global_mut::<crate::theme::ThemeState>().set_auto(spec);
            self.app.update(cx, |_, cx| cx.notify()).ok();
        }
    }
}

impl Render for Backdrop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f64()).unwrap_or(0.).min(0.1);
        self.last = Some(now);
        let sf = window.scale_factor();
        let light = cx.t().light;
        let disp = window.display(cx).map(|d| {
            let s = d.bounds().size;
            (f32::from(s.width), f32::from(s.height))
        });
        let just_built = self.ensure_built(sf, light, disp);

        let mut unit = 3f32;
        if let (Some(b), Some(live)) = (self.bounds.get(), self.live.as_mut()) {
            unit = live.info.px as f32;
            // mouse parallax: target from the pointer over the card, eased + a slow drift
            let m = window.mouse_position();
            let (tx, ty) = if b.contains(&m) {
                (((m.x - b.origin.x) / b.size.width - 0.5) as f64 * 2., ((m.y - b.origin.y) / b.size.height - 0.5) as f64 * 2.)
            } else {
                (0., 0.)
            };
            // Reduce motion: the mock draws one still frame and stops animating.
            if !just_built && !crate::ui::reduced_motion() {
                self.t += dt;
            }
            let t = self.t;
            let e = 1. - (-dt * 3.).exp();
            self.mx += (tx + (t * 0.13).sin() * 0.15 - self.mx) * e;
            self.my += (ty + (t * 0.09 + 1.).sin() * 0.15 - self.my) * e;
            live.vp.mx = self.mx;
            live.vp.my = self.my;
            if !just_built {
                live.scene.update(&live.vp, t, dt);
            }
            let t0 = Instant::now();
            let mut f = Frame::new(live.vp.s);
            live.scene.draw(&live.vp, t, &mut f);
            self.passes = Rc::new(f.finish());
            if std::env::var_os("TAKO_SCENE_PROF").is_some() {
                let n: usize = self.passes.iter().map(|p| p.ops.len()).sum();
                eprintln!("scene frame: {:?}, {} passes, {} ops", t0.elapsed(), self.passes.len(), n);
            }
        }
        if let Some(f) = &mut self.fade {
            f.a -= dt / f.dur;
            if f.a <= 0. {
                let f = self.fade.take().unwrap();
                self.trash.extend(f.imgs);
            }
        }
        if self.extract_at.is_some_and(|at| self.t >= at) {
            self.extract_at = None;
            self.sample_theme(cx);
        }

        let t = cx.t();
        let cell = self.bounds.clone();
        let trash = std::mem::take(&mut self.trash);
        let pass_el = |passes: &Rc<Vec<Pass>>, i: usize, wh: (f64, f64)| {
            let ps = passes.clone();
            // Native pixel size, anchored bottom-centre (crop, never stretch). Only if a build
            // is somehow smaller than the card (no display info), scale it up to cover.
            let place = move |b: Bounds<Pixels>| {
                let (aw, ah) = (wh.0 as f32 * unit, wh.1 as f32 * unit);
                let k = (f32::from(b.size.width) / aw).max(f32::from(b.size.height) / ah).max(1.);
                let (aw, ah) = (aw * k, ah * k);
                let o = point(b.origin.x + px((f32::from(b.size.width) - aw) / 2.), b.origin.y + px(f32::from(b.size.height) - ah));
                (Bounds { origin: o, size: size(px(aw), px(ah)) }, unit * k)
            };
            div()
                .absolute()
                .inset_0()
                .when(ps[i].opacity < 1., |d| d.opacity(ps[i].opacity))
                .child(canvas(|_, _, _| (), move |b, _, window, _| {
                    let (b, u) = place(b);
                    px::paint_ops(&ps[i].ops, b, u, window)
                }).size_full())
        };
        let mut root = div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(t.panel_solid)
            .child(
                canvas(
                    move |b, window, _| {
                        cell.set(Some(b));
                        for im in trash {
                            window.drop_image(im).ok();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );
        let live_wh = self.live.as_ref().map(|l| (l.vp.w, l.vp.h)).unwrap_or((1., 1.));
        for i in 0..self.passes.len() {
            root = root.child(pass_el(&self.passes, i, live_wh));
        }
        if let Some(f) = &self.fade {
            let a = f.a as f32;
            let mut ov = div().absolute().inset_0().opacity(a * a * (3. - 2. * a));
            for i in 0..f.passes.len() {
                ov = ov.child(pass_el(&f.passes, i, f.wh));
            }
            root = root.child(ov);
        }
        // round the card's corners (GPUI clips to rectangles only)
        let (r, bg) = (self.radius, t.bg);
        if r > 0. {
            root = root.child(
                canvas(|_, _, _| (), move |b, _, window, _| {
                    let k = 0.4477; // 1 - cubic-bezier circle constant
                    let o = b.origin;
                    let (w, h) = (b.size.width, b.size.height);
                    let rr = px(r);
                    for (cx0, cy0, sx, sy) in [(o.x, o.y, 1., 1.), (o.x + w, o.y, -1., 1.), (o.x, o.y + h, 1., -1.), (o.x + w, o.y + h, -1., -1.)] {
                        let p = |dx: f32, dy: f32| point(cx0 + rr * dx * sx, cy0 + rr * dy * sy);
                        let mut pb = PathBuilder::fill();
                        pb.move_to(p(0., 0.));
                        pb.line_to(p(1., 0.));
                        pb.cubic_bezier_to(p(0., 1.), p(k, 0.), p(0., k));
                        pb.close();
                        if let Ok(path) = pb.build() {
                            window.paint_path(path, bg);
                        }
                    }
                })
                .absolute()
                .size_full(),
            );
        }
        if !crate::ui::reduced_motion() || self.fade.is_some() || self.pending.is_some() {
            window.request_animation_frame();
        }
        root
    }
}
