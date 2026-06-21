# Glossary

> **Status:** Draft  
> **Last updated:** M-DOC

| Term | Definition |
|------|------------|
| **Cell** | One grid square `(col, row)` |
| **Tick** | One logical simulation step in `robbo-core` |
| **Snapshot** | Full clone of `World` for undo |
| **Projection** | `GridProjection` — maps cells to world/pixel coords |
| **Skin** | Asset pack: atlases + `skin.ron` manifest |
| **Pack** | Collection of levels in one `.dat` file |
| **Screw** | Collectible bolt; required count opens capsule |
| **Capsule** | Exit tile; win when Robbo enters after screws met |
| **Command** | One player action applied via `World::step` |
| **Bridge** | Bevy layer syncing ECS visuals to core state |
| **Interpolation** | Visual tween between `from_cell` and `to_cell` |
| **ADR** | Architecture Decision Record |
| **GNU Robbo** | Open-source Robbo clone; reference for logic and levels |
| **RobboVI** | Atari game GNU Robbo is based on; 56 original levels |
