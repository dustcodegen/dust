//! Build the pinned native grammar without downloading or regenerating sources.

/// Compiles the pinned C sources; no downloads or parser generation are needed.
fn main() {
    let source = std::path::Path::new("src");
    let mut build = cc::Build::new();
    build.include(source);
    // Preserve the upstream build's C compiler settings.
    build
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs");
    #[cfg(target_env = "msvc")]
    build.flag("-utf-8");
    for file in ["parser.c", "scanner.c"] {
        let path = source.join(file);
        build.file(&path);
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=src/tree_sitter");
    build.compile("parser");
}
