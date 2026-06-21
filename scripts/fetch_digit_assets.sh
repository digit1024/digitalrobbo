#!/usr/bin/env bash
# Download fonts, audio, and UI sprites from the DigitAdventures digit1024 project.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="https://raw.githubusercontent.com/digit1024/DigitAdventures/master/cocos2d-x-2.2.6/projects/digit1024/Resources"

fetch() {
    local dest="$1"
    local url="$2"
    mkdir -p "$(dirname "$dest")"
    if [[ -f "$dest" ]]; then
        echo "skip $dest"
        return
    fi
    echo "fetch $dest"
    curl -fsSL "$url" -o "$dest"
}

slice_atlas() {
    local atlas="$ROOT/assets/ui/robbo_atlas.png"
    local out_dir="$ROOT/assets/ui"
    if [[ ! -f "$atlas" ]]; then
        return
    fi
    if [[ -f "$out_dir/bmideas.png" ]]; then
        echo "skip atlas slices"
        return
    fi
    if ! command -v convert &>/dev/null; then
        echo "warn: install imagemagick to slice bmideas.png from robbo_atlas.png"
        return
    fi
    echo "slice atlas frames"
    convert "$atlas" -crop 623x207+2+368 +repage "$out_dir/bmideas.png"
}

# ── fonts ─────────────────────────────────────────────────────────────
fetch "$ROOT/assets/fonts/MarkerFelt.ttf" \
    "$BASE/fonts/Marker%20Felt.ttf"

# ── UI ──────────────────────────────────────────────────────────────────
for name in space.png planet.png mute.png unmute.png settings.png ribbon.png giveUp.png settingss.png replays.png; do
    fetch "$ROOT/assets/ui/$name" "$BASE/$name"
done

fetch "$ROOT/assets/ui/robbo_atlas.png" "$BASE/robbo.png"
slice_atlas

# ── SFX (.ogg on Linux, matching original SOUND_EXT) ────────────────────
SFX=(
    321go ammo bomb crack door end ice key lelvelUp life screw shoot szur teleport walk
)
for name in "${SFX[@]}"; do
    fetch "$ROOT/assets/audio/sfx/${name}.ogg" "$BASE/Sounds/${name}.ogg"
done

# ── music (.ogg from music/ subfolder) ──────────────────────────────────
declare -A MUSIC=(
    ["super_friendly.ogg"]="Super%20Friendly.ogg"
    ["chipper_doodle_v2.ogg"]="Chipper%20Doodle%20v2.ogg"
    ["deliberate_thought.ogg"]="Deliberate%20Thought.ogg"
    ["happy_bee.ogg"]="Happy%20Bee.ogg"
    ["move_forward.ogg"]="Move%20Forward.ogg"
    ["pinball_spring_160.ogg"]="Pinball%20Spring%20160.ogg"
    ["new_friendly.ogg"]="New%20Friendly.ogg"
    ["show_your_moves.ogg"]="Show%20Your%20Moves.ogg"
    ["mellowtron.ogg"]="Mellowtron.ogg"
    ["kick_shock.ogg"]="Kick%20Shock.ogg"
)
for local_name in "${!MUSIC[@]}"; do
    remote="${MUSIC[$local_name]}"
    fetch "$ROOT/assets/audio/music/$local_name" "$BASE/Sounds/music/$remote"
done

echo "Done. Assets under $ROOT/assets/"
