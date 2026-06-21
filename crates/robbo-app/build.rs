use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let levels_dir = Path::new(&manifest_dir).join("../../assets/levels");

    let mut filenames: Vec<String> = fs::read_dir(&levels_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", levels_dir.display()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "dat")
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    filenames.sort();

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let dest = Path::new(&out_dir).join("level_packs.rs");

    let mut code = String::from("pub const PACK_FILES: &[(&str, &[u8])] = &[\n");
    for name in &filenames {
        code.push_str(&format!(
            "    (\"{name}\", include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/levels/{name}\"))),\n"
        ));
    }
    code.push_str("];\n");

    fs::write(&dest, code).expect("write level_packs.rs");

    println!("cargo:rerun-if-changed={}", levels_dir.display());
    for name in &filenames {
        println!(
            "cargo:rerun-if-changed={}",
            levels_dir.join(name).display()
        );
    }
}
