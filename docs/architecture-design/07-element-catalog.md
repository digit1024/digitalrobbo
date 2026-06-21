# 07 — Element Catalog

> **Status:** Draft  
> **Last updated:** 2025-06

Parity reference for GNU Robbo 0.65.6 / Atari Robbo. **Implemented** reflects `robbo-core` behaviour (Robbo move/shoot timing uses our instant-per-step model, not gnurobbo `DELAY_*` cycles).

## Player

| Element | Behaviour | Tick rules | Implemented |
|---------|-----------|------------|-------------|
| Robbo | Move 4-dir, push boxes, shoot | One cell per input step (no gnurobbo cooldown) | Yes |
| Robbo death | Touch hazard/enemy/projectile/laser | Immediate fail | Yes |

## Terrain

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Empty `.` | Walkable | Yes |
| Walls `O/o/-Q/s` | Block movement and shots | Yes |
| Ground `H` | Blocks movement; shootable/destroyable | Yes |
| Door `D` | Blocks until key; opens per-door on step (key consumed, Robbo stays) | Yes |
| STOP `X` | Walk clears tile to empty | Yes |
| Barrier `=` | Conveyor row shift every 4 ticks | Yes |

## Collectibles

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Screw `T` | Required count to open capsule | Yes |
| Key `%` | Collected only; used when stepping on doors | Yes |
| Bullet `'` | +9 ammo per pickup | Yes |
| Life `+` | Stripped to empty at load (gnurobbo parity) | Yes |
| Capsule `!` | Exit when screws met; Robbo enters to win | Yes |

## Pushables / hazards

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Box `#` | Pushable; blocks | Yes |
| Push box `~` | Push then slides every 4 ticks; shoots when blocked | Yes |
| Bomb `b` | 3×3 explosion; chain reaction | Yes |
| Questionmark `?` | Pushable; shot → BigBoom → reveal; bomb destroys without reveal | Yes |
| Barbed wire `k` | Walk clears + kills Robbo | Yes |

## Enemies

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Bear `@` | Wall-following CW/CCW; GNU `DELAY_BEAR=4` → every 2 sim ticks (10 Hz) | Yes |
| Black bear `*` | Right-hand wall-follow (same speed as bear) | Yes |
| Bird `^` | Patrol along `direction`; GNU `DELAY_BIRD=4` → every 2 sim ticks; optional 1/8 shoot | Yes |
| Butterfly `V` | Homing toward Robbo; GNU `DELAY_BUTTERFLY=4` → every 2 sim ticks | Yes |

## Mechanics

| Element | Behaviour | Implemented |
|---------|-----------|-------------|
| Gun `}` | Regular/laser/blaster; movable/rotating; 1/8 fire on gun ticks | Yes |
| Laser `L`/`l` | Beam segments; gun-linked orphan cleanup | Yes |
| Blaster | Spread cells; destroys most objects | Yes |
| Magnet `M` | Locks Robbo; pulls 1 cell/tick toward magnet | Yes |
| Teleport `&` | Ring lookup by group+index; exit-direction search | Yes |
| Projectile | From Robbo or guns; full collision set | Yes |

## Win / lose

| Condition | Rule | Implemented |
|-----------|------|-------------|
| Win | Required screws collected + Robbo on capsule | Yes |
| Lose | Robbo dies (hazard/enemy/explosion/laser) | Yes |
