#!/bin/zsh
# Merge an agent branch; shared core files keep ours, append-only files take both sides.
cd "${0:A:h}/.."
b=$1
git merge --no-edit $b >/dev/null 2>&1
for f in $(git diff --name-only --diff-filter=U); do
  case $f in
    src/snap.rs|src/chat.rs|src/app.rs|src/ui.rs) git checkout --ours $f ;;
    *) python3 - "$f" <<'E'
import sys,re
p=sys.argv[1]; s=open(p).read()
s=re.sub(r"<<<<<<< [^\n]*\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n", lambda m: m.group(1)+m.group(2), s, flags=re.S)
open(p,"w").write(s)
E
    ;;
  esac
  git add $f; echo "resolved $f"
done
git commit -qm "Merge $b

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" 2>/dev/null
echo "merged $b"
