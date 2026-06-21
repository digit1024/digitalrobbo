# 10 — Persistence

> **Status:** Draft  
> **Last updated:** M-DOC

## Save model

```ron
(
    version: 1,
    profile: (
        last_pack: "original",
        last_level: 12,
    ),
    packs: {
        "original": (
            levels: {
                "1": ( completed: true, best_time_ms: 45230, stars: 1 ),
                "2": ( completed: true, best_time_ms: 38100, stars: 1 ),
            },
        ),
    },
    settings: (
        master_volume: 0.8,
        sfx_volume: 1.0,
        colourblind_mode: false,
        show_grid: false,
        skin: "default",
        keymap: { /* ... */ },
    ),
)
```

## Storage backends

| Platform | Backend | Path / API |
|----------|---------|------------|
| Desktop | RON file | `~/.config/digitalrobbo/save.ron` |
| Web | localStorage | key `digitalrobbo_save` |
| Mobile | platform app storage | TBD per Bevy mobile template |

Abstract via `SaveStorage` trait in `robbo-app`; inject platform implementation at startup.

## What is persisted

- Per-level completion and best time
- Last played pack/level
- Settings (audio, keys, accessibility, skin)
- Optional: undo disabled preference

## What is NOT persisted

- In-level undo stack (session only)
- Mid-level state (puzzle restarts on exit unless added later)

## Migration

- `version` field in save blob
- Upgrade fn per version bump
