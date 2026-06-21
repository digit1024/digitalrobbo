use crate::char_map::{element_to_char, tile_to_char};
use crate::pack::{Level, LevelPack};

pub fn serialize_pack(pack: &LevelPack) -> String {
    let mut out = String::new();
    out.push_str("[name]\n");
    out.push_str(&pack.name);
    out.push('\n');
    out.push_str("\n[last_level]\n");
    out.push_str(&pack.levels.len().to_string());
    out.push('\n');
    out.push_str("\n[default_level_colour]\n");
    out.push_str(&format!("{:06x}", pack.default_colour));
    out.push('\n');

    for level in &pack.levels {
        out.push_str("\n[level]\n");
        out.push_str(&level.index.to_string());
        out.push('\n');
        out.push_str("\n[colour]\n");
        out.push_str(&format!("{:06x}", level.colour));
        out.push('\n');
        out.push_str("\n[size]\n");
        out.push_str(&format!("{}.{}", level.width, level.height));
        out.push('\n');
        out.push_str("\n[author]\n");
        out.push_str(&level.author);
        out.push('\n');
        out.push_str("\n[level_notes]\n");
        out.push_str(&level.notes);
        out.push('\n');
        out.push_str("\n[data]\n");
        for row in 0..level.height as usize {
            for col in 0..level.width as usize {
                let idx = row * level.width as usize + col;
                let cell = robbo_core::Cell::new(col as i16, row as i16);
                if let Some((_, el)) = level
                    .elements
                    .iter()
                    .find(|(c, s)| *c == cell && !s.hidden)
                {
                    out.push(element_to_char(&el.kind));
                } else {
                    out.push(tile_to_char(level.tiles[idx]));
                }
            }
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::parse_pack_str;

    #[test]
    fn round_trip_sample() {
        let sample = include_str!("../../robbo-formats/tests/fixtures/sample.dat");
        let pack = parse_pack_str(sample).expect("parse");
        let serialized = serialize_pack(&pack);
        let reparsed = parse_pack_str(&serialized).expect("reparse");
        assert_eq!(pack.levels.len(), reparsed.levels.len());
        assert_eq!(pack.levels[0].width, reparsed.levels[0].width);
    }
}
