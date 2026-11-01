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
  ( cd "$ROOT/rust" && cargo build --release )
  export_platform linux "Linux" game.x86_64
}

build_windows() {
  if [ "$HOST" = "windows" ]; then
    ( cd "$ROOT/rust" && cargo build --release )
  else
    # cross-compile from Linux, then copy the dll to where .gdextension looks
    ( cd "$ROOT/rust" && cargo build --release --target x86_64-pc-windows-gnu )
    mkdir -p "$ROOT/rust/target/release"
    cp "$ROOT/rust/target/x86_64-pc-windows-gnu/release/$CRATE_LIB.dll" \
       "$ROOT/rust/target/release/"
  fi
  export_platform windows "Windows Desktop" game.exe
}
build_android() {
  local platform="android"
  local folder="$GAME_NAME-v$VERSION-$platform"
  local outdir="$ROOT/build/$platform/$folder"

  command -v cargo-ndk >/dev/null || { echo "cargo-ndk is not installed (cargo install cargo-ndk)"; exit 1; }

  local sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
  local jdk="${JAVA_HOME_17_X64:-${JAVA_HOME:-}}"
  [ -n "$sdk" ] || { echo "ANDROID_HOME is not set"; exit 1; }
  [ -n "$jdk" ] || { echo "No JDK found (JAVA_HOME is not set)"; exit 1; }

  for v in GODOT_ANDROID_KEYSTORE_RELEASE_PATH GODOT_ANDROID_KEYSTORE_RELEASE_USER GODOT_ANDROID_KEYSTORE_RELEASE_PASSWORD; do
    [ -n "${!v:-}" ] || { echo "$v is not set"; exit 1; }
  done

  rm -rf "$ROOT/build/$platform"
  mkdir -p "$outdir" "$ROOT/export/$platform"

  # 1. build the Rust library for Android (arm64 only)
  ( cd "$ROOT/rustworkspace" && cargo ndk -t arm64-v8a build --release )

  # 2. on the GitHub computer only: tell Godot where the SDK and JDK are
  #    (never touches your own editor settings)
  if [ "${CI:-}" = "true" ]; then
    local mm
    mm="$("$GODOT" --version | cut -d. -f1,2)"
    mkdir -p "$HOME/.config/godot"
    for f in "editor_settings-4.tres" "editor_settings-$mm.tres"; do
      cat > "$HOME/.config/godot/$f" <<EOF
[gd_resource type="EditorSettings" format=3]

[resource]
export/android/android_sdk_path = "$sdk"
export/android/java_sdk_path = "$jdk"
EOF
    done
  fi

  # 3. import + export the signed release APK
  ( cd "$ROOT/godot" && "$GODOT" --headless --import ) || true
  ( cd "$ROOT/godot" && "$GODOT" --headless --export-release "Android" "$outdir/game.apk" )

  # 4. APK on its own (for itch and phones) + a zip for the GitHub release
  cp "$outdir/game.apk" "$ROOT/export/$platform/$folder.apk"
  rm -f "$ROOT/export/$platform/$folder.zip"
  ( cd "$ROOT/build/$platform" && make_zip "$ROOT/export/$platform/$folder.zip" "$folder" )
  echo "Done: export/$platform/$folder.apk"
}
# build_android() { echo "android: not set up yet, skipping"; }
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
