use std::collections::HashMap;

use robbo_core::{Cell, Direction, ElementState, TileKind, World};

use crate::char_map::{apply_additional_line, cell_from_grid, tile_or_element};
use crate::error::{FormatError, FormatResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelPack {
    pub name: String,
    pub default_colour: u32,
    pub levels: Vec<Level>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Level {
    pub index: u32,
    pub width: u16,
    pub height: u16,
    pub colour: u32,
    pub author: String,
    pub notes: String,
    pub tiles: Vec<TileKind>,
    pub elements: Vec<(Cell, ElementState)>,
    pub required_screws: u32,
    pub barrier_directions: HashMap<Cell, Direction>,
}

impl Level {
    pub fn to_world(&self) -> World {
        let mut world = World::from_level(
            self.width,
            self.height,
            self.tiles.clone(),
            self.elements.clone(),
            self.required_screws,
        );
        world.barrier_directions = self.barrier_directions.clone();
        world.init_after_load();
        world
    }
}

pub fn parse_pack_str(input: &str) -> FormatResult<LevelPack> {
    parse_pack_sections(input)
}

pub fn parse_pack(input: &[u8]) -> FormatResult<LevelPack> {
    let s = std::str::from_utf8(input).map_err(|e| FormatError::Parse(e.to_string()))?;
    parse_pack_str(s)
}

fn parse_pack_sections(input: &str) -> FormatResult<LevelPack> {
    let mut name = String::new();
    let mut default_colour = 608050u32;
    let mut levels = Vec::new();
    let mut section = String::new();
    let mut builder: Option<LevelBuilder> = None;

    let mut flush_level = |levels: &mut Vec<Level>, builder: &mut Option<LevelBuilder>, default_colour: u32| -> FormatResult<()> {
        if let Some(b) = builder.take() {
            if !b.data_rows.is_empty() {
                levels.push(b.finish_grid(default_colour)?);
            }
        }
        Ok(())
    };

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let new_section = trimmed[1..trimmed.len() - 1].to_string();

            if new_section == "level" {
                flush_level(&mut levels, &mut builder, default_colour)?;
                builder = Some(LevelBuilder::default());
            }

            section = new_section;
            continue;
        }

        match section.as_str() {
            "name" if name.is_empty() => name = trimmed.to_string(),
            "default_level_colour" if !trimmed.is_empty() => {
                default_colour = trimmed.parse().unwrap_or(default_colour);
            }
            "data" => {
                if let Some(b) = builder.as_mut() {
                    b.data_rows.push(trimmed.to_string());
                }
            }
            "additional" => {
                if let Some(b) = builder.as_mut() {
                    if !trimmed.is_empty() {
                        b.additional_lines.push(trimmed.to_string());
                    }
                }
            }
            "screws" => {
                if let Some(b) = builder.as_mut() {
                    b.screws_override = trimmed.parse().ok();
                }
            }
            "level" | "colour" | "size" | "author" | "level_notes" => {
                if let Some(b) = builder.as_mut() {
                    b.apply_kv(&section, trimmed);
                }
            }
            _ => {}
        }
    }

    flush_level(&mut levels, &mut builder, default_colour)?;

    if name.is_empty() {
        name = "Unnamed".to_string();
    }

    Ok(LevelPack {
        name,
        default_colour,
        levels,
    })
}

#[derive(Default)]
struct LevelBuilder {
    index: u32,
    width: u16,
    height: u16,
    colour: u32,
    author: String,
    notes: String,
    data_rows: Vec<String>,
    additional_lines: Vec<String>,
    screws_override: Option<u32>,
}

impl LevelBuilder {
    fn apply_kv(&mut self, key: &str, value: &str) {
        match key {
            "level" => self.index = value.parse().unwrap_or(1),
            "colour" => self.colour = parse_hex_colour(value),
            "size" => {
                if let Some((w, h)) = value.split_once('.') {
                    self.width = w.parse().unwrap_or(16);
                    self.height = h.parse().unwrap_or(31);
                }
            }
            "author" => self.author = value.to_string(),
            "level_notes" => self.notes = value.to_string(),
            _ => {}
        }
    }

    fn finish_grid(self, default_colour: u32) -> FormatResult<Level> {
        let height = self.height;
        let width = self.width;
        if self.data_rows.len() != height as usize {
            return Err(FormatError::RowCountMismatch {
                expected: height,
                got: self.data_rows.len(),
            });
        }

        let mut tiles = Vec::with_capacity((width as usize) * (height as usize));
        let mut elements = Vec::new();
        let mut screw_count = 0u32;

        for (row_idx, row) in self.data_rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            if chars.len() != width as usize {
                return Err(FormatError::Parse(format!(
                    "row {} width {} expected {}",
                    row_idx,
                    chars.len(),
                    width
                )));
            }
            for (col_idx, c) in chars.iter().enumerate() {
                let (tile, element) = tile_or_element(*c)?;
                tiles.push(tile);
                if let Some(el) = element {
                    if matches!(el.kind, robbo_core::ElementKind::Screw) {
                        screw_count += 1;
                    }
                    elements.push((cell_from_grid(col_idx, row_idx), el));
                }
            }
        }

        for line in &self.additional_lines {
            apply_additional_line(&mut elements, line);
        }

        let mut barrier_directions = HashMap::new();
        for row in 0..height as i16 {
            for col in 0..width as i16 {
                let cell = Cell::new(col, row);
                let idx = row as usize * width as usize + col as usize;
                if tiles[idx] == TileKind::Barrier {
                    barrier_directions.insert(cell, Direction::Right);
                }
            }
        }
        for line in &self.additional_lines {
            let parts: Vec<&str> = line.split('.').collect();
            if parts.len() >= 4 && parts[2] == "=" {
                if let (Ok(col), Ok(row), Ok(dir)) = (
                    parts[0].parse::<i16>(),
                    parts[1].parse::<i16>(),
                    parts[3].parse::<u8>(),
                ) {
                    let cell = Cell::new(col, row);
                    barrier_directions.insert(
                        cell,
                        crate::char_map::direction_from_gnurobbo(dir),
                    );
                }
            }
        }

        let colour = if self.colour == 0 {
            default_colour
        } else {
            self.colour
        };

        Ok(Level {
            index: self.index,
            width,
            height,
            colour,
            author: self.author,
            notes: self.notes,
            tiles,
            elements,
            required_screws: self.screws_override.unwrap_or(screw_count),
            barrier_directions,
        })
    }
}

fn parse_hex_colour(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SAMPLE: &str = r#"[name]
TestPack

[last_level]
1

[default_level_colour]
608050

[level]
1

[colour]
555555

[size]
5.5

[author]
test

[level_notes]

[data]
.....
.RT..
.....
.....
.....
"#;

    #[test]
    fn parse_sample_level() {
        let pack = parse_pack_str(SAMPLE).expect("parse");
        assert_eq!(pack.name, "TestPack");
        assert_eq!(pack.levels.len(), 1);
        assert_eq!(pack.levels[0].width, 5);
    }

    #[test]
    fn parse_original_dat() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/levels/original.dat");
        if let Ok(bytes) = fs::read(path) {
            let pack = parse_pack(&bytes).expect("parse original.dat");
            assert!(!pack.levels.is_empty(), "expected levels in original.dat");
        }
    }

    #[test]
    fn original_level4_bird_defaults_to_east() {
        use robbo_core::{BirdVariant, Cell, Direction, ElementKind};

        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/levels/original.dat");
        let bytes = fs::read(path).expect("read original.dat");
        let pack = parse_pack(&bytes).expect("parse original.dat");
        let level = pack
            .levels
            .iter()
            .find(|l| l.index == 4)
            .expect("level 4");
        let bird = level
            .elements
            .iter()
            .find(|(_, s)| matches!(s.kind, ElementKind::Bird { .. }))
            .expect("bird on level 4");
        assert_eq!(bird.0, Cell::new(4, 1));
        assert_eq!(bird.1.direction, Direction::Right);
        assert!(matches!(
            bird.1.kind,
            ElementKind::Bird {
                variant: BirdVariant::Horizontal,
                shooting: false,
            }
        ));
    }
}
