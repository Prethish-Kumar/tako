#!/bin/zsh
# Drive the app with real mouse clicks and keystrokes (through GPUI's own hit-testing and
# key dispatch) and print the resulting state after each step. Window is 1440×900.
cd "${0:A:h}/.."
cargo build -q 2>&1 | grep -E "^error" -A6
A=""
step() { A="$A;$1;wait:250;draw;dump:$2"; }
step "click:26,114" "rail chats"
step "click:590,22" "tab Changes (worktree 1)"
step "key:cmd-[" "cmd-[ back to Review"
step "click:200,253" "sidebar row 2"
step "click:26,72" "rail home"
step "click:26,114" "rail chats again"
step "click:323,161;wait:700" "fold tako"
step "click:323,161;wait:700" "unfold tako"
step "key:cmd-b" "cmd-b sidebar"
step "key:cmd-b" "cmd-b again"
step "click:567,862" "chip machine"
step "key:escape" "escape"
step "click:26,156" "rail automations"
step "click:26,198" "rail plugins"
step "click:26,875" "avatar"
step "click:26,794" "rail settings"
step "key:cmd-n" "cmd-n home"
TAKO_SNAP="${1:-/tmp/smoke.png}" TAKO_VIEW=home TAKO_ACT="${A#;}" TAKO_SNAP_MS=300 ./target/debug/tako 2>&1 | grep -E "STATE|panic"
