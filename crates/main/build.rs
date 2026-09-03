//! A build script for generating shader descriptors

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let root: PathBuf = env::var("CARGO_MANIFEST_DIR")
        .expect("couldn't read CARGO_MANIFEST_DIR")
        .into();
    let gfx_root = root.join("gfx");

    let mut output = String::new();

    for entry in gfx_root.read_dir().expect("couldn't read directory") {
        let entry = entry.expect("couldn't read entry");

        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "wgsl") {
            continue;
        }

        let Some(name) = path.file_stem() else {
            continue;
        };

        let mut ident = String::new();
        let mut fallbacked = false;
        for &byte in name.as_encoded_bytes() {
            if byte.is_ascii_alphabetic() {
                ident.push(byte.to_ascii_uppercase() as char);
                fallbacked = false;
            } else if !fallbacked {
                ident.push('_');
                fallbacked = true;
            }
        }

        output.push_str("pub const ");
        output.push_str(&ident);
        output.push_str(": ::wgpu::ShaderModuleDescriptor = ::wgpu::ShaderModuleDescriptor {\n");
        output.push_str("    label: Some(\"fvi:gfx.");
        output.push_str(&ident);
        output.push_str("\"),\n");
        output.push_str("    source: ::wgpu::ShaderSource::Wgsl(::std::borrow::Cow::Borrowed(::std::include_str!(\"");

        for &byte in path.as_os_str().as_encoded_bytes() {
            if byte.is_ascii() && !byte.is_ascii_control() {
                output.push(byte as char);
            } else {
                let (hi, lo) = (b'0' + (byte >> 4), b'0' + (byte & 0x0F));
                output.push_str("\\x");
                output.push(hi as char);
                output.push(lo as char);
            }
        }

        output.push_str("\"))),\n");
        output.push_str("};\n");
    }

    let output_path = root.join("src").join("gfx.g.rs");
    File::create(output_path)
        .expect("couldn't create output file")
        .write_all(output.as_bytes())
        .expect("couldn't write generated source");
}
