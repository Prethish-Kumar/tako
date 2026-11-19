//! Home: the scene backdrop, the composer and the project PR heatmap card
//! (port of the mock's "HOME — one screen" CSS + the HOME STATS script).
//! The pixel hero hides the headline and starter chips, so neither is shown.
use crate::{
    app::TakoApp,
    data::PrState,
    pix::pix,
    theme::{ActiveTheme, Theme},
    ui::*,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

const COLS: usize = 53;
const GAP: f32 = 3.;

#[derive(Clone, Copy)]
pub struct Day {
    /// days since 1970-01-01
    pub epoch_day: i64,
    pub n: u32,
}

pub struct Stats {
    pub days: Vec<Day>,
    pub raised: u32,
    pub merged: u32,
    pub open: u32,
    pub closed: u32,
}

pub struct HomeState {
    /// When the current visit to Home started (drives the rise / reveal animations).
    enter: Instant,
    /// Laid-out bounds of the heatmap grid (width drives its aspect-ratio height;
    /// origin maps the pointer to a cell).
    grid: Rc<Cell<Bounds<Pixels>>>,
    /// A timer is pending to wake us for the next light sweep.
    wake: bool,
    hover: Option<usize>,
    stats: Option<(String, Rc<Stats>)>,
}
impl HomeState {
    pub fn new(_: &mut Window, _: &mut Context<TakoApp>) -> Self {
        HomeState {
            enter: Instant::now(),
            grid: Rc::new(Cell::new(Bounds::new(point(px(0.), px(0.)), size(px(664.), px(90.))))),
            wake: false,
            hover: None,
            stats: None,
        }
    }
}

// ---------- deterministic data, same generator as the mock ----------

fn hash(s: &str) -> u32 {
    s.chars().fold(2166136261u32, |h, c| (h ^ c as u32).wrapping_mul(16777619))
}

/// mulberry32, bit-for-bit with the mock's `rng`.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f64 {
        let s = self.0;
        let s = (s ^ (s >> 15)).wrapping_mul(1 | s).wrapping_add(0x6d2b79f5);
        self.0 = s;
        s as f64 / 4294967296.
    }
}

/// JS Math.round (half up).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Local "today" as days since the epoch.
fn today() -> i64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    secs.div_euclid(86400)
}

/// 0 = Sunday … 6 = Saturday (1970-01-01 was a Thursday).
fn weekday(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

/// (year, month 1-12, day) from days since the epoch (Howard Hinnant's civil_from_days).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec"];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub fn stats(pid: &str, open_prs: u32) -> Stats {
    let mut r = Rng(hash(pid));
    let busy = 0.5 + r.next() * 0.9;
    let today = today();
    let start = today - 52 * 7 - weekday(today) + 1;
    let mut days = vec![];
    for (i, d) in (start..=today).enumerate() {
        let wd = weekday(d);
        let weekend = wd == 0 || wd == 6;
        let ramp = 0.45 + (i as f64 / 371.) * 0.8;
        let x = r.next();
        let n = if x < if weekend { 0.7 } else { 0.22 } { 0. } else { js_round(x * 4.2 * busy * ramp * if weekend { 0.4 } else { 1. }) };
        days.push(Day { epoch_day: d, n: n as u32 });
    }
    let raised: u32 = days.iter().map(|d| d.n).sum();
    let open = if open_prs > 0 { open_prs } else { js_round(2. + r.next() * 6.) as u32 };
    let merged = js_round((raised as f64 - open as f64) * (0.84 + r.next() * 0.1)).max(0.) as u32;
    let closed = raised.saturating_sub(open + merged);
    Stats { days, raised, merged, open, closed }
}

/// color-mix(in srgb, a p%, b) with premultiplied alpha, like the browser.
fn mix(a: Hsla, p: f32, b: Hsla) -> Hsla {
    let (a, b) = (Rgba::from(a), Rgba::from(b));
    let q = 1. - p;
    let al = a.a * p + b.a * q;
    if al <= 0. {
        return Rgba { r: 0., g: 0., b: 0., a: 0. }.into();
    }
    let ch = |x: f32, y: f32| (x * a.a * p + y * b.a * q) / al;
    Rgba { r: ch(a.r, b.r), g: ch(a.g, b.g), b: ch(a.b, b.b), a: al }.into()
}

fn level_color(t: &Theme, l: u32) -> Hsla {
    match l {
        0 => t.surface_active,
        1 => mix(t.accent, 0.30, t.surface_active),
        2 => mix(t.accent, 0.55, t.surface_active),
        3 => mix(t.accent, 0.78, t.surface_active),
        _ => t.accent,
    }
}

/// The mock's `rise` keyframes (opacity + 16px lift; the blur can't be done).
fn rise<E: Styled>(el: E, enter: Instant, delay_ms: f32) -> E {
    let p = ((enter.elapsed().as_secs_f32() * 1000. - delay_ms) / 1000.).clamp(0., 1.);
    let e = ease_snappy(p);
    el.relative().top(px(16. * (1. - e))).opacity(e)
}

impl TakoApp {
    /// 1 → 0 while a Home card rises in (delay in ms), for the blur-clearing halo; None once done.
    pub fn home_rise_halo(&self, delay_ms: f32) -> Option<f32> {
        if self.view != crate::app::View::Home || crate::ui::reduced_motion() {
            return None;
        }
        let p = ((self.home.enter.elapsed().as_secs_f32() * 1000. - delay_ms) / 1000.).clamp(0., 1.);
        let k = 1. - ease_snappy(p);
        (k > 0.01).then_some(k)
    }

    pub fn render_home(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        // Element state lives only while it is rendered every frame, so this
        // instant is re-created each time Home is entered: replay the entrance.
        let enter = *window.use_keyed_state("home-enter", cx, |_, _| Instant::now()).read(cx);
        if enter != self.home.enter {
            self.home.enter = enter;
            self.home.hover = None;
        }
        let ms = enter.elapsed().as_secs_f32() * 1000.;
        // intro (rise + reveal) and the light sweep (first 30% of each 9s cycle after 2.5s)
        let sweep_ph = ((ms - 2500.) / 9000.).max(0.).fract();
        if ms < 1600. || (ms >= 2500. && sweep_ph < 0.30) {
            window.request_animation_frame();
        } else if !self.home.wake {
            let wait = if ms < 2500. { 2500. - ms } else { (1. - sweep_ph) * 9000. };
            self.home.wake = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(wait as u64 + 5)).await;
                this.update(cx, |this, cx| {
                    this.home.wake = false;
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }

        let composer = self.render_composer(true, window, cx).into_any_element();
        let card = self.render_home_stats(window, cx).into_any_element();
        let backdrop = self.render_backdrop(window, cx).into_any_element();
        let switch = self.render_scene_switch(window, cx).into_any_element();

        div()
            .id("home")
            .size_full()
            .relative()
            .rounded(px(12.))
            .overflow_hidden()
            .bg(t.bg)
            .child(div().absolute().top_0().left_0().size_full().child(backdrop))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .py(px(40.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(rise(div().w_full(), enter, 80.).child(composer))
                    .child(rise(div().w_full().mt(px(-2.)), enter, 200.).child(div().w_full().max_w(px(760.)).mx_auto().px(px(32.)).child(card))),
            )
            .child(div().absolute().top(px(14.)).right(px(14.)).child(switch))
    }

    fn render_home_stats(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let pid = "tako";
        let proj = self.projects.iter().find(|p| p.id == pid).cloned();
        let (name, icon_id) = proj.map(|p| (p.name, p.icon)).unwrap_or((pid, "star"));
        let key = format!("{pid}|{name}|{icon_id}");
        if self.home.stats.as_ref().is_none_or(|(k, _)| *k != key) {
            let open = self.threads.iter().filter(|t| t.repo == pid && matches!(t.pr_state, PrState::Open | PrState::Draft)).count() as u32;
            self.home.stats = Some((key, Rc::new(stats(pid, open))));
        }
        let d = self.home.stats.as_ref().unwrap().1.clone();
        let max = d.days.iter().map(|d| d.n).max().unwrap_or(1).max(1);
        let lvl = move |n: u32| if n == 0 { 0 } else { ((n as f32 / max as f32) * 4.).ceil().min(4.) as u32 };

        // month labels at the first column that starts in a new month
        let mut months: Vec<(usize, &str)> = vec![];
        let mut prev = 0;
        let mut c = 0;
        while c * 7 < d.days.len() {
            let m = civil(d.days[c * 7].epoch_day).1;
            if m != prev {
                months.push((c, MONTHS[m as usize - 1]));
                prev = m;
            }
            c += 1;
        }
        let months: Vec<(usize, &str)> =
            months.iter().enumerate().filter(|(i, (c, _))| months.get(i + 1).is_none_or(|n| n.0 - c >= 3)).map(|(_, m)| *m).collect();

        let num = |v: u32, label: &'static str| {
            div().flex().child(div().font_weight(FontWeight::SEMIBOLD).text_color(t.text).mr(px(5.)).child(v.to_string())).child(label)
        };

        let grid_b = self.home.grid.clone();
        let w = f32::from(grid_b.get().size.width);
        let cell = ((w - GAP * (COLS as f32 - 1.)) / COLS as f32).max(1.);
        let grid_h = 7. * cell + 6. * GAP;
        let enter = self.home.enter;
        let hover = self.home.hover;
        let days = d.clone();
        let colors: Vec<Hsla> = (0..5).map(|l| level_color(&t, l)).collect();
        let outline = t.text_2;

        let grid = canvas(
            move |b, _, _| {
                grid_b.set(b);
            },
            move |b, _, window, _| {
                let w = f32::from(b.size.width);
                let cell = ((w - GAP * (COLS as f32 - 1.)) / COLS as f32).max(1.);
                let ms = enter.elapsed().as_secs_f32() * 1000.;
                for (i, day) in days.days.iter().enumerate() {
                    let (col, row) = (i / 7, i % 7);
                    // sunrise reveal: weeks light up left to right
                    let p = ((ms - 250. - col as f32 * 14.) / 550.).clamp(0., 1.);
                    let e = ease_snappy(p);
                    if e <= 0. {
                        continue;
                    }
                    let s = cell * (0.4 + 0.6 * e);
                    let x = f32::from(b.origin.x) + col as f32 * (cell + GAP) + (cell - s) / 2.;
                    let y = f32::from(b.origin.y) + row as f32 * (cell + GAP) + (cell - s) / 2.;
                    let mut c = colors[lvl(day.n) as usize];
                    c.a *= e;
                    let r = Bounds::new(point(px(x), px(y)), size(px(s), px(s)));
                    window.paint_quad(fill(r, c).corner_radii(px(2.)));
                    if hover == Some(i) {
                        let o = Bounds::new(point(px(x - 2.), px(y - 2.)), size(px(s + 4.), px(s + 4.)));
                        window.paint_quad(outline_quad(o, outline));
                    }
                }
                // a faint band of light drifts across now and then (9s loop after 2.5s)
                let tt = (ms - 2500.) / 9000.;
                if tt > 0. {
                    let ph = tt.fract();
                    if ph < 0.30 {
                        let k = ph / 0.30;
                        let k = k * k * (3. - 2. * k);
                        let off = w * (-1. + 2. * k);
                        let band = Hsla { a: 0.07, ..gpui_kit::white() };
                        let clear = Hsla { a: 0., ..band };
                        let gb = Bounds::new(point(b.origin.x, b.origin.y - px(4.)), size(b.size.width, b.size.height + px(8.)));
                        window.with_content_mask(Some(ContentMask { bounds: gb }), |window| {
                            let half = w * 0.1;
                            let x0 = f32::from(b.origin.x) + off + w * 0.4;
                            let l = Bounds::new(point(px(x0), gb.origin.y), size(px(half), gb.size.height));
                            let r = Bounds::new(point(px(x0 + half), gb.origin.y), size(px(half), gb.size.height));
                            window.paint_quad(fill(l, linear_gradient(100., linear_color_stop(clear, 0.), linear_color_stop(band, 1.))));
                            window.paint_quad(fill(r, linear_gradient(100., linear_color_stop(band, 0.), linear_color_stop(clear, 1.))));
                        });
                    }
                }
            },
        )
        .w_full()
        .h(px(grid_h));

        let n_days = d.days.len();
        let grid_b = self.home.grid.clone();
        let tip = hover.and_then(|i| d.days.get(i).map(|day| (i, *day))).map(|(i, day)| {
            let (_, m, dd) = civil(day.epoch_day);
            let today = i == n_days - 1;
            let what = match day.n {
                0 => "No PRs".to_string(),
                1 => "1 PR".to_string(),
                n => format!("{n} PRs"),
            };
            let text = format!("{what} · {} {dd} {}{}", WEEKDAYS[weekday(day.epoch_day) as usize], MONTHS[m as usize - 1], if today { " (today)" } else { "" });
            let (col, row) = (i / 7, i % 7);
            let x = col as f32 * (cell + GAP) + cell / 2.;
            let y = row as f32 * (cell + GAP);
            div()
                .absolute()
                .left(px(x))
                .top(px(y - 30.))
                .child(
                    div()
                        .relative()
                        .left(px(-80.))
                        .w(px(160.))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(6.))
                                .bg(t.panel_solid)
                                .border_1()
                                .border_color(t.border)
                                .shadow_md()
                                .text_size(px(11.5))
                                .text_color(t.text)
                                .whitespace_nowrap()
                                .child(text),
                        ),
                )
        });

        div()
            .w_full()
            .pt(px(14.))
            .px(px(16.))
            .pb(px(12.))
            .rounded(px(14.))
            .bg(Hsla { a: t.panel_solid.a * 0.74, ..t.panel_solid })
            // `rise`'s blur(8px) → 0: a soft halo in the card's colour that clears as it lands
            .when_some(self.home_rise_halo(200.), |d, k| d.shadow(vec![BoxShadow { color: t.panel_solid.opacity(0.85 * k), offset: point(px(0.), px(0.)), blur_radius: px(14. * k), spread_radius: px(3. * k), inset: false }]))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .mb(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(t.text_2)
                            .whitespace_nowrap()
                            .child(pix(icon_id, 16.))
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(t.text).child(name))
                            .child("· last 12 months"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(18.))
                            .text_size(px(12.5))
                            .text_color(t.text_2)
                            .child(num(d.raised, "raised"))
                            .child(num(d.merged, "merged"))
                            .child(num(d.open, "open"))
                            .child(num(d.closed, "closed")),
                    ),
            )
            .child(
                div().relative().h(px(14.)).text_size(px(10.5)).text_color(t.text_2).children(
                    months.into_iter().map(|(c, m)| div().absolute().top_0().left(relative(c as f32 / COLS as f32)).line_height(px(12.)).child(m)),
                ),
            )
            .child(
                div()
                    .id("hs-grid")
                    .relative()
                    .w_full()
                    .child(grid)
                    .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, _, cx| {
                        let b = grid_b.get();
                        let w = f32::from(b.size.width);
                        let cell = ((w - GAP * (COLS as f32 - 1.)) / COLS as f32).max(1.);
                        let x = f32::from(e.position.x - b.origin.x);
                        let y = f32::from(e.position.y - b.origin.y);
                        let (col, row) = ((x / (cell + GAP)).floor(), (y / (cell + GAP)).floor());
                        let i = (col >= 0. && row >= 0. && row < 7.).then(|| col as usize * 7 + row as usize).filter(|i| *i < n_days);
                        if i != this.home.hover {
                            this.home.hover = i;
                            cx.notify();
                        }
                    }))
                    .on_hover(cx.listener(|this, h: &bool, _, cx| {
                        if !*h && this.home.hover.is_some() {
                            this.home.hover = None;
                            cx.notify();
                        }
                    }))
                    .children(tip),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(3.))
                    .mt(px(8.))
                    .text_size(px(10.5))
                    .text_color(t.text_2)
                    .child(div().mr(px(5.)).child("Less"))
                    .children((0..5).map(|l| div().size(px(10.)).rounded(px(2.)).bg(level_color(&t, l))))
                    .child(div().ml(px(5.)).child("More")),
            )
    }
}

/// A 1px outline (the cell hover ring).
fn outline_quad(b: Bounds<Pixels>, c: Hsla) -> PaintQuad {
    quad(b, px(3.), transparent_black(), px(1.), c, BorderStyle::Solid)
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
/// `hscell:<i>` hovers heatmap cell i (to snapshot the tooltip).
pub fn act(app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k == "hscell" {
        app.home.hover = v.parse().ok();
        cx.notify();
        return true;
    }
    false
}
