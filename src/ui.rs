//! Small shared building blocks.

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub const FONT: &str = "Inter";
pub const MONO: &str = "JetBrains Mono";
pub const SERIF: &str = "Instrument Serif";
pub const PIXEL: &str = "Silkscreen";

/// One of the mock's sprite icons (assets/icons/<name>.svg), tinted with `color`.
pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg().path(SharedString::from(format!("icons/{name}.svg"))).size(px(size)).flex_none().text_color(color)
}

/// Inline text with `code` spans (backticks) rendered in the mono face on a chip.
pub fn rich_text(text: &str, color: Hsla, code_bg: Hsla) -> StyledText {
    let mut s = String::new();
    let mut runs: Vec<TextRun> = vec![];
    let mut code = false;
    let mut start = 0;
    let push = |runs: &mut Vec<TextRun>, len: usize, code: bool| {
        if len == 0 {
            return;
        }
        runs.push(TextRun {
            len,
            font: font(if code { MONO } else { FONT }),
            color,
            background_color: if code { Some(code_bg) } else { None },
            underline: None,
            strikethrough: None,
        });
    };
    for ch in text.chars() {
        if ch == '`' {
            push(&mut runs, s.len() - start, code);
            code = !code;
            start = s.len();
            // a little air around code chips, like the CSS padding
            s.push('\u{2009}');
            continue;
        }
        s.push(ch);
    }
    push(&mut runs, s.len() - start, code);
    StyledText::new(s).with_runs(runs)
}

/// Ease-out cubic.
pub fn ease_out(p: f32) -> f32 {
    1. - (1. - p.clamp(0., 1.)).powi(3)
}

/// The mock's cubic-bezier(.2,.8,.2,1), approximated by solving the curve.
pub fn ease_snappy(p: f32) -> f32 {
    let p = p.clamp(0., 1.);
    // Solve x(t) = p for the bezier with control points (.2,.8),(.2,1)
    let (x1, y1, x2, y2) = (0.2f32, 0.8f32, 0.2f32, 1.0f32);
    let bez = |t: f32, a: f32, b: f32| 3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * b + t * t * t;
    let mut lo = 0f32;
    let mut hi = 1f32;
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if bez(mid, x1, x2) < p {
            lo = mid
        } else {
            hi = mid
        }
    }
    bez((lo + hi) / 2., y1, y2)
}

/// The mock's spinner: a 1.6px ring with a transparent top quarter, turning every 0.9s.
pub fn spinner(id: impl Into<ElementId>, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path("icons/_spinner.svg")
        .size(px(size))
        .flex_none()
        .text_color(color)
        .with_animation(id, Animation::new(std::time::Duration::from_millis(900)).repeat(), |s, p| {
            s.with_transformation(Transformation::rotate(percentage(p)))
        })
}

/// An invisible layer that records its parent's laid-out height, so height
/// animations can open to the content's natural size.
pub fn measurer(cell: std::rc::Rc<std::cell::Cell<f32>>) -> impl IntoElement {
    canvas(move |b, _, _| cell.set(f32::from(b.size.height)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

/// A height tween: 0→1 progress with an easing, started at `start`.
#[derive(Clone, Debug)]
pub struct Tween {
    pub start: std::time::Instant,
    pub ms: f32,
    pub delay_ms: f32,
}
impl Tween {
    pub fn new(ms: f32) -> Self {
        Tween { start: std::time::Instant::now(), ms, delay_ms: 0. }
    }
    pub fn delayed(ms: f32, delay_ms: f32) -> Self {
        Tween { start: std::time::Instant::now(), ms, delay_ms }
    }
    /// Milliseconds since start (effectively "forever" under Reduce motion).
    pub fn ms(&self) -> f32 {
        if reduced_motion() {
            return 1e9;
        }
        self.start.elapsed().as_secs_f32() * 1000.
    }
    pub fn raw(&self) -> f32 {
        if reduced_motion() {
            return 1.;
        }
        ((self.start.elapsed().as_secs_f32() * 1000. - self.delay_ms) / self.ms).clamp(0., 1.)
    }
    pub fn done(&self) -> bool {
        reduced_motion() || self.start.elapsed().as_secs_f32() * 1000. >= self.ms + self.delay_ms
    }
}

/// A paragraph with `code` spans as real chips (smaller mono, rounded, padded).
/// GPUI text runs can't change size or add padding, so words become inline
/// flex items that wrap like text.
pub fn rich_par(text: &str, color: Hsla, t: crate::theme::Theme, size: f32, lh: f32) -> Div {
    let mut d = div().flex().flex_wrap().items_baseline().text_size(px(size)).text_color(color).line_height(px(size * lh));
    let mut code = false;
    for (i, seg) in text.split('`').enumerate() {
        if i > 0 {
            code = !code;
        }
        if seg.is_empty() {
            continue;
        }
        if code {
            d = d.child(
                div()
                    .font_family(MONO)
                    .text_size(px(12.5))
                    .px(px(6.))
                    .mx(px(2.))
                    .rounded(px(5.))
                    .bg(t.surface_active)
                    .line_height(px(size * 1.45))
                    .child(seg.to_string()),
            );
        } else {
            // keep spaces attached to the preceding word so wrapping matches text
            let mut words: Vec<String> = vec![];
            let mut cur = String::new();
            for ch in seg.chars() {
                cur.push(ch);
                if ch == ' ' {
                    words.push(std::mem::take(&mut cur));
                }
            }
            if !cur.is_empty() {
                words.push(cur);
            }
            for w in words {
                d = d.child(div().whitespace_nowrap().child(w));
            }
        }
    }
    d
}

/// macOS "Reduce motion" (System Settings → Accessibility → Display), or TAKO_REDUCE_MOTION=1.
/// The mock honours prefers-reduced-motion: tweens jump to the end and scenes hold still.
pub fn reduced_motion() -> bool {
    static R: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *R.get_or_init(|| {
        if std::env::var("TAKO_REDUCE_MOTION").is_ok() {
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("defaults")
                .args(["read", "com.apple.universalaccess", "reduceMotion"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    })
}

/// CSS `letter-spacing` for short labels: GPUI text has no tracking, so each
/// character is its own inline item with `em` × font-size after it.
pub fn tracked(text: &str, size: f32, em: f32) -> Div {
    let gap = size * em;
    let n = text.chars().count();
    div().flex().flex_none().children(text.chars().enumerate().map(move |(i, c)| {
        let d = div().child(if c == ' ' { "\u{00a0}".to_string() } else { c.to_string() });
        if i + 1 < n { d.mr(px(gap)) } else { d }
    }))
}
