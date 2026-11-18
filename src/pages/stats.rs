//! Stats page (port of the mock's PROFILE → Stats page).
//!
//! All numbers come from the mock's seeded generator (same LCG, same call
//! order, same float maths) so every scope/range shows exactly what the HTML
//! shows. Dates are plain calendar days (the mock runs in a timezone without
//! DST, so `SINCE + i·DAY` never drifts).
use crate::{app::TakoApp, pix::pix, theme::ActiveTheme, ui::*};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{sync::Arc, sync::OnceLock, time::Duration};

// ───────────────────────── data (seeded, deterministic) ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prov {
    Claude,
    Codex,
}

impl Prov {
    fn key(self) -> &'static str {
        match self {
            Prov::Claude => "claude",
            Prov::Codex => "codex",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Prov::Claude => "Claude",
            Prov::Codex => "Codex",
        }
    }
    /// The provider badge colour (`.pv`).
    fn color(self) -> Hsla {
        match self {
            Prov::Claude => rgb(0xd97757).into(),
            Prov::Codex => rgb(0x5b7cfa).into(),
        }
    }
    fn letter(self) -> &'static str {
        match self {
            Prov::Claude => "C",
            Prov::Codex => "X",
        }
    }
}

pub struct Acct {
    pub id: &'static str,
    pub p: Prov,
    pub label: &'static str,
    pub plan: &'static str,
}

/// The usage module's seed accounts (USAGE → SEED.accounts).
pub const ACCTS: &[Acct] = &[
    Acct { id: "c1", p: Prov::Claude, label: "Personal", plan: "Max 20×" },
    Acct { id: "c2", p: Prov::Claude, label: "Work", plan: "Team" },
    Acct { id: "x1", p: Prov::Codex, label: "Personal", plan: "Plus" },
    Acct { id: "x2", p: Prov::Codex, label: "Acme client", plan: "Pro" },
];

/// Aug 31 2026 … Nov 18 2026 inclusive.
pub const DAYS: usize = 80;
const MONTHS: [&str; 4] = ["Aug", "Sep", "Oct", "Nov"];

/// Day index relative to SINCE (Aug 31 = 0) → "Mon D".
fn fmt_d(idx: i32) -> String {
    let mut o = idx + 30; // days since Aug 1
    for (m, len) in [31, 30, 31, 30].iter().enumerate() {
        if o < *len {
            return format!("{} {}", MONTHS[m], o + 1);
        }
        o -= len;
    }
    format!("Dec {}", o + 1)
}
/// Sunday = 0 (Aug 31 2026 was a Monday).
fn weekday(idx: i32) -> i32 {
    (1 + idx).rem_euclid(7)
}

fn rng(seed: u32) -> impl FnMut() -> f64 {
    let mut s = seed as u64;
    move || {
        s = (s * 1664525 + 1013904223) & 0xffff_ffff;
        s as f64 / 4294967296.0
    }
}
/// JS Math.round.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[derive(Clone, Copy, Default)]
struct Raw {
    tot: f64,
    inp: f64,
    out: f64,
    cr: f64,
    cw: f64,
    rs: f64,
    agents: f64,
}

fn series(a: &Acct) -> &'static Vec<Raw> {
    static CACHE: OnceLock<Vec<Vec<Raw>>> = OnceLock::new();
    let all = CACHE.get_or_init(|| ACCTS.iter().map(gen_series).collect());
    let i = ACCTS.iter().position(|x| x.id == a.id).unwrap_or(0);
    &all[i]
}

fn gen_series(a: &Acct) -> Vec<Raw> {
    let mut h: i64 = 7;
    for c in a.id.chars() {
        h = h * 31 + c as i64;
    }
    let mut r = rng(h as u32);
    let heavy = if a.p == Prov::Claude { 1. } else { 0.55 };
    let l = a.label.to_lowercase();
    let work = if ["work", "team", "acme", "client"].iter().any(|w| l.contains(w)) { 1.25 } else { 0.8 };
    let mut out = Vec::with_capacity(DAYS);
    for i in 0..DAYS {
        let wd = weekday(i as i32);
        let we = wd == 0 || wd == 6;
        let ramp = 0.45 + 0.55 * (i as f64 / DAYS as f64);
        let off = r() < if we { 0.55 } else { 0.14 };
        let tot = if off {
            0.
        } else {
            let (a1, a2) = (r(), r());
            js_round((2. + a1 * a2 * 46.) * 1e6 * heavy * work * ramp * if we { 0.5 } else { 1. })
        };
        let cr = tot * (0.82 + r() * 0.12);
        let rest = tot - cr;
        let agents = if off { 0. } else { js_round(tot / 1.6e6 + r() * 4.) };
        out.push(Raw {
            tot,
            inp: rest * 0.38,
            out: rest * 0.28,
            cr,
            cw: if a.p == Prov::Claude { rest * 0.24 } else { 0. },
            rs: if a.p == Prov::Codex { rest * 0.34 } else { rest * 0.1 },
            agents,
        });
    }
    out
}

#[derive(Clone, Copy, Default)]
struct Day {
    idx: i32,
    tot: f64,
    inp: f64,
    out: f64,
    cr: f64,
    cw: f64,
    rs: f64,
    cost: f64,
    agents: f64,
    /// tokens per provider, in insertion order (claude first when present)
    by: [Option<f64>; 2],
}

fn price(p: Prov) -> [f64; 5] {
    match p {
        Prov::Claude => [3., 15., 0.3, 3.75, 15.],
        Prov::Codex => [1.25, 10., 0.125, 0., 10.],
    }
}

fn in_scope(a: &Acct, scope: &str) -> bool {
    scope == "all" || a.p.key() == scope || a.id == scope
}

fn collect(scope: &str) -> (usize, Vec<Day>) {
    let list: Vec<&Acct> = ACCTS.iter().filter(|a| in_scope(a, scope)).collect();
    let mut d: Vec<Day> = (0..DAYS).map(|i| Day { idx: i as i32, ..Default::default() }).collect();
    for a in &list {
        let p = price(a.p);
        for (i, x) in series(a).iter().enumerate() {
            let m = &mut d[i];
            m.tot += x.tot;
            m.inp += x.inp;
            m.out += x.out;
            m.cr += x.cr;
            m.cw += x.cw;
            m.rs += x.rs;
            m.agents += x.agents;
            m.cost += (x.inp * p[0] + x.out * p[1] + x.cr * p[2] + x.cw * p[3] + x.rs * p[4]) / 1e6;
            let slot = (a.p == Prov::Codex) as usize;
            m.by[slot] = Some(m.by[slot].unwrap_or(0.) + x.tot);
        }
    }
    (list.len(), d)
}

pub fn fmt_n(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.2}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}k", n / 1e3)
    } else {
        format!("{}", js_round(n))
    }
}
pub fn fmt_int(n: f64) -> String {
    let s = format!("{}", js_round(n) as i64);
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
pub fn fmt_usd(n: f64) -> String {
    if n >= 1000. { format!("${}", fmt_int(n)) } else { format!("${:.2}", n) }
}
pub fn dur(h: f64) -> String {
    let d = (h / 24.).floor();
    let hh = js_round(h % 24.);
    if d > 0. { format!("{d}d {hh}h") } else { format!("{hh}h") }
}

pub struct Totals {
    pub agents: f64,
    pub hours: f64,
    pub prs: f64,
    pub merged: f64,
}
pub fn totals() -> Totals {
    let agents = collect("all").1.iter().fold(0., |s, d| s + d.agents);
    Totals { agents, hours: agents * 0.21, prs: js_round(agents / 12.5), merged: js_round(agents / 15.4) }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Range {
    D7,
    D30,
    All,
}
impl Range {
    fn n(self) -> usize {
        match self {
            Range::D7 => 7,
            Range::D30 => 30,
            Range::All => DAYS,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Range::D7 => "7D",
            Range::D30 => "30D",
            Range::All => "All",
        }
    }
}

/// Everything the page shows for one scope + range.
struct Model {
    n_accts: usize,
    tot: f64,
    cost: f64,
    active: usize,
    cache_share: f64,
    delta: Option<f64>,
    cells: Vec<Cell>,
    best_idx: i32,
    cur: usize,
    longest: usize,
    mix: Vec<(&'static str, f64, usize)>, // label, value, colour slot
    rs: f64,
    models: Vec<(&'static str, f64, Prov)>,
    projects: Vec<(String, f64, &'static str)>,
    peak: f64,
}

#[derive(Clone)]
struct Cell {
    fut: bool,
    level: u8, // 0 = base, 1..4
    best: bool,
    tip: (String, String, String),
}

const WEEKS: usize = 12;
/// Heatmap starts on the Sunday 11 weeks before this week (Aug 30 = idx -1).
const HM_START: i32 = -1;

fn model(scope: &str, range: Range, projects: &[(String, &'static str)]) -> Model {
    let (n_accts, d) = collect(scope);
    let n = range.n();
    let win = &d[DAYS - n..];
    let sum = |f: &dyn Fn(&Day) -> f64| win.iter().fold(0., |s, x| s + f(x));
    let tot = sum(&|x| x.tot);
    let cost = sum(&|x| x.cost);
    let active = win.iter().filter(|x| x.tot > 0.).count();
    let cache_share = if tot > 0. { sum(&|x| x.cr) / tot } else { 0. };
    let prev_tot = if 2 * n <= DAYS { d[DAYS - 2 * n..DAYS - n].iter().fold(0., |s, x| s + x.tot) } else { 0. };
    let delta = if range != Range::All && prev_tot > 0. { Some(js_round((tot / prev_tot - 1.) * 100.)) } else { None };

    // heatmap
    let mut vals: Vec<f64> = d.iter().map(|x| x.tot).filter(|v| *v > 0.).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| {
        if vals.is_empty() {
            return 1.;
        }
        let v = vals[(p * (vals.len() - 1) as f64).floor() as usize];
        if v == 0. { 1. } else { v }
    };
    let (q3, q6, q88) = (q(0.3), q(0.6), q(0.88));
    let lv = |v: f64| if v == 0. { 0 } else if v < q3 { 1 } else if v < q6 { 2 } else if v < q88 { 3 } else { 4 };
    let best = d.iter().fold(d[0], |b, x| if x.tot > b.tot { *x } else { b });
    let mut cells = vec![];
    for i in 0..WEEKS * 7 {
        let idx = HM_START + i as i32;
        if idx > DAYS as i32 - 1 {
            cells.push(Cell { fut: true, level: 0, best: false, tip: Default::default() });
            continue;
        }
        let x = if idx >= 0 { Some(d[idx as usize]) } else { None };
        cells.push(Cell {
            fut: false,
            level: x.map(|x| lv(x.tot)).unwrap_or(0),
            best: x.map(|x| x.idx == best.idx).unwrap_or(false),
            tip: (
                fmt_d(idx),
                x.map(|x| format!("{} tokens", fmt_n(x.tot))).unwrap_or("No activity".into()),
                x.filter(|x| x.tot > 0.).map(|x| fmt_usd(x.cost)).unwrap_or_default(),
            ),
        });
    }
    // streaks
    let (mut cur, mut longest, mut run) = (0, 0, 0);
    for x in &d {
        run = if x.tot > 0. { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    for x in d.iter().rev() {
        if x.tot == 0. {
            break;
        }
        cur += 1;
    }
    // mix
    let mix: Vec<_> = [("Input", sum(&|x| x.inp), 0), ("Output", sum(&|x| x.out), 1), ("Cache read", sum(&|x| x.cr), 2), ("Cache write", sum(&|x| x.cw), 3), ("Reasoning", sum(&|x| x.rs), 4)]
        .into_iter()
        .filter(|m| m.1 > 0.)
        .collect();
    let rs = sum(&|x| x.rs);
    // models
    let mut by_p: Vec<(Prov, f64)> = vec![];
    for x in win {
        for (slot, v) in x.by.iter().enumerate() {
            if let Some(v) = v {
                let p = if slot == 0 { Prov::Claude } else { Prov::Codex };
                match by_p.iter_mut().find(|e| e.0 == p) {
                    Some(e) => e.1 += v,
                    None => by_p.push((p, *v)),
                }
            }
        }
    }
    let mut models: Vec<(&'static str, f64, Prov)> = by_p
        .iter()
        .flat_map(|(p, v)| {
            let list: &[(&'static str, f64)] = match p {
                Prov::Claude => &[("Opus 5.5", 0.58), ("Sonnet 5.5", 0.34), ("Haiku 5.5", 0.08)],
                Prov::Codex => &[("GPT-5 Codex", 0.81), ("GPT-5 Codex mini", 0.19)],
            };
            list.iter().map(move |(m, f)| (*m, v * f, *p))
        })
        .collect();
    models.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    // projects
    let mut pr = rng((scope.len() * 97 + n) as u32);
    let weights: Vec<f64> = (0..projects.len()).map(|i| if i == 0 { 3.2 } else { 1. } * (0.4 + pr())).collect();
    let wsum = weights.iter().fold(0., |a, b| a + b);
    let wsum = if wsum == 0. { 1. } else { wsum };
    let mut pj: Vec<(String, f64, &'static str)> = projects.iter().enumerate().map(|(i, (name, ic))| (name.clone(), tot * weights[i] / wsum, *ic)).collect();
    pj.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    pj.truncate(5);
    let peak = d.iter().fold(f64::MIN, |m, x| m.max(x.agents));

    Model { n_accts, tot, cost, active, cache_share, delta, cells, best_idx: best.idx, cur, longest, mix, rs, models, projects: pj, peak }
}

// ───────────────────────── state ─────────────────────────

pub struct StatsState {
    pub scope: String,
    pub range: Range,
    pub sel_open: bool,
    pub hover: Option<usize>,
    pub refreshing: bool,
    pub scroll: ScrollHandle,
}

impl StatsState {
    pub fn new(_window: &mut Window, _cx: &mut Context<TakoApp>) -> Self {
        StatsState { scope: "all".into(), range: Range::D30, sel_open: false, hover: None, refreshing: false, scroll: ScrollHandle::new() }
    }
}

fn scope_label(s: &str) -> (String, String) {
    match s {
        "all" => ("Overview".into(), "All accounts".into()),
        "claude" => ("Claude".into(), "Every Claude account".into()),
        "codex" => ("Codex".into(), "Every Codex account".into()),
        _ => {
            let a = ACCTS.iter().find(|a| a.id == s).unwrap_or(&ACCTS[0]);
            (format!("{} · {}", a.p.name(), a.label), a.plan.into())
        }
    }
}
fn scope_prov(s: &str) -> Option<Prov> {
    match s {
        "all" => None,
        "claude" => Some(Prov::Claude),
        "codex" => Some(Prov::Codex),
        _ => ACCTS.iter().find(|a| a.id == s).map(|a| a.p),
    }
}

/// color-mix(in srgb, a p%, b)
pub fn mix(a: Hsla, p: f32, b: Hsla) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.into(), b.into());
    let wa = a.a * p;
    let wb = b.a * (1. - p);
    let al = wa + wb;
    if al <= 0. {
        return hsla(0., 0., 0., 0.);
    }
    Rgba { r: (a.r * wa + b.r * wb) / al, g: (a.g * wa + b.g * wb) / al, b: (a.b * wa + b.b * wb) / al, a: al }.into()
}

pub fn tnum() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// An approximation of `radial-gradient(closest-side, c, transparent)`:
/// concentric rounded quads whose alphas stack to a linear falloff.
pub fn radial_glow(c: Hsla) -> Canvas<()> {
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let steps = 18;
            for k in 0..steps {
                let f = 1. - k as f32 / steps as f32;
                let w = b.size.width * f;
                let h = b.size.height * f;
                let o = point(b.origin.x + (b.size.width - w) / 2., b.origin.y + (b.size.height - h) / 2.);
                let r = w.min(h) / 2.;
                window.paint_quad(fill(Bounds { origin: o, size: size(w, h) }, alpha(c, 1. / steps as f32)).corner_radii(r));
            }
        },
    )
}

fn alpha(c: Hsla, a: f32) -> Hsla {
    hsla(c.h, c.s, c.l, c.a * a)
}

impl TakoApp {
    pub fn open_stats(&mut self, cx: &mut Context<Self>) {
        self.pages.stats.scroll.set_offset(point(px(0.), px(0.)));
        self.pages.stats.sel_open = false;
        self.go(crate::app::View::Stats, cx);
    }

    pub fn render_stats(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let st = &self.pages.stats;
        let projects: Vec<(String, &'static str)> = self.projects.iter().map(|p| (p.name.to_string(), p.icon)).collect();
        let m = model(&st.scope, st.range, &projects);
        let tt = totals();
        let n = st.range.n();
        let (sl, _ss) = scope_label(&st.scope);
        let pk = scope_prov(&st.scope);
        let base = alpha(t.text, 0.06);
        let lvl = [base, mix(t.accent, 0.28, transparent_black()), mix(t.accent, 0.5, transparent_black()), mix(t.accent, 0.76, transparent_black()), t.accent];
        let mix_cols = [t.accent, t.blue, alpha(t.text, 0.32), t.yellow, t.green];

        // ── small builders ──
        let ss_ic = |name: &'static str, sz: f32, r: f32| {
            div().size(px(sz)).rounded(px(r)).flex().flex_none().items_center().justify_center().bg(t.surface).border_1().border_color(t.border).child(icon(name, 18., t.text_2))
        };
        let pv = |p: Prov| div().flex_none().bg(p.color()).child(p.letter());
        let pv_badge = |p: Option<Prov>| -> AnyElement {
            match p {
                Some(p) => div()
                    .size(px(16.))
                    .flex_none()
                    .rounded(px(4.))
                    .bg(p.color())
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_family(MONO)
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(9.))
                    .text_color(white())
                    .child(p.letter())
                    .into_any_element(),
                None => icon("chart", 14., t.text_3).into_any_element(),
            }
        };
        let boxed = || div().p(px(16.)).rounded(px(12.)).border_1().border_color(t.border).bg(t.surface).min_w_0();
        let h4 = |s: SharedString| div().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child(s);
        let sub = |s: SharedString| div().mt(px(3.)).text_color(t.text_3).text_size(px(12.)).line_height(rems(1.45 * 12. / 16.)).child(s);
        let pill = |pre: Option<&str>, b: String, post: Option<&str>| {
            div()
                .flex_none()
                .flex()
                .px(px(9.))
                .py(px(3.))
                .rounded_full()
                .border_1()
                .border_color(t.border)
                .text_size(px(11.5))
                .text_color(t.text_2)
                .whitespace_nowrap()
                .when_some(pre.map(|s| s.to_string()), |d, s| d.child(s))
                .child(div().text_color(t.text).font_weight(FontWeight::SEMIBOLD).child(b))
                .when_some(post.map(|s| s.to_string()), |d, s| d.child(s))
        };

        // ── KPIs ──
        let kpi = |ic: &'static str, b: String, small: String| {
            div()
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .relative()
                .overflow_hidden()
                .flex()
                .items_center()
                .gap(px(14.))
                .px(px(18.))
                .py(px(16.))
                .rounded(px(14.))
                .border_1()
                .border_color(t.border)
                .bg(t.panel_solid)
                .child(radial_glow(mix(t.accent, 0.14, transparent_black())).absolute().right(relative(-0.3)).bottom(relative(-0.6)).w(px(160.)).h(px(120.)))
                .child(ss_ic(ic, 40., 11.))
                .child(
                    div()
                        .min_w_0()
                        .child(div().text_size(px(24.)).font_weight(FontWeight::SEMIBOLD).font_features(tnum()).child(b))
                        .child(div().mt(px(1.)).text_size(px(13.)).text_color(t.text_2).whitespace_nowrap().overflow_hidden().text_ellipsis().child(small)),
                )
        };
        let kpis = div()
            .flex()
            .gap(px(12.))
            .child(kpi("bot", fmt_int(tt.agents), "Agents spawned".into()))
            .child(kpi("clock", dur(tt.hours), "Time agents worked".into()))
            .child(kpi("pr", format!("{}", tt.prs), format!("PRs created · {} merged", tt.merged)));

        // ── section head: range + scope select ──
        let range_btn = |r: Range| {
            let on = st.range == r;
            div()
                .id(SharedString::from(format!("ss-range-{}", r.label())))
                .h(px(28.))
                .px(px(12.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .text_size(px(12.5))
                .text_color(if on { t.text } else { t.text_3 })
                .when(on, |d| d.bg(t.panel_solid).shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.2), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false }]))
                .child(r.label())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pages.stats.range = r;
                    this.pages.stats.sel_open = false;
                    cx.notify();
                }))
        };
        let menu_item = |key: &'static str, lead: AnyElement, label: String, small: Option<&'static str>| {
            let on = st.scope == key;
            div()
                .id(SharedString::from(format!("ss-scope-{key}")))
                .w_full()
                .flex()
                .items_center()
                .gap(px(9.))
                .px(px(8.))
                .py(px(7.))
                .rounded(px(7.))
                .text_size(px(13.))
                .text_color(if on { t.text } else { t.text_2 })
                .hover(|s| s.bg(t.surface_hover).text_color(t.text))
                .child(lead)
                .child(label)
                .when_some(small, |d, s| d.child(div().text_size(px(11.5)).text_color(t.text_3).child(s)))
                .when(on, |d| d.child(div().ml_auto().text_size(px(12.)).text_color(t.accent).child("✓")))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pages.stats.scope = key.into();
                    this.pages.stats.sel_open = false;
                    cx.notify();
                }))
        };
        let hd = |s: &'static str| div().px(px(8.)).pt(px(8.)).pb(px(3.)).text_size(px(10.5)).text_color(t.text_3).child(s.to_uppercase());
        let menu = div()
            .id("ss-menu")
            .occlude()
            .absolute()
            .right_0()
            .top(px(40.))
            .w(px(230.))
            .p(px(5.))
            .rounded(px(11.))
            .border_1()
            .border_color(t.border)
            .bg(t.panel_solid)
            .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.3), offset: point(px(0.), px(18.)), blur_radius: px(40.), spread_radius: px(0.), inset: false }])
            .child(menu_item("all", icon("chart", 14., t.text_3).into_any_element(), "Overview".into(), None))
            .child(hd("Providers"))
            .child(menu_item("claude", pv(Prov::Claude).into_any_element(), "Claude".into(), None))
            .child(menu_item("codex", pv(Prov::Codex).into_any_element(), "Codex".into(), None))
            .child(hd("Accounts"))
            .children(ACCTS.iter().map(|a| menu_item(a.id, pv(a.p).into_any_element(), a.label.to_string(), Some(a.plan))));
        let sel = div()
            .relative()
            .child(
                div()
                    .id("ss-sel")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(34.))
                    .pl(px(12.))
                    .pr(px(10.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.panel_solid)
                    .text_size(px(13.))
                    .child(pv_badge(pk))
                    .child(sl.clone())
                    .child(icon("chev-d", 14., t.text_3))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pages.stats.sel_open = !this.pages.stats.sel_open;
                        cx.notify();
                    })),
            )
            .when(st.sel_open, |d| {
                d.child(deferred(menu.on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.pages.stats.sel_open = false;
                    cx.notify();
                }))).with_priority(1))
            });
        let sec = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .mb(px(12.))
            .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child("Usage analytics"))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .items_center()
                    .child(div().flex().p(px(3.)).rounded(px(9.)).bg(t.surface).border_1().border_color(t.border).children([Range::D7, Range::D30, Range::All].map(range_btn)))
                    .child(sel),
            );

        // ── card head ──
        let refreshing = st.refreshing;
        let ref_icon: AnyElement = if refreshing {
            icon("refresh", 15., t.text_3)
                .with_animation("ss-ref-spin", Animation::new(Duration::from_millis(700)).repeat(), |s, p| s.with_transformation(Transformation::rotate(percentage(p))))
                .into_any_element()
        } else {
            icon("refresh", 15., t.text_3).into_any_element()
        };
        let card_h = div()
            .flex()
            .items_start()
            .justify_between()
            .gap(px(12.))
            .mb(px(16.))
            .child(
                div()
                    .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(if st.scope == "all" { "Usage overview".to_string() } else { sl.clone() }))
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .text_color(t.text_3)
                            .text_size(px(12.5))
                            .child(format!("{} account{} · Updated just now", m.n_accts, if m.n_accts == 1 { "" } else { "s" }))
                            .when_some(m.delta, |d, dl| {
                                d.child("\u{a0}·\u{a0}").child(
                                    div()
                                        .text_color(if dl >= 0. { t.green } else { t.text_2 })
                                        .child(format!("{} {}% vs previous {} days", if dl >= 0. { "▲" } else { "▼" }, dl.abs(), n)),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .id("ss-ref")
                    .size(px(30.))
                    .flex_none()
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(t.surface_hover))
                    .child(ref_icon)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.pages.stats.refreshing {
                            return;
                        }
                        this.pages.stats.refreshing = true;
                        cx.notify();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_millis(650)).await;
                            this.update(cx, |this, cx| {
                                this.pages.stats.refreshing = false;
                                cx.notify();
                            })
                            .ok();
                        })
                        .detach();
                    })),
            );

        // ── tiles ──
        let tile = |ic: &'static str, b: AnyElement, small: &'static str| {
            div()
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(12.))
                .p(px(14.))
                .rounded(px(12.))
                .border_1()
                .border_color(t.border)
                .bg(t.surface)
                .child(ss_ic(ic, 36., 10.))
                .child(div().min_w_0().child(b).child(div().text_size(px(12.)).text_color(t.text_3).whitespace_nowrap().child(small)))
        };
        let big = |s: String| div().text_size(px(20.)).font_weight(FontWeight::SEMIBOLD).font_features(tnum()).child(s);
        let tiles = div()
            .flex()
            .gap(px(10.))
            .mb(px(12.))
            .child(tile("stars", big(fmt_n(m.tot)).into_any_element(), "Total tokens"))
            .child(tile("coin", big(fmt_usd(m.cost)).into_any_element(), "Est. cost"))
            .child(tile(
                "cal",
                div()
                    .flex()
                    .items_baseline()
                    .child(big(m.active.to_string()))
                    .child(div().text_size(px(13.)).text_color(t.text_3).child(format!("\u{a0}/ {n}")))
                    .into_any_element(),
                "Active days",
            ))
            .child(tile("db", big(format!("{}%", js_round(m.cache_share * 100.))).into_any_element(), "Cache share"));

        // ── heatmap ──
        let hover = st.hover;
        let cell_sz = 22.;
        let mut cols = vec![];
        for w in 0..WEEKS {
            let mut col = div().flex().flex_col().gap(px(4.));
            for r in 0..7 {
                let i = w * 7 + r;
                let c = m.cells[i].clone();
                let hov = hover == Some(i) && !c.fut;
                let bg = if c.fut { transparent_black() } else { lvl[c.level as usize] };
                let mut cell = div().id(("hm", i)).size(px(cell_sz)).rounded(px(3.)).relative().bg(bg);
                if c.fut {
                    cell = cell.border_1().border_dashed().border_color(alpha(t.text, 0.08));
                }
                if c.level == 4 {
                    cell = cell.shadow(vec![BoxShadow { color: mix(t.accent, 0.55, transparent_black()), offset: point(px(0.), px(0.)), blur_radius: px(8.), spread_radius: px(0.), inset: false }]);
                }
                if c.best {
                    cell = cell.child(div().absolute().top(px(-3.)).left(px(-3.)).size(px(cell_sz + 6.)).rounded(px(5.)).border(px(1.5)).border_color(alpha(t.text, 0.7)));
                }
                if hov {
                    // transform: scale(1.35) + outline, drawn as an enlarged copy on top
                    let g = cell_sz * 0.35 / 2.;
                    let (a, b, cc) = c.tip.clone();
                    cell = cell.child(
                        deferred(
                            div()
                                .absolute()
                                .top(px(-g))
                                .left(px(-g))
                                .size(px(cell_sz * 1.35))
                                .rounded(px(3. * 1.35))
                                .bg(t.panel_solid)
                                .child(div().size_full().rounded(px(3. * 1.35)).bg(bg).border_1().border_color(alpha(t.text, 0.4)))
                                .child(
                                    // tooltip, centred above the cell
                                    div()
                                        .absolute()
                                        .bottom(px(cell_sz * 1.35 + 8. - g + g))
                                        .left(px(-200.))
                                        .w(px(cell_sz * 1.35 + 400.))
                                        .flex()
                                        .justify_center()
                                        .child(
                                            div()
                                                .flex()
                                                .whitespace_nowrap()
                                                .px(px(9.))
                                                .py(px(6.))
                                                .rounded(px(8.))
                                                .bg(t.panel_solid)
                                                .border_1()
                                                .border_color(t.border)
                                                .text_size(px(12.))
                                                .text_color(t.text)
                                                .shadow(vec![BoxShadow { color: hsla(0., 0., 0., 0.3), offset: point(px(0.), px(10.)), blur_radius: px(24.), spread_radius: px(0.), inset: false }])
                                                .child(div().font_weight(FontWeight::BOLD).child(a))
                                                .child(div().ml(px(6.)).text_color(t.text_3).child(if cc.is_empty() { b } else { format!("{b} · {cc}") })),
                                        ),
                                ),
                        )
                        .with_priority(2),
                    );
                }
                if !c.fut {
                    cell = cell.on_hover(cx.listener(move |this, h: &bool, _, cx| {
                        let s = &mut this.pages.stats;
                        if *h {
                            s.hover = Some(i);
                        } else if s.hover == Some(i) {
                            s.hover = None;
                        }
                        cx.notify();
                    }));
                }
                col = col.child(cell);
            }
            cols.push(col);
        }
        let leg = |c: Hsla| div().size(px(9.)).rounded(px(2.)).bg(c);
        let heat = boxed()
            .flex_1()
            .flex_basis(px(0.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_start()
                    .gap(px(10.))
                    .mb(px(14.))
                    .child(div().child(h4("Daily intensity".into())).child(sub(format!("Tokens per day, last {WEEKS} weeks").into())))
                    .child(pill(Some("Best · "), fmt_d(m.best_idx), None)),
            )
            .child(div().flex().justify_between().children(cols))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mt(px(10.))
                    .text_size(px(11.5))
                    .text_color(t.text_3)
                    .child(fmt_d(HM_START))
                    .child(div().flex().items_center().gap(px(3.)).child("Less\u{a0}").children(lvl.map(leg)).child("\u{a0}More"))
                    .child(fmt_d(DAYS as i32 - 1)),
            );

        // ── token mix ──
        let mix_box = boxed()
            .flex_1()
            .flex_basis(px(0.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_start()
                    .gap(px(10.))
                    .mb(px(14.))
                    .child(
                        div().child(h4("Token mix".into())).child(sub(
                            format!("Input, output and cache over {}", if st.range == Range::All { "all time".to_string() } else { format!("the last {n} days") }).into(),
                        )),
                    )
                    .when(m.rs > 0., |d| d.child(pill(None, fmt_n(m.rs), Some("\u{a0}reasoning")))),
            )
            .child(div().flex().h(px(12.)).rounded_full().overflow_hidden().gap(px(2.)).bg(base).mt(px(6.)).mb(px(14.)).children(m.mix.iter().map(|(_, v, c)| {
                let mut seg = div().h_full().min_w(px(3.)).flex_basis(px(0.)).bg(mix_cols[*c]);
                seg.style().flex_grow = Some(*v as f32);
                seg
            })))
            .child(div().flex().flex_col().gap(px(8.)).children(m.mix.iter().map(|(l, v, c)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(t.text_2)
                    .child(div().size(px(8.)).rounded_full().flex_none().bg(mix_cols[*c]))
                    .child(*l)
                    .child(div().ml_auto().font_weight(FontWeight::MEDIUM).text_color(t.text).font_features(tnum()).child(fmt_n(*v)))
                    .child(
                        div()
                            .w(px(38.))
                            .flex()
                            .justify_end()
                            .text_color(t.text_3)
                            .font_features(tnum())
                            .child(format!("{}%", if m.tot > 0. { js_round(v / m.tot * 100.) } else { 0. })),
                    )
            })));

        // ── bars ──
        let bar = |lead: AnyElement, name: String, v: f64, max: f64, color: Hsla| {
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .text_size(px(12.5))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(div().flex_1().min_w_0().flex().items_center().gap(px(7.)).text_color(t.text_2).child(lead).child(name))
                        .child(div().text_color(t.text).font_features(tnum()).child(fmt_n(v))),
                )
                .child(div().h(px(6.)).rounded_full().bg(base).overflow_hidden().child(div().h_full().rounded_full().w(relative((v / max) as f32)).bg(color)))
        };
        let m_max = m.models.first().map(|x| x.1).unwrap_or(1.);
        let models_box = boxed()
            .flex_1()
            .flex_basis(px(0.))
            .child(h4("Models".into()))
            .child(sub("Share of tokens by model".into()))
            .child(div().flex().flex_col().gap(px(10.)).mt(px(12.)).children(m.models.iter().map(|(name, v, p)| bar(pv(*p).into_any_element(), name.to_string(), *v, m_max, p.color()))));
        let p_max = m.projects.first().map(|x| x.1).unwrap_or(1.);
        let projects_box = boxed()
            .flex_1()
            .flex_basis(px(0.))
            .child(h4("Projects".into()))
            .child(sub("Where the tokens went".into()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .mt(px(12.))
                    .children(m.projects.iter().map(|(name, v, ic)| bar(div().size(px(16.)).flex_none().child(pix(ic, 16.)).into_any_element(), name.clone(), *v, p_max, t.accent))),
            );

        // ── fun ──
        let fun = |em: &'static str, b: String, small: String| {
            boxed()
                .flex_1()
                .flex_basis(px(0.))
                .flex()
                .gap(px(12.))
                .items_center()
                .child(div().w(px(36.)).flex_none().flex().justify_center().text_size(px(22.)).child(em))
                .child(div().child(div().text_size(px(14.)).font_weight(FontWeight::BOLD).child(b)).child(div().mt(px(1.)).text_size(px(12.)).text_color(t.text_3).child(small)))
        };
        let arms = (js_round(m.peak / 6.)).max(3.).min(8.);
        let funs = div()
            .flex()
            .gap(px(12.))
            .mt(px(12.))
            .child(fun("🔥", format!("{}-day streak", m.cur), format!("Longest: {} days", m.longest)))
            .child(fun("🐙", format!("{arms} arms at once"), "Most agents running together".into()))
            .child(fun(
                "💸",
                format!("{} saved", fmt_usd(m.cost * m.cache_share * 8.5)),
                format!("by cache hits {}", if st.range == Range::All { "overall".to_string() } else { format!("in {n} days") }),
            ));

        let card = div()
            .border_1()
            .border_color(t.border)
            .rounded(px(16.))
            .bg(t.panel_solid)
            .p(px(20.))
            .child(card_h)
            .child(tiles)
            .child(div().flex().gap(px(12.)).child(heat).child(mix_box))
            .child(div().flex().gap(px(12.)).mt(px(12.)).child(models_box).child(projects_box))
            .child(funs)
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(t.text_3)
                    .child(icon("shield", 13., t.text_3))
                    .child("Costs are estimates from public API prices. Plan usage isn't billed per token."),
            );

        let wrap = div()
            .w_full()
            .max_w(px(1040.))
            .px(px(40.))
            .pt(px(32.))
            .pb(px(72.))
            .child(
                div()
                    .mb(px(22.))
                    .child(div().text_size(px(28.)).font_weight(FontWeight::SEMIBOLD).line_height(px(34.)).child("Stats"))
                    .child(div().mt(px(6.)).text_color(t.text_2).child("What your agents got done across every Claude and Codex account.")),
            )
            .child(kpis)
            .child(div().mt(px(12.)).mx(px(4.)).mb(px(30.)).text_color(t.text_3).text_size(px(12.5)).child("Tracking since Aug 31, 2026"))
            .child(sec)
            .child(card);

        div().size_full().pt(px(44.)).child(
            div()
                .id("stats-page")
                .size_full()
                .border_t_1()
                .border_l_1()
                .border_color(t.border)
                .rounded_tl(px(12.))
                .bg(t.bg)
                .overflow_y_scroll()
                .track_scroll(&self.pages.stats.scroll)
                .on_scroll(cx.listener(|this, _, _, cx| {
                    if this.pages.stats.hover.take().is_some() {
                        cx.notify();
                    }
                }))
                .child(div().w_full().flex().justify_center().child(wrap)),
        )
    }
}

/// Scripted actions for this module (TAKO_ACT "key:value"); return true if handled.
///   stats            open the page
///   stats:7d|30d|all range
///   stats:overview|claude|codex|c1|c2|x1|x2  scope
///   stats:menu       toggle the scope dropdown
///   stats:hover:N    hover heatmap cell N (0..83, column-major)
///   stats:scroll:Y   scroll the page to Y px
pub fn act(app: &mut TakoApp, k: &str, v: &str, _window: &mut Window, cx: &mut Context<TakoApp>) -> bool {
    if k != "stats" {
        return false;
    }
    let s = &mut app.pages.stats;
    match v {
        "" => app.open_stats(cx),
        "7d" => s.range = Range::D7,
        "30d" => s.range = Range::D30,
        "all" => s.range = Range::All,
        "overview" => s.scope = "all".into(),
        "claude" | "codex" | "c1" | "c2" | "x1" | "x2" => s.scope = v.into(),
        "menu" => s.sel_open = !s.sel_open,
        _ => {
            if let Some(n) = v.strip_prefix("hover:") {
                s.hover = n.parse().ok();
            } else if let Some(y) = v.strip_prefix("scroll:") {
                let y: f32 = y.parse().unwrap_or(0.);
                s.scroll.set_offset(point(px(0.), px(-y)));
            } else {
                return false;
            }
        }
    }
    cx.notify();
    true
}
