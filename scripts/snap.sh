#!/bin/zsh
# Render the app offscreen to a PNG (no window is shown).
# usage: scripts/snap.sh out.png [VIEW] ["ACTIONS"] [ms]
#   VIEW: home | chats | automations | plugins | settings | stats
#   ACTIONS: ';'-separated TAKO_ACT script, e.g. "open:2;tab:2;wait:400;theme"
cd "${0:A:h}/.."
cargo build -q 2>&1 | grep -E "^error" -A6 | head -40
TAKO_SNAP="${1:A}" TAKO_VIEW="${2:-home}" TAKO_ACT="$3" TAKO_SNAP_MS="${4:-1200}" ./target/debug/tako 2>&1 | grep -E "snap|panic|error" | tail -5
