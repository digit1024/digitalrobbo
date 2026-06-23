#!/usr/bin/env bash
# Regenerate launcher mipmaps from assets/icon.png (1024×1024 source).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/assets/icon.png"
RES="$ROOT/mobile/android/app/src/main/res"

if [[ ! -f "$SRC" ]]; then
  echo "Missing source icon: $SRC" >&2
  exit 1
fi

mkdir -p \
  "$RES/mipmap-mdpi" "$RES/mipmap-hdpi" "$RES/mipmap-xhdpi" \
  "$RES/mipmap-xxhdpi" "$RES/mipmap-xxxhdpi" "$RES/mipmap-anydpi-v26" "$RES/values"

for spec in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
  density="${spec%%:*}"
  size="${spec##*:}"
  convert "$SRC" -resize "${size}x${size}" "$RES/mipmap-$density/ic_launcher.png"
  cp "$RES/mipmap-$density/ic_launcher.png" "$RES/mipmap-$density/ic_launcher_round.png"
done

for spec in mdpi:108 hdpi:162 xhdpi:216 xxhdpi:324 xxxhdpi:432; do
  density="${spec%%:*}"
  size="${spec##*:}"
  convert "$SRC" -resize "${size}x${size}" "$RES/mipmap-$density/ic_launcher_foreground.png"
done

echo "Android launcher icons updated from $SRC"
