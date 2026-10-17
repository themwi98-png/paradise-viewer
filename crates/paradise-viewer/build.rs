//! Builds the viewer page (every file in viewer/) into the exe: writes
//! `viewer_files.rs`, a table of (path inside viewer/, contents).

use std::path::{Path, PathBuf};

fn main() {
    let viewer = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../viewer").canonicalize().expect("viewer/");
    println!("cargo:rerun-if-changed={}", viewer.display());
    let mut files = Vec::new();
    collect(&viewer, &mut files);
    files.sort();
    let mut out = String::from("/// The viewer page's files: (path inside viewer/, contents).\npub static VIEWER: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let rel = f.strip_prefix(&viewer).unwrap().to_string_lossy().replace('\\', "/");
        out += &format!("    ({rel:?}, include_bytes!({:?})),\n", f.to_string_lossy());
    }
    out += "];\n";
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("viewer_files.rs"), out).unwrap();
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
