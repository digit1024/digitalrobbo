#!/usr/bin/env python3
"""
Extract individual sprites from TileGameResources sprite sheets.
Outputs to assets/sprites/ ready for Bevy loading.

Sheets used:
  TiledObjects.png  - 800x800, 10x10 grid of 80x80 tiles  (objects/entities)
  wall.png          - 128x128, 2x2 grid of 64x64 tiles     (wall variants)
"""
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).parent.parent
SRC  = ROOT / "assets" / "sprites" / "raw"
OUT  = ROOT / "assets" / "sprites"
OUT.mkdir(parents=True, exist_ok=True)

TILE = 80  # common output tile size

def crop(img: Image.Image, col: int, row: int, src_tile: int = 80) -> Image.Image:
    x = col * src_tile
    y = row * src_tile
    tile = img.crop((x, y, x + src_tile, y + src_tile))
    if src_tile != TILE:
        tile = tile.resize((TILE, TILE), Image.LANCZOS)
    return tile

# ──────────────────────────────────────────────────────────────
# TiledObjects.png  (80x80 tiles)
# ──────────────────────────────────────────────────────────────
obj = Image.open(SRC / "TiledObjects.png").convert("RGBA")

# (col, row): output_name
OBJ_MAP = {
    # ── tiles / floor ──
    (0, 0): "tile_ground",        # brown dirt floor
    (1, 0): "tile_wall_grey",     # grey cobblestone wall
    (3, 1): "tile_wall_solid",    # ice crystal wall
    (4, 1): "tile_barrier",       # lava / force field
    (0, 1): "door",               # brown door
    # ── collectibles ──
    (1, 2): "screw",              # red star bolt  ← corrected from (0,2)
    (4, 0): "capsule",            # bright yellow exit capsule
    (0, 3): "extra_life",         # gold star
    (1, 3): "key",                # golden key
    (2, 3): "bullet_pickup",      # horizontal bullet clip (also used for projectile)
    (3, 3): "question_mark",      # ? mark
    (3, 0): "bomb",               # black sphere bomb
    # ── pushables ──
    (2, 0): "box",                # wooden crate
    (1, 1): "push_box",           # grey boulder push-box
    # ── robbo ──
    (0, 4): "robbo",
    # ── teleport ──
    (2, 1): "teleport",           # UFO saucer
    (9, 3): "teleport2",          # ice crystal portal (alternate)
    # ── magnet ──
    (9, 2): "magnet",             # silver metallic ball
    # ── bears: 4 directional frames (right/front/back/up) ──
    (5, 0): "bear_right",
    (5, 1): "bear_front",
    (5, 2): "bear_back",
    (5, 3): "bear_up",
    # ── black bear ──
    (8, 0): "blackbear_right",
    (8, 1): "blackbear_front",
    (8, 2): "blackbear_back",
    (8, 3): "blackbear_up",
    # ── butterfly (fire creature): 4 frames ──
    (7, 0): "butterfly_right",
    (7, 1): "butterfly_front",
    (7, 2): "butterfly_back",
    (7, 3): "butterfly_up",
    # ── bird ──
    (9, 1): "bird",
    (9, 9): "bird2",
    # ── projectile (copy of bullet_pickup, written last) ──
    (2, 3): "projectile",
    # ── gun: 4 directions ──
    (7, 4): "gun_right",
    (7, 5): "gun_down",
    (7, 6): "gun_left",
    (7, 7): "gun_up",
}

for (col, row), name in OBJ_MAP.items():
    tile = crop(obj, col, row)
    out_path = OUT / f"{name}.png"
    tile.save(out_path)
    print(f"  {name}.png  ({col},{row})")

# ──────────────────────────────────────────────────────────────
# wall.png  (128x128, treat as 2x2 grid of 64x64)
# ──────────────────────────────────────────────────────────────
wall = Image.open(SRC / "wall.png").convert("RGBA")
WALL_MAP = {
    (0, 0): "tile_wall_a",
    (1, 0): "tile_wall_b",
    (0, 1): "tile_wall_c",
    (1, 1): "tile_wall_d",
}
for (col, row), name in WALL_MAP.items():
    tile = crop(wall, col, row, src_tile=64)
    out_path = OUT / f"{name}.png"
    tile.save(out_path)
    print(f"  {name}.png  wall ({col},{row})")

print(f"\nDone — {len(OBJ_MAP) + len(WALL_MAP)} sprites → {OUT}")
