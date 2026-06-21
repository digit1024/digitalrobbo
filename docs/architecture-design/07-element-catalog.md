# 07 — Element Catalog

> **Status:** Draft  
> **Last updated:** M-DOC

Parity reference for GNU Robbo / Atari Robbo. Update **Implemented** column each milestone.

## Player

| Element | Behaviour | Tick rules | Implemented |
|---------|-----------|------------|-------------|
| Robbo | Move 4-dir, push boxes, shoot | One cell per input step | M1 |
| Robbo death | Touch hazard/enemy/projectile | Immediate fail or respawn | M4 |

## Terrain

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Empty `.` | Walkable | M1 |
| Walls `O/o/-Q/s` | Block movement and shots | M1 |
| Ground `H` | Walkable; may affect visuals | M4 |
| Door `D` | Blocks until key collected | M4 |
| Barrier `=` | Energetic barrier entry/exit | M4 |

## Collectibles

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Screw `T` | Required count to open capsule | M1 |
| Key `%` | Opens doors | M4 |
| Bullet `'` | Ammo pickup | M1 |
| Life `+` | Extra life (if lives enabled) | M4 |
| Capsule `!` | Exit when screws met; Robbo enters to win | M1 |

## Pushables / hazards

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Box `#` | Pushable; blocks | M1 |
| Push box `~` | Pushable variant | M4 |
| Bomb `b` | Explodes; chain reaction | M4 |
| Questionmark `?` | Reveals hidden item | M4 |

## Enemies

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Bear `@` | Wall-following CW/CCW | M4 |
| Black bear `*` | Faster wall-follower | M4 |
| Bird `^` | Patrol / vertical / firing variants | M4 |
| Butterfly `V` | Random movement pattern | M4 |

## Mechanics

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Gun `}` | Laser/blaster/regular; fixed/rotating/moving | M4 |
| Magnet `M` | Pulls Robbo directionally | M4 |
| Teleport `&` | Paired numbered portals | M4 |
| Projectile | From Robbo or guns; collision rules | M1/M4 |

## Win / lose

| Condition | Rule | Implemented |
|-----------|------|-------------|
| Win | Required screws collected + Robbo on capsule | M1 |
| Lose | Robbo dies (hazard/enemy/explosion) | M4 |
