#!/usr/bin/env bash
# Publish ./wiki to the GitHub Wiki remote (M1.wiki.git).
# Requires: Wiki enabled on the repo (public repo, or GitHub Pro for private).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WIKI_REMOTE="${WIKI_REMOTE:-https://github.com/anutrk111/M1.wiki.git}"
TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

git clone "$WIKI_REMOTE" "$TMP" || {
  echo "Clone failed. Enable Wiki in repo Settings, or make the repo public / upgrade to Pro." >&2
  exit 1
}

rsync -a --delete --exclude .git "$ROOT/wiki/" "$TMP/"
cd "$TMP"
git add -A
if git diff --cached --quiet; then
  echo "Wiki already up to date."
  exit 0
fi
git commit -m "Sync wiki from repository wiki/ folder"
git push origin HEAD
echo "Published to $WIKI_REMOTE"
