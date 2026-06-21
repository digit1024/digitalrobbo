# 06 — Level Format

> **Status:** Draft  
> **Last updated:** M-DOC

## gnurobbo `.dat` pack structure

INI-like sections. Example from `robbo01.dat`:

```ini
[name]
RobboI

[last_level]
56

[default_level_colour]
608050

[offset]
3897

[level]
1

[colour]
555555

[size]
16.31

[author]
Janusz Pelc

[level_notes]

[data]
ssssssssssssssss
s...s...sT.s...s
...
```

### Pack-level keys

- `[name]` — pack display name
- `[last_level]` — level count
- `[default_level_colour]` — hex RGB default
- `[offset]` — optional metadata offset

### Per-level keys

- `[level]` — 1-based index
- `[colour]` — hex RGB background tint
- `[size]` — `WxH` e.g. `16.31`, `32.31`
- `[author]`, `[level_notes]`
- `[data]` — ASCII grid, one row per line

## Tile legend (GNU Robbo)

| Char | Element |
|------|---------|
| `.` | Empty |
| `R` | Robbo spawn |
| `O` / `o` | Grey / green wall |
| `-` | Black wall |
| `Q` | Red wall |
| `T` | Screw |
| `'` | Bullet / ammo |
| `#` | Box |
| `%` | Key |
| `b` | Bomb |
| `D` | Door |
| `?` | Questionmark |
| `@` | Bear |
| `^` | Bird |
| `!` | Capsule |
| `+` | Extra life |
| `H` | Ground |
| `*` | Black bear |
| `V` | Butterfly |
| `&` | Teleport |
| `}` | Gun |
| `M` | Magnet |
| `~` | Push box |
| `=` | Energetic barrier |

Directional variants use extended chars — see gnurobbo `screen.c` and `tools/LevelFormats.txt`.

## Caveat: real files vs documentation

Pack files may use **variant characters** (e.g. `s` for wall in some packs). Parser MUST mirror gnurobbo's loader, not only `LevelFormats.txt`. Validation: round-trip all shipped packs; compare entity counts with reference loader.

## Parser output model

```rust
pub struct LevelPack {
    pub name: String,
    pub levels: Vec<Level>,
}

pub struct Level {
    pub index: u32,
    pub width: u16,
    pub height: u16,
    pub colour: u32,
    pub author: String,
    pub notes: String,
    pub cells: Vec<Vec<TileKind>>,
    pub required_screws: u32,
}
```

## Validation strategy

1. Unit tests per char mapping
2. Round-trip: parse → serialize → parse equality
3. Integration: load `original.dat`, assert 56 levels, dimensions, screw counts
4. Solvability harness (later): no soft-locks on original set
