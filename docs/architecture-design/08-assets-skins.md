# 08 — Assets and Skins

> **Status:** Implemented  
> **Last updated:** 2026-06-21

## Goals

- Consistent aesthetics across all tiles and entities
- Easy to swap atlases and add animations without code changes
- Forward-compatible with isometric 2.5D art (separate skin pack)

## Directory layout

```
assets/
├── audio/
│   ├── manifest.ron
│   ├── music/          # BGM pool (Kevin MacLeod, CC BY 3.0)
│   └── sfx/            # one-shot effects
├── fonts/
│   └── MarkerFelt.ttf
├── ui/                 # menu backgrounds and icons
├── sprites/            # gameplay sprites
├── levels/             # .dat packs
└── skins/
    └── default/
        ├── skin.ron
        ├── tiles.png
        ├── entities.png
        └── ui.png
```

## Audio manifest (`assets/audio/manifest.ron`)

```ron
(
    menu_music: "audio/music/super_friendly.ogg",
    level_music: [
        "audio/music/chipper_doodle_v2.ogg",
        // ... pool; track picked by level_content_seed(level)
    ],
    sfx: {
        "walk": "audio/sfx/walk.ogg",
        "collected_screw": "audio/sfx/screw.ogg",
        // ...
    },
)
```

Level BGM index = `level_content_seed(level) % level_music.len()` (`robbo-formats/level_hash.rs`).

Fetch script: `scripts/fetch_digit_assets.sh` (DigitAdventures source). Attribution: `assets/AUDIO_ATTRIBUTION.md`.

## Skin manifest schema (RON)

```ron
(
    name: "default",
    tile_size: 32,
    palette: (
        primary: "#4A6B8A",
        secondary: "#7D5C34",
        background: "#1A1A2E",
        accent: "#E67E22",
    ),
    tiles: {
        "wall_grey": ( atlas: "tiles.png", rect: (x:0, y:0, w:32, h:32) ),
        "empty": ( atlas: "tiles.png", rect: (x:32, y:0, w:32, h:32) ),
    },
    entities: {
        "robbo": (
            atlas: "entities.png",
            animations: {
                "walk_down": ( frames: [(0,0),(1,0),(2,0),(3,0)], fps: 8, loop: true ),
                "idle_down": ( frames: [(0,0)], fps: 1, loop: true ),
            },
        ),
    },
    sounds: {
        "walk": "sfx/step.ogg",
        "collect_screw": "sfx/screw.ogg",
    },
)
```

Logical names (`robbo`, `wall_grey`) map to atlas regions and animation definitions. Rendering code only uses logical names.

## v1 asset sources

1. **DigitAdventures** (Cocos2d-x remake) — reuse where license/quality permits
2. **Curated free assets** — fill gaps with consistent pixel scale
3. **gnurobbo GPL skins** — reference/fallback for parity checking

## Swapping skins

1. Add folder under `assets/skins/<name>/`
2. Provide `skin.ron` + atlases
3. Select via settings or `--skin` CLI flag

## Isometric forward plan (M7)

- New skin pack with pre-rendered or hand-drawn iso sprites
- Same logical names in manifest; different rects/atlases
- `IsometricProjection` + iso `sort_key` for depth

## Animation principles

- 60 FPS render; sprite anim ≥ 8 FPS for walk cycles
- Ease-in-out on movement tweens (see [04-view-decoupling.md](04-view-decoupling.md))
- Push: synchronized Robbo + box
- Teleport: alpha fade 0.2s + 0.2s
