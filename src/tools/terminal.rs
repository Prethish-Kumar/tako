//! Terminal tab: a real shell (see `pty.rs`). The mock's fake shell is gone.

use super::{pty::TermView, st};
use crate::{app::TakoApp, chat::Tab};
use gpui_kit::*;

impl TakoApp {
    pub(super) fn ensure_term(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) -> Entity<TermView> {
        if let Some(v) = st(|s| s.pty.get(&id).cloned()) {
            return v;
        }
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| "/".into());
        let v = cx.new(|cx| TermView::new(home, window, cx));
        st(|s| s.pty.insert(id, v.clone()));
        v
    }

    /// Type a command into the tab's shell (scripted `term:` action).
    pub(super) fn term_run(&mut self, id: usize, cmd: String, window: &mut Window, cx: &mut Context<Self>) {
        let v = self.ensure_term(id, window, cx);
        v.read(cx).run(&cmd);
    }

    pub(super) fn render_terminal(&mut self, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Transparent: the chat pane's blurred backdrop (bleed) shows through, as in the chat.
        let v = self.ensure_term(tab.id, window, cx);
        // switching to this tab puts the keyboard in the shell
        if st(|s| s.pty_active.replace(tab.id)) != Some(tab.id) {
            let fh = v.read(cx).focus_handle(cx);
            window.focus(&fh, cx);
        }
        div().flex_1().min_w_0().h_full().child(v)
    }
}
