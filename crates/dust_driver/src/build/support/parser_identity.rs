use std::sync::OnceLock;

/// Parser, syntax/text contracts, native grammar, dependency lock and target inputs.
const PARSER_INPUTS: &str = include_str!(concat!(env!("OUT_DIR"), "/parser_fingerprint.txt"));

/// Independent from plugin/codegen fingerprints, with a versioned wire contract.
pub(crate) fn parser_identity() -> u64 {
    static IDENTITY: OnceLock<u64> = OnceLock::new();
    *IDENTITY.get_or_init(|| {
        let mut hash = super::hash_text("dust-parser-facts-v2\0");
        super::update_hash_bytes(&mut hash, PARSER_INPUTS.as_bytes());
        hash
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_manifest_tracks_parser_native_dependency_and_target_inputs_only() {
        for path in [
            "Cargo.lock",
            "crates/dust_text/src/range.rs",
            "crates/dust_dart_syntax/src/language_version.rs",
            "crates/dust_parser_dart/src/surface.rs",
            "crates/dust_parser_dart_ts/src/lib.rs",
            "crates/dust_driver/src/namespace_sources/parser_cache/mod.rs",
            "crates/dust_driver/src/namespace_sources/parser_cache/frame.rs",
            "crates/dust_driver/src/namespace_sources/loader/input.rs",
            "vendor/tree-sitter-dart/src/parser.c",
            "vendor/tree-sitter-dart/src/scanner.c",
            "vendor/tree-sitter-dart/Cargo.toml",
            "target\0",
        ] {
            assert!(PARSER_INPUTS.contains(path), "{path}");
        }
        assert!(!PARSER_INPUTS.contains("crates/dust_plugin_derive/"));
        assert!(!PARSER_INPUTS.contains("crates/dust_driver/src/build/batch/"));
        assert!(!PARSER_INPUTS.contains("crates/dust_emitter/"));
        assert!(!PARSER_INPUTS.contains("parser_cache/tests"));
        assert_eq!(parser_identity(), parser_identity());
    }
}
