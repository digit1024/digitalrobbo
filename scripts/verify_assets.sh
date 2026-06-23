#!/usr/bin/env bash
# Verify runtime asset paths referenced by DigitalRobbo exist under assets/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS="$ROOT/assets"
MISSING=0

check() {
    local path="$1"
    if [[ ! -e "$ASSETS/$path" ]]; then
        echo "MISSING: assets/$path"
        MISSING=1
    fi
}

# Sprites (from assets.rs)
for name in screw capsule capsule_ready bomb box push_box key bullet_pickup extra_life \
    question_mark bullet teleport magnet bear_up baar_2Up butterfly bird \
    gun_right gun_down gun_left gun_up tile_ground tile_wall_grey tile_wall_solid \
    tile_wall_a tile_wall_b tile_wall_c dirt door tile_barrier; do
    check "sprites/${name}.png"
done
check "sprites/player/player_spritesheet.png"

# UI
for name in space planet menu_panel facebook replay next replays settingss bmideas; do
    check "ui/${name}.png"
done

# Font
check "fonts/MarkerFelt.ttf"

# Audio manifest entries
MANIFEST="$ASSETS/audio/manifest.ron"
if [[ ! -f "$MANIFEST" ]]; then
    echo "MISSING: assets/audio/manifest.ron"
    MISSING=1
else
    while IFS= read -r path; do
        check "$path"
    done < <(grep -oE '"[^"]+\.(ogg|wav)"' "$MANIFEST" | tr -d '"')
fi

check "camera.ron"

if [[ "$MISSING" -ne 0 ]]; then
    echo "Asset verification failed."
    exit 1
fi
echo "All referenced runtime assets present."
