#!/usr/bin/env bash
# usage: scripts/release.sh            (asks for the version)
#        scripts/release.sh 0.1.0-prealpha.3
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

ask_number() {
  local v
  while true; do
    read -rp "$1: " v
    if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; return; fi
    echo "  Please type a whole number." >&2
  done
}

# ---------- version ----------
if [ $# -ge 1 ]; then
  VERSION="$1"
else
  MAJOR=$(ask_number "Major")
  MINOR=$(ask_number "Minor")
  PATCH=$(ask_number "Patch")
  while true; do
    read -rp "Level (prealpha / alpha / beta / rc / none): " LEVEL
    case "$LEVEL" in prealpha|alpha|beta|rc|none) break ;; esac
    echo "  Pick one of: prealpha alpha beta rc none" >&2
  done
  VERSION="$MAJOR.$MINOR.$PATCH"
  if [ "$LEVEL" != "none" ]; then
    LEVELNUM=$(ask_number "Level patch")
    VERSION="$VERSION-$LEVEL.$LEVELNUM"
  fi
fi
TAG="v$VERSION"

# ---------- checks ----------
if ! grep -qF "## [$VERSION]" CHANGELOG.md; then
  echo "CHANGELOG.md has no heading '## [$VERSION] - date'. Add it first."
  exit 1
fi
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "Tag $TAG already exists on this computer."; exit 1
fi
if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
  echo "Tag $TAG already exists on GitHub."; exit 1
fi

# ---------- show what will be committed ----------
git add -A
echo
echo "Files that will be committed:"
git status --short
echo
read -rp "Release $TAG? [y/N] " OK
[[ "$OK" =~ ^[Yy]$ ]] || { echo "Cancelled."; git reset -q; exit 1; }

# ---------- commit, push, tag, push the tag ----------
git diff --cached --quiet || git commit -m "Release $TAG"
git push -u origin HEAD
git tag "$TAG"
git push origin "$TAG"

echo
echo "Done. $TAG is pushed and the build has started."
echo "Watch it in the Actions tab of your repo on GitHub."
