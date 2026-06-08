#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

CONFIG="${1:-release}"
APP="build/PrivateWhisper.app"

echo "Building ($CONFIG)..."
swift build -c "$CONFIG"
BIN="$(swift build -c "$CONFIG" --show-bin-path)"

echo "Assembling ${APP}..."
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp Resources/Info.plist "$APP/Contents/Info.plist"
cp "$BIN/PrivateWhisperApp" "$APP/Contents/MacOS/PrivateWhisperApp"

# Скопировать ресурсные бандлы SPM (в т.ч. Metal-шейдеры whisper.cpp), если есть.
for b in "$BIN"/*.bundle; do
  [ -e "$b" ] && cp -R "$b" "$APP/Contents/Resources/"
done

# Подпись: предпочитаем стабильную идентичность (Apple Development / Developer ID),
# иначе ad-hoc. Стабильная подпись сохраняет TCC-разрешения между пересборками.
SIGN_ID="${CODESIGN_IDENTITY:-}"
if [ -z "$SIGN_ID" ]; then
  SIGN_ID="$(security find-identity -v -p codesigning 2>/dev/null \
    | awk -F'"' '/Apple Development|Developer ID Application/ {print $2; exit}')"
fi
if [ -z "$SIGN_ID" ]; then SIGN_ID="-"; fi

echo "Signing as: $SIGN_ID"
codesign --force --deep --sign "$SIGN_ID" "$APP"

echo "Done: $APP"
