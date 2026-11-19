//! A real terminal: the user's shell on a PTY, parsed by `alacritty_terminal` (the same
//! VT engine Zed uses), painted cell-by-cell on a GPUI canvas. Its own view, so output
//! redraws don't re-render the app.

use crate::{theme::ActiveTheme, ui::MONO};
use alacritty_terminal::{
    event::{Event, EventListener, WindowSize},
    event_loop::{EventLoop, EventLoopSender, Msg},
    grid::{Dimensions, Scroll},
    sync::FairMutex,
    term::{Config, Term, TermMode, cell::Flags},
    tty,
    vte::ansi::{Color, CursorShape, NamedColor, Rgb},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::{
    borrow::Cow,
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

actions!(tako_term, [TermEscape]);

/// Escape belongs to the shell (vim, less…) while the terminal has focus — the app binds
/// it to Dismiss globally, and bindings are matched before key listeners.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", TermEscape, Some("Terminal"))]);
}

const FONT_SIZE: f32 = 12.5;
const LINE_H: f32 = 1.45;
const PAD: (f32, f32) = (16., 12.);

#[derive(Clone)]
struct Listener {
    dirty: Arc<AtomicBool>,
    inbox: Arc<Mutex<Vec<Event>>>,
}

impl EventListener for Listener {
    fn send_event(&self, ev: Event) {
        match ev {
            Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {}
            ev => self.inbox.lock().unwrap().push(ev),
        }
        self.dirty.store(true, Ordering::Release);
    }
}

struct Size {
    cols: usize,
    rows: usize,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

struct Session {
    term: Arc<FairMutex<Term<Listener>>>,
    tx: EventLoopSender,
}

impl Drop for Session {
    // closing the tab hangs up the shell (the PTY's drop sends SIGHUP)
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
    }
}

pub struct TermView {
    s: Option<Session>,
    listener: Listener,
    focus: FocusHandle,
    /// (cols, rows) the PTY was last sized to; updated from layout.
    grid: Rc<Cell<(usize, usize)>>,
    cell: (f32, f32),
    exited: bool,
    scroll_acc: f32,
    cwd: std::path::PathBuf,
    pub title: String,
}

impl TermView {
    pub fn new(cwd: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let listener = Listener { dirty: Arc::default(), inbox: Arc::default() };
        // monospace cell from the font's advance
        let fs = px(FONT_SIZE);
        let ts = window.text_system();
        let fid = ts.resolve_font(&font(MONO));
        let cw = ts.advance(fid, fs, 'm').map(|s| f32::from(s.width)).unwrap_or(FONT_SIZE * 0.6);
        let ch = (FONT_SIZE * LINE_H).round();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut v = TermView { s: None, listener, focus, grid: Rc::new(Cell::new((80, 24))), cell: (cw, ch), exited: false, scroll_acc: 0., cwd, title: String::new() };
        v.spawn();
        // Pump: the PTY thread flags new output; redraw at most once per frame-ish tick.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(8)).await;
                let Ok(alive) = this.update(cx, |v, cx| {
                    if v.listener.dirty.swap(false, Ordering::AcqRel) {
                        v.drain(cx);
                        cx.notify();
                    }
                }) else { break };
                let _ = alive;
            }
        })
        .detach();
        v
    }

    fn spawn(&mut self) {
        let (cols, rows) = self.grid.get();
        let term = Arc::new(FairMutex::new(Term::new(Config::default(), &Size { cols, rows }, self.listener.clone())));
        let mut env = HashMap::new();
        env.insert("TERM".into(), "xterm-256color".into());
        env.insert("COLORTERM".into(), "truecolor".into());
        env.insert("TERM_PROGRAM".into(), "tako".into());
        let opts = tty::Options { shell: None, working_directory: Some(self.cwd.clone()), drain_on_exit: true, env, ..Default::default() };
        let ws = self.wsize(cols, rows);
        let Ok(pty) = tty::new(&opts, ws, 0) else {
            self.exited = true;
            return;
        };
        let Ok(el) = EventLoop::new(term.clone(), self.listener.clone(), pty, true, false) else {
            self.exited = true;
            return;
        };
        let tx = el.channel();
        el.spawn();
        self.s = Some(Session { term, tx });
        self.exited = false;
    }

    fn wsize(&self, cols: usize, rows: usize) -> WindowSize {
        WindowSize { num_lines: rows as u16, num_cols: cols as u16, cell_width: self.cell.0 as u16, cell_height: self.cell.1 as u16 }
    }

    /// Type a line into the shell.
    pub fn run(&self, cmd: &str) {
        self.write(format!("{cmd}\r").into_bytes());
    }

    fn write(&self, b: impl Into<Cow<'static, [u8]>>) {
        if let Some(s) = &self.s {
            let _ = s.tx.send(Msg::Input(b.into()));
        }
    }

    fn drain(&mut self, cx: &mut Context<Self>) {
        let evs = std::mem::take(&mut *self.listener.inbox.lock().unwrap());
        for ev in evs {
            match ev {
                Event::PtyWrite(s) => self.write(s.into_bytes()),
                Event::Title(t) => self.title = t,
                Event::ResetTitle => self.title.clear(),
                Event::ClipboardStore(_, s) => cx.write_to_clipboard(ClipboardItem::new_string(s)),
                Event::ClipboardLoad(_, f) => {
                    let s = cx.read_from_clipboard().and_then(|c| c.text()).unwrap_or_default();
                    self.write(f(&s).into_bytes());
                }
                Event::TextAreaSizeRequest(f) => {
                    let (c, r) = self.grid.get();
                    self.write(f(self.wsize(c, r)).into_bytes());
                }
                Event::ColorRequest(i, f) => {
                    let c = palette(i, &cx.t());
                    let r = Rgba::from(c);
                    self.write(f(Rgb { r: (r.r * 255.) as u8, g: (r.g * 255.) as u8, b: (r.b * 255.) as u8 }).into_bytes());
                }
                Event::Exit | Event::ChildExit(_) => {
                    self.exited = true;
                    self.s = None;
                }
                _ => {}
            }
        }
    }

    fn mode(&self) -> TermMode {
        self.s.as_ref().map(|s| *s.term.lock().mode()).unwrap_or(TermMode::empty())
    }

    fn key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        if self.exited {
            if ks.key == "enter" {
                self.spawn();
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if m.platform {
            match ks.key.as_str() {
                "v" => {
                    if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                        let t = t.replace("\r\n", "\r").replace('\n', "\r");
                        if self.mode().contains(TermMode::BRACKETED_PASTE) {
                            self.write(format!("\x1b[200~{t}\x1b[201~").into_bytes());
                        } else {
                            self.write(t.into_bytes());
                        }
                        cx.stop_propagation();
                    }
                }
                "k" => {
                    // clear screen + scrollback, like Terminal.app
                    if let Some(s) = &self.s {
                        s.term.lock().grid_mut().clear_history();
                    }
                    self.write(&b"\x0c"[..]);
                    cx.stop_propagation();
                }
                _ => {} // ⌘-shortcuts belong to the app
            }
            return;
        }
        let Some(bytes) = key_bytes(ks, self.mode()) else { return };
        if let Some(s) = &self.s {
            s.term.lock().scroll_display(Scroll::Bottom);
        }
        self.write(bytes);
        cx.stop_propagation();
    }

    fn scroll(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = &self.s else { return };
        let dy = f32::from(ev.delta.pixel_delta(px(self.cell.1)).y);
        self.scroll_acc += dy / self.cell.1;
        let n = self.scroll_acc.trunc() as i32;
        if n == 0 {
            return;
        }
        self.scroll_acc -= n as f32;
        let mode = *s.term.lock().mode();
        if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
            // full-screen apps (less, vim) get arrow keys
            let k: &[u8] = if n > 0 { b"\x1bOA" } else { b"\x1bOB" };
            for _ in 0..n.abs() {
                self.write(k.to_vec());
            }
        } else {
            s.term.lock().scroll_display(Scroll::Delta(n));
        }
        cx.notify();
    }
}

impl TermView {
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }
}

impl Focusable for TermView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Bytes a key sends (xterm conventions). None for keys the terminal doesn't consume.
fn key_bytes(ks: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    let m = ks.modifiers;
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let modn = 1 + (m.shift as u8) + 2 * (m.alt as u8) + 4 * (m.control as u8);
    let csi = |c: char| -> Vec<u8> {
        if modn > 1 {
            format!("\x1b[1;{modn}{c}").into_bytes()
        } else if app_cursor {
            format!("\x1bO{c}").into_bytes()
        } else {
            format!("\x1b[{c}").into_bytes()
        }
    };
    let tilde = |n: u8| -> Vec<u8> { if modn > 1 { format!("\x1b[{n};{modn}~").into_bytes() } else { format!("\x1b[{n}~").into_bytes() } };
    let esc = |mut v: Vec<u8>| -> Vec<u8> {
        if m.alt {
            v.insert(0, 0x1b);
        }
        v
    };
    Some(match ks.key.as_str() {
        "enter" => esc(b"\r".to_vec()),
        "backspace" => esc(if m.control { vec![0x08] } else { vec![0x7f] }),
        "tab" => if m.shift { b"\x1b[Z".to_vec() } else { b"\t".to_vec() },
        "escape" => b"\x1b".to_vec(),
        "up" => csi('A'),
        "down" => csi('B'),
        "right" => csi('C'),
        "left" => csi('D'),
        "home" => csi('H'),
        "end" => csi('F'),
        "pageup" => tilde(5),
        "pagedown" => tilde(6),
        "delete" => tilde(3),
        "insert" => tilde(2),
        "f1" => b"\x1bOP".to_vec(),
        "f2" => b"\x1bOQ".to_vec(),
        "f3" => b"\x1bOR".to_vec(),
        "f4" => b"\x1bOS".to_vec(),
        "f5" => tilde(15),
        "f6" => tilde(17),
        "f7" => tilde(18),
        "f8" => tilde(19),
        "f9" => tilde(20),
        "f10" => tilde(21),
        "f11" => tilde(23),
        "f12" => tilde(24),
        "space" if m.control => vec![0],
        k if m.control && k.chars().count() == 1 => {
            let c = k.chars().next()?.to_ascii_lowercase();
            let b = match c {
                'a'..='z' => c as u8 - b'a' + 1,
                '[' | '3' => 0x1b,
                '\\' | '4' => 0x1c,
                ']' | '5' => 0x1d,
                '6' => 0x1e,
                '-' | '/' | '7' => 0x1f,
                '2' | '@' => 0,
                _ => return None,
            };
            esc(vec![b])
        }
        // Option acts as Meta (ESC prefix) on the unshifted key, like most terminal configs.
        k if m.alt && k.chars().count() == 1 => {
            let s = if m.shift { k.to_uppercase() } else { k.to_string() };
            esc(s.into_bytes())
        }
        "space" => b" ".to_vec(),
        _ => ks.key_char.as_ref()?.as_bytes().to_vec(),
    })
}

fn rgbh(r: u8, g: u8, b: u8) -> Hsla {
    Rgba { r: r as f32 / 255., g: g as f32 / 255., b: b as f32 / 255., a: 1. }.into()
}

/// The 16 ANSI colours follow the theme; 16–255 are the xterm cube + greys.
fn palette(i: usize, t: &crate::theme::Theme) -> Hsla {
    let base = [
        rgbh(0x1d, 0x1f, 0x21),
        t.red,
        t.green,
        t.yellow,
        t.blue,
        rgbh(0xc6, 0x78, 0xdd),
        rgbh(0x56, 0xb6, 0xc2),
        rgbh(0xd0, 0xd0, 0xd0),
    ];
    match i {
        0..=7 => base[i],
        8 => rgbh(0x6b, 0x6f, 0x76),
        9..=14 => {
            let c = base[i - 8];
            hsla(c.h, c.s, (c.l + 0.1).min(0.9), 1.)
        }
        15 => rgbh(0xf5, 0xf5, 0xf5),
        16..=231 => {
            let n = i - 16;
            let v = |x: usize| if x == 0 { 0 } else { (55 + 40 * x) as u8 };
            rgbh(v(n / 36), v((n / 6) % 6), v(n % 6))
        }
        232..=255 => {
            let g = (8 + 10 * (i - 232)) as u8;
            rgbh(g, g, g)
        }
        _ => t.text,
    }
}

impl Render for TermView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.t();
        let fg0 = t.text;
        let (cw, ch) = self.cell;
        let focused = self.focus.is_focused(window);
        let term = self.s.as_ref().map(|s| s.term.clone());
        let tx = self.s.as_ref().map(|s| s.tx.clone());
        let grid = self.grid.clone();
        let exited = self.exited;
        let ws_cell = (cw as u16, ch as u16);
        let tt = t.clone();
        let accent = t.accent;
        let text3 = t.text_3;

        let surface = canvas(
            // prepaint: size the PTY to the laid-out area
            move |b, _, _| {
                let cols = ((f32::from(b.size.width) / cw).floor() as usize).max(2);
                let rows = ((f32::from(b.size.height) / ch).floor() as usize).max(1);
                if grid.get() != (cols, rows) {
                    grid.set((cols, rows));
                    if let (Some(term), Some(tx)) = (&term, &tx) {
                        term.lock().resize(Size { cols, rows });
                        let _ = tx.send(Msg::Resize(WindowSize { num_lines: rows as u16, num_cols: cols as u16, cell_width: ws_cell.0, cell_height: ws_cell.1 }));
                    }
                }
                term
            },
            move |b, term, window, cx| {
                let Some(term) = term else { return };
                let term = term.lock();
                let content = term.renderable_content();
                let colors = content.colors;
                let off = content.display_offset as i32;
                let resolve = |c: Color, fg: bool, flags: Flags| -> Option<Hsla> {
                    match c {
                        Color::Spec(r) => Some(rgbh(r.r, r.g, r.b)),
                        Color::Indexed(i) => Some(colors[i as usize].map(|r| rgbh(r.r, r.g, r.b)).unwrap_or_else(|| palette(i as usize, &tt))),
                        Color::Named(n) => {
                            let i = n as usize;
                            if let Some(r) = colors[i] {
                                return Some(rgbh(r.r, r.g, r.b));
                            }
                            match n {
                                NamedColor::Foreground | NamedColor::BrightForeground => Some(fg0),
                                NamedColor::Background => if fg { Some(fg0) } else { None },
                                NamedColor::DimForeground => Some(fg0.opacity(0.66)),
                                _ if i < 8 && fg && flags.contains(Flags::BOLD) => Some(palette(i + 8, &tt)),
                                _ if i < 16 => Some(palette(i, &tt)),
                                _ if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize).contains(&i) => {
                                    Some(palette(i - NamedColor::DimBlack as usize, &tt).opacity(0.66))
                                }
                                _ => Some(fg0),
                            }
                        }
                    }
                };
                let o = b.origin;
                // rows of (col, char, fg, bg, flags)
                let mut rows: Vec<Vec<(usize, char, Hsla, Option<Hsla>, Flags)>> = vec![];
                for ic in content.display_iter {
                    let row = (ic.point.line.0 + off) as usize;
                    if rows.len() <= row {
                        rows.resize_with(row + 1, Vec::new);
                    }
                    let cell = &ic.cell;
                    if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                        continue;
                    }
                    let (mut fg, mut bg) = (resolve(cell.fg, true, cell.flags).unwrap_or(fg0), resolve(cell.bg, false, cell.flags));
                    if cell.flags.contains(Flags::INVERSE) {
                        let f = fg;
                        fg = bg.unwrap_or(rgbh(0x10, 0x10, 0x12));
                        bg = Some(f);
                    }
                    if cell.flags.contains(Flags::DIM) {
                        fg = fg.opacity(0.66);
                    }
                    if cell.flags.contains(Flags::HIDDEN) {
                        fg = fg.opacity(0.);
                    }
                    rows[row].push((ic.point.column.0, cell.c, fg, bg, cell.flags));
                }
                // backgrounds
                for (r, cells) in rows.iter().enumerate() {
                    for (col, _, _, bg, fl) in cells {
                        if let Some(bg) = bg {
                            let w = if fl.contains(Flags::WIDE_CHAR) { 2. } else { 1. };
                            window.paint_quad(fill(Bounds::new(point(o.x + px(*col as f32 * cw), o.y + px(r as f32 * ch)), size(px(cw * w + 0.5), px(ch))), *bg));
                        }
                    }
                }
                // text: one shaped run per same-style span, placed at its column so
                // fallback glyphs can't push the rest of the line off the grid
                let fs = px(FONT_SIZE);
                for (r, cells) in rows.iter().enumerate() {
                    let y = o.y + px(r as f32 * ch);
                    let mut i = 0;
                    while i < cells.len() {
                        let (c0, _, fg, _, fl) = cells[i];
                        let key = (fg, fl & (Flags::BOLD | Flags::ITALIC | Flags::UNDERLINE | Flags::STRIKEOUT));
                        let mut s = String::new();
                        let mut j = i;
                        let mut col = c0;
                        while j < cells.len() {
                            let (cc, chr, f2, _, fl2) = cells[j];
                            let wide = fl2.contains(Flags::WIDE_CHAR);
                            if j > i && (cc != col || (f2, fl2 & (Flags::BOLD | Flags::ITALIC | Flags::UNDERLINE | Flags::STRIKEOUT)) != key || wide) {
                                break;
                            }
                            s.push(if chr == '\0' { ' ' } else { chr });
                            col = cc + if wide { 2 } else { 1 };
                            j += 1;
                            if wide {
                                break; // wide glyphs get their own run, pinned to their column
                            }
                        }
                        i = j;
                        if s.trim().is_empty() && !key.1.intersects(Flags::UNDERLINE | Flags::STRIKEOUT) {
                            continue;
                        }
                        let mut f = font(MONO);
                        if key.1.contains(Flags::BOLD) {
                            f.weight = FontWeight::BOLD;
                        }
                        if key.1.contains(Flags::ITALIC) {
                            f.style = FontStyle::Italic;
                        }
                        let run = TextRun {
                            len: s.len(),
                            font: f,
                            color: fg,
                            background_color: None,
                            underline: key.1.contains(Flags::UNDERLINE).then(|| UnderlineStyle { thickness: px(1.), color: Some(fg), wavy: false }),
                            strikethrough: key.1.contains(Flags::STRIKEOUT).then(|| StrikethroughStyle { thickness: px(1.), color: Some(fg) }),
                        };
                        let line = window.text_system().shape_line(s.into(), fs, &[run], None);
                        let _ = line.paint(point(o.x + px(c0 as f32 * cw), y), px(ch), TextAlign::Left, None, window, cx);
                    }
                }
                // cursor
                let cur = content.cursor;
                if content.mode.contains(TermMode::SHOW_CURSOR) && cur.shape != CursorShape::Hidden {
                    let row = cur.point.line.0 + off;
                    if row >= 0 {
                        let cb = Bounds::new(point(o.x + px(cur.point.column.0 as f32 * cw), o.y + px(row as f32 * ch)), size(px(cw), px(ch)));
                        if !focused {
                            window.paint_quad(outline(cb, accent.opacity(0.7), BorderStyle::Solid));
                        } else {
                            match cur.shape {
                                CursorShape::Beam => window.paint_quad(fill(Bounds::new(cb.origin, size(px(2.), cb.size.height)), accent)),
                                CursorShape::Underline => window.paint_quad(fill(Bounds::new(point(cb.origin.x, cb.bottom() - px(2.)), size(cb.size.width, px(2.))), accent)),
                                _ => window.paint_quad(fill(cb, accent.opacity(0.55))),
                            }
                        }
                    }
                }
            },
        )
        .size_full();

        div()
            .id("pty")
            .key_context("Terminal")
            .track_focus(&self.focus)
            .size_full()
            .relative()
            .px(px(PAD.0))
            .py(px(PAD.1))
            .cursor_text()
            .on_key_down(cx.listener(Self::key))
            .on_action(cx.listener(|this, _: &TermEscape, _, _| this.write(&b"\x1b"[..])))
            .on_scroll_wheel(cx.listener(Self::scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                window.focus(&this.focus, cx);
                cx.notify();
            }))
            .child(surface)
            .when(exited, |d| {
                d.child(
                    div()
                        .absolute()
                        .bottom(px(12.))
                        .left(px(PAD.0))
                        .font_family(MONO)
                        .text_size(px(12.))
                        .text_color(text3)
                        .child("[process exited — press Enter to start a new shell]"),
                )
            })
    }
}
