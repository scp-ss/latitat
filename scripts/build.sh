#!/usr/bin/env bash
set -euo pipefail

# ====== EDIT THESE ======
GAME_NAME="Latiat"
CRATE_LIB="godot-bridge"          # the godot-rust crate's library name (not the data crate)
GODOT="${GODOT:-godot}"     # change to godot4 if that is your command


ROOT="$(git rev-parse --show-toplevel)"

case "$(uname -s)" in
  Linux*) HOST=linux ;;
  MINGW*|MSYS*|CYGWIN*) HOST=windows ;;
  *) echo "Unsupported host: $(uname -s)"; exit 1 ;;
esac

# ---------- options (used by GitHub Actions, skips the questions) ----------
VERSION=""
PLATFORMS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --version)  VERSION="$2"; shift 2 ;;
    --platform) PLATFORMS+=("$2"); shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

ask_number() {
  local v
  while true; do
    read -rp "$1: " v
    if [[ "$v" =~ ^[0-9]+$ ]]; then echo "$v"; return; fi
    echo "  Please type a whole number." >&2
  done
}

# ---------- version questions ----------
if [ -z "$VERSION" ]; then
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
    LEVELNUM=$(ask_number "Level patch (1 means $LEVEL.1)")
    VERSION="$VERSION-$LEVEL.$LEVELNUM"
  fi
  read -rp "Build version $VERSION? [y/N] " OK
  [[ "$OK" =~ ^[Yy]$ ]] || { echo "Cancelled."; exit 1; }
fi

# ---------- platform question ----------
if [ ${#PLATFORMS[@]} -eq 0 ]; then
  read -rp "Platforms (linux windows android web, or 'all') [linux windows]: " P
  P="${P:-linux windows}"
  [ "$P" = "all" ] && P="linux windows android web"
  read -ra PLATFORMS <<< "$P"
fi

# ---------- helpers ----------
make_zip() {   # make_zip <zip path> <folder name>
  if command -v zip >/dev/null 2>&1; then
    zip -r "$1" "$2"
  elif command -v 7z >/dev/null 2>&1; then
    7z a -tzip "$1" "$2"
  else
    echo "Need 'zip' or '7z' installed."; exit 1
  fi
}

export_platform() {   # export_platform <platform> <godot preset> <output file name>
  local platform="$1" preset="$2" outfile="$3"
  local folder="$GAME_NAME-v$VERSION-$platform"
  local outdir="$ROOT/build/$platform/$folder"

  rm -rf "$ROOT/build/$platform"
  mkdir -p "$outdir" "$ROOT/export/$platform"

  ( cd "$ROOT/godot" && "$GODOT" --headless --import ) || true
  ( cd "$ROOT/godot" && "$GODOT" --headless --export-release "$preset" "$outdir/$outfile" )

  if ! ls "$outdir" | grep -Eq '\.(so|dll)$'; then
    echo "WARNING: no .so/.dll in $outdir. The game will crash. Check your .gdextension paths." >&2
  fi
  if [ "$platform" = "linux" ]; then
    chmod +x "$outdir/$outfile"
  fi

  rm -f "$ROOT/export/$platform/$folder.zip"
  ( cd "$ROOT/build/$platform" && make_zip "$ROOT/export/$platform/$folder.zip" "$folder" )
  echo "Done: export/$platform/$folder.zip"
}

# ---------- one function per platform ----------
build_linux() {
  [ "$HOST" = "linux" ] || { echo "Linux builds need a Linux machine."; exit 1; }
  ( cd "$ROOT/rustworkspace" && cargo build --release )
  export_platform linux "Linux" game.x86_64
}

build_windows() {
  if [ "$HOST" = "windows" ]; then
    ( cd "$ROOT/rustworkspace" && cargo build --release )
  else
    # cross-compile from Linux, then copy the dll to where .gdextension looks
    ( cd "$ROOT/rustworkspace" && cargo build --release --target x86_64-pc-windows-gnu )
    mkdir -p "$ROOT/rustworkspace/target/release"
    cp "$ROOT/rustworkspace/target/x86_64-pc-windows-gnu/release/$CRATE_LIB.dll" \
       "$ROOT/rustworkspace/target/release/"
  fi
  export_platform windows "Windows Desktop" game.exe
}

build_android() { echo "android: not set up yet, skipping"; }
build_web()     { echo "web: not set up yet, skipping"; }

# ---------- run ----------
echo "Version: $VERSION"
for p in "${PLATFORMS[@]}"; do
  echo "==> $p"
  case "$p" in
    linux)   build_linux ;;
    windows) build_windows ;;
    android) build_android ;;
    web)     build_web ;;
    *) echo "Unknown platform: $p"; exit 1 ;;
  esac
done
echo "All done. Zips are in export/<platform>/"
