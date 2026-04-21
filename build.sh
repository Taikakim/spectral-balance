#!/usr/bin/env bash
# Build, sign, notarize, and optionally package spectral_forge (.clap + .vst3) for macOS.
#
# Certificates needed (both in login keychain — team: RentalPoint Software Inc):
#   - Developer ID Application: for signing the .clap and .vst3 bundles
#   - Developer ID Installer:   for signing the .pkg installer
#
# For notarization, store credentials once:
#   xcrun notarytool store-credentials "spectral-forge-notary" \
#       --apple-id warren@rentp.com \
#       --team-id XXXXXX \
#       --password <app-specific-password-from-account.apple.com>
# Then set: export NOTARY_PROFILE=spectral-forge-notary
#
# Or set env vars instead:
#   APPLE_ID        warren@rentp.com
#   APPLE_APP_PASS  app-specific password
#   APPLE_TEAM_ID   XXXXX
#
# Usage:
#   ./build.sh                    # build + sign + notarize .clap and .vst3
#   ./build.sh --no-notarize      # build + sign only
#   ./build.sh --pkg              # also build a signed+notarized .pkg installer
#   ./build.sh --install          # also install to ~/.clap/ and ~/Library/Audio/Plug-Ins/VST3/
#   ./build.sh --pkg --install    # all of the above

set -euo pipefail

PLUGIN_NAME="spectral_forge"
BUNDLE_ID="com.nih-plug.spectral_forge"
CLAP_PATH="target/bundled/${PLUGIN_NAME}.clap"
VST3_PATH="target/bundled/${PLUGIN_NAME}.vst3"
PKG_PATH="target/bundled/${PLUGIN_NAME}.pkg"
CLAP_INSTALL_DIR="$HOME/.clap"
VST3_INSTALL_DIR="$HOME/Library/Audio/Plug-Ins/VST3"
NOTARIZE=true
BUILD_PKG=false
INSTALL=false

for arg in "$@"; do
  case "$arg" in
    --no-notarize) NOTARIZE=false ;;
    --pkg)         BUILD_PKG=true ;;
    --install)     INSTALL=true ;;
    --help|-h)
      sed -n '2,20p' "$0" | sed 's/^# //'
      exit 0 ;;
  esac
done

# ── Find cargo ───────────────────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  CARGO=$(command -v cargo)
elif [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  CARGO="$HOME/.cargo/bin/cargo"
else
  echo "ERROR: cargo not found. Install Rust from https://rustup.rs" >&2
  exit 1
fi
echo "Using cargo: $CARGO"

# ── Build ────────────────────────────────────────────────────────────────────
echo ""
echo "==> Building release..."
"$CARGO" build --release

echo ""
echo "==> Bundling CLAP + VST3..."
"$CARGO" run --package xtask -- bundle "$PLUGIN_NAME" --release

# ── Pick signing identity ─────────────────────────────────────────────────────
APP_SIGN_ID=$(security find-identity -v -p codesigning \
  | grep "Developer ID Application" \
  | head -1 \
  | sed 's/.*"\(.*\)".*/\1/' || true)

if [[ -z "$APP_SIGN_ID" ]]; then
  APP_SIGN_ID=$(security find-identity -v -p codesigning \
    | grep "Apple Development" \
    | head -1 \
    | sed 's/.*"\(.*\)".*/\1/' || true)
  if [[ -n "$APP_SIGN_ID" ]]; then
    echo ""
    echo "WARNING: No 'Developer ID Application' cert found — falling back to: $APP_SIGN_ID"
    echo "  This build CANNOT be notarized."
    NOTARIZE=false
    BUILD_PKG=false
  else
    echo "WARNING: No code-signing identity found — skipping signing and notarization."
    APP_SIGN_ID=""
    NOTARIZE=false
    BUILD_PKG=false
  fi
else
  echo ""
  echo "==> App signing identity:       $APP_SIGN_ID"
fi

INSTALLER_SIGN_ID=$(security find-identity -v \
  | grep "Developer ID Installer" \
  | head -1 \
  | sed 's/.*"\(.*\)".*/\1/' || true)

if [[ -n "$INSTALLER_SIGN_ID" ]]; then
  echo "==> Installer signing identity: $INSTALLER_SIGN_ID"
fi

# ── Sign a bundle (inner binary first, then the bundle) ───────────────────────
sign_bundle() {
  local bundle="$1"
  local binary_name="$2"
  local identifier="$3"
  local binary

  # VST3 nests the binary one level deeper: Contents/MacOS/<name>
  # Both .clap and .vst3 use the same layout on macOS.
  binary="$bundle/Contents/MacOS/$binary_name"

  codesign --force --sign "$APP_SIGN_ID" \
    --options runtime --timestamp --strict \
    "$binary"

  codesign --force --sign "$APP_SIGN_ID" \
    --options runtime --timestamp --strict \
    --identifier "$identifier" \
    "$bundle"

  codesign --verify --deep --strict --verbose=2 "$bundle"
}

if [[ -n "$APP_SIGN_ID" ]]; then
  echo ""
  echo "==> Signing .clap bundle..."
  sign_bundle "$CLAP_PATH" "$PLUGIN_NAME" "$BUNDLE_ID"

  echo ""
  echo "==> Signing .vst3 bundle..."
  sign_bundle "$VST3_PATH" "$PLUGIN_NAME" "${BUNDLE_ID}.vst3"
fi

# ── Notarize helper ───────────────────────────────────────────────────────────
notarize_file() {
  local file="$1"
  local ext="${file##*.}"
  local zip="${file%.*}_${ext}.zip"

  rm -f "$zip"
  ditto -c -k --keepParent "$file" "$zip"

  if [[ -n "${NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$zip" --keychain-profile "$NOTARY_PROFILE" --wait
  elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_PASS:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    xcrun notarytool submit "$zip" \
      --apple-id "$APPLE_ID" \
      --password "$APPLE_APP_PASS" \
      --team-id "$APPLE_TEAM_ID" \
      --wait
  else
    echo ""
    echo "ERROR: Notarization credentials not set."
    echo "  Set NOTARY_PROFILE, or APPLE_ID + APPLE_APP_PASS + APPLE_TEAM_ID."
    echo ""
    echo "  Store credentials once with:"
    echo "    xcrun notarytool store-credentials \"spectral-forge-notary\" \\"
    echo "      --apple-id <your-apple-id> --team-id <your-team-id> \\"
    echo "      --password <app-specific-password>"
    echo "  Then: export NOTARY_PROFILE=spectral-forge-notary"
    exit 1
  fi

  xcrun stapler staple "$file"
  rm -f "$zip"
}

# ── Notarize .clap and .vst3 ─────────────────────────────────────────────────
if [[ "$NOTARIZE" == "true" ]]; then
  echo ""
  echo "==> Notarizing .clap bundle..."
  notarize_file "$CLAP_PATH"
  echo "==> .clap notarization complete."

  echo ""
  echo "==> Notarizing .vst3 bundle..."
  notarize_file "$VST3_PATH"
  echo "==> .vst3 notarization complete."
fi

# ── Build .pkg installer ──────────────────────────────────────────────────────
if [[ "$BUILD_PKG" == "true" ]]; then
  echo ""
  echo "==> Building .pkg installer (CLAP + VST3)..."

  if [[ -z "$INSTALLER_SIGN_ID" ]]; then
    echo "ERROR: No 'Developer ID Installer' cert found — cannot build signed .pkg." >&2
    exit 1
  fi

  STAGE_DIR="target/bundled/pkg_stage"
  rm -rf "$STAGE_DIR"
  mkdir -p "$STAGE_DIR/Library/Audio/Plug-Ins/CLAP"
  mkdir -p "$STAGE_DIR/Library/Audio/Plug-Ins/VST3"
  cp -R "$CLAP_PATH" "$STAGE_DIR/Library/Audio/Plug-Ins/CLAP/"
  cp -R "$VST3_PATH" "$STAGE_DIR/Library/Audio/Plug-Ins/VST3/"

  pkgbuild \
    --root "$STAGE_DIR" \
    --identifier "$BUNDLE_ID.pkg" \
    --version "0.1.0" \
    --install-location "/" \
    --sign "$INSTALLER_SIGN_ID" \
    --timestamp \
    "$PKG_PATH"

  echo "==> Verifying .pkg signature..."
  pkgutil --check-signature "$PKG_PATH"

  if [[ "$NOTARIZE" == "true" ]]; then
    echo ""
    echo "==> Notarizing .pkg..."
    notarize_file "$PKG_PATH"
    echo "==> .pkg notarization complete."
  fi

  echo "==> Package: $PKG_PATH"
fi

# ── Install ───────────────────────────────────────────────────────────────────
if [[ "$INSTALL" == "true" ]]; then
  echo ""
  echo "==> Installing .clap to $CLAP_INSTALL_DIR/..."
  mkdir -p "$CLAP_INSTALL_DIR"
  rm -rf "$CLAP_INSTALL_DIR/${PLUGIN_NAME}.clap"
  cp -R "$CLAP_PATH" "$CLAP_INSTALL_DIR/"
  echo "    Installed: $CLAP_INSTALL_DIR/${PLUGIN_NAME}.clap"

  echo "==> Installing .vst3 to $VST3_INSTALL_DIR/..."
  mkdir -p "$VST3_INSTALL_DIR"
  rm -rf "$VST3_INSTALL_DIR/${PLUGIN_NAME}.vst3"
  cp -R "$VST3_PATH" "$VST3_INSTALL_DIR/"
  echo "    Installed: $VST3_INSTALL_DIR/${PLUGIN_NAME}.vst3"
fi

# ── Done ─────────────────────────────────────────────────────────────────────
echo ""
echo "==> Build complete."
for bundle in "$CLAP_PATH" "$VST3_PATH"; do
  echo ""
  codesign -dv --verbose=4 "$bundle" 2>&1 | grep -E "Authority|TeamIdentifier|Identifier" || true
done
