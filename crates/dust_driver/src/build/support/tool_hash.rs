use super::registry::RegistrySelection;

/// Stable hash of the active code generation logic and plugin set.
#[derive(Clone, Copy)]
pub(crate) struct CodegenToolHash {
    /// FNV-1a hash value used in cache entries.
    hash: u64,
}

impl CodegenToolHash {
    /// Returns the raw hash value persisted in cache metadata.
    pub(crate) fn value(self) -> u64 {
        self.hash
    }
}

/// Generated manifest of source and template inputs that affect Dart output.
const CODEGEN_TOOL_FINGERPRINT_INPUT: &str =
    include_str!(concat!(env!("OUT_DIR"), "/codegen_tool_fingerprint.txt"));

/// Hashes UTF-8 text using Dust's stable cache hash algorithm.
pub(crate) fn hash_text(text: &str) -> u64 {
    hash_bytes(text.as_bytes())
}

/// Hashes raw bytes using FNV-1a.
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603_u64;
    update_hash_bytes(&mut hash, bytes);
    hash
}

/// Mixes bytes into an existing FNV-1a hash state.
fn update_hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = (*hash).wrapping_mul(1099511628211);
    }
}

/// Computes the codegen tool hash for the selected plugin registry mode.
pub(crate) fn codegen_tool_hash_for_selection(selection: RegistrySelection) -> CodegenToolHash {
    codegen_tool_hash_for_selection_with_input(selection, CODEGEN_TOOL_FINGERPRINT_INPUT)
}

/// Computes a codegen hash from an explicit fingerprint manifest.
fn codegen_tool_hash_for_selection_with_input(
    selection: RegistrySelection,
    fingerprint_input: &str,
) -> CodegenToolHash {
    let mut hash = 1469598103934665603_u64;
    let cache_salt = selection.cache_salt();
    update_hash_bytes(&mut hash, cache_salt.as_bytes());
    update_hash_bytes(&mut hash, b"\0");
    update_hash_bytes(&mut hash, fingerprint_input.as_bytes());
    CodegenToolHash { hash }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_hash_changes_when_generated_fingerprint_changes() {
        let original =
            codegen_tool_hash_for_selection_with_input(RegistrySelection::All, "manifest:a");
        let changed =
            codegen_tool_hash_for_selection_with_input(RegistrySelection::All, "manifest:b");

        assert_ne!(original.value(), changed.value());
    }

    #[test]
    fn tool_hash_preserves_registry_mode_salt() {
        let normal =
            codegen_tool_hash_for_selection_with_input(RegistrySelection::All, "same-manifest");
        let db_only = codegen_tool_hash_for_selection_with_input(
            RegistrySelection::for_build(crate::request::DbRequestOptions {
                only_db: true,
                offline: false,
            }),
            "same-manifest",
        );

        assert_ne!(normal.value(), db_only.value());
    }

    #[test]
    fn generated_fingerprint_manifest_covers_sources_and_templates() {
        assert!(CODEGEN_TOOL_FINGERPRINT_INPUT.contains("crates/dust_driver/src/lower.rs"));
        assert!(CODEGEN_TOOL_FINGERPRINT_INPUT.contains("crates/dust_plugin_derive/src/emit.rs"));
        assert!(
            CODEGEN_TOOL_FINGERPRINT_INPUT
                .contains("crates/dust_plugin_derive/src/features/templates/debug_fields.jinja")
        );
        assert!(
            CODEGEN_TOOL_FINGERPRINT_INPUT
                .contains("crates/dust_http_client_plugin/src/plugin/emit/test_file.rs")
        );
        assert!(CODEGEN_TOOL_FINGERPRINT_INPUT.contains("crates/dust_dart_syntax/src/lib.rs"));
    }

    #[test]
    fn grammar_fingerprint_contains_exact_active_inputs_and_bytes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let mut paths = Vec::new();
        for entry in CODEGEN_TOOL_FINGERPRINT_INPUT
            .lines()
            .skip(1)
            .filter(|entry| entry.starts_with("vendor/"))
        {
            let fields = entry.split('\0').collect::<Vec<_>>();
            assert_eq!(fields.len(), 3);
            let bytes = std::fs::read(root.join(fields[0])).unwrap();
            assert_eq!(fields[1], bytes.len().to_string());
            assert_eq!(fields[2], format!("{:016x}", hash_bytes(&bytes)));
            paths.push(fields[0]);
        }
        assert_eq!(
            paths,
            [
                "vendor/tree-sitter-dart/Cargo.toml",
                "vendor/tree-sitter-dart/UPSTREAM.json",
                "vendor/tree-sitter-dart/bindings/rust/build.rs",
                "vendor/tree-sitter-dart/bindings/rust/lib.rs",
                "vendor/tree-sitter-dart/grammar.js",
                "vendor/tree-sitter-dart/src/node-types.json",
                "vendor/tree-sitter-dart/src/parser.c",
                "vendor/tree-sitter-dart/src/scanner.c",
                "vendor/tree-sitter-dart/src/tree_sitter/alloc.h",
                "vendor/tree-sitter-dart/src/tree_sitter/array.h",
                "vendor/tree-sitter-dart/src/tree_sitter/parser.h",
            ]
        );
    }

    #[test]
    fn native_only_grammar_change_invalidates_all_registry_hashes() {
        let path = "vendor/tree-sitter-dart/src/scanner.c";
        let entry = CODEGEN_TOOL_FINGERPRINT_INPUT
            .lines()
            .find(|entry| entry.split('\0').next() == Some(path))
            .expect("active native grammar must be fingerprinted");
        let mut fields = entry.split('\0');
        assert_eq!(fields.next(), Some(path));
        let size = fields.next().unwrap();
        let digest = fields.next().unwrap();
        let changed_digest = format!("{:016x}", u64::from_str_radix(digest, 16).unwrap() ^ 1);
        let changed_entry = format!("{path}\0{size}\0{changed_digest}");
        let changed_manifest = CODEGEN_TOOL_FINGERPRINT_INPUT.replace(entry, &changed_entry);
        for selection in [
            RegistrySelection::All,
            RegistrySelection::for_build(crate::request::DbRequestOptions {
                only_db: true,
                offline: false,
            }),
        ] {
            let original = codegen_tool_hash_for_selection_with_input(
                selection,
                CODEGEN_TOOL_FINGERPRINT_INPUT,
            );
            let changed = codegen_tool_hash_for_selection_with_input(selection, &changed_manifest);
            assert_ne!(original.value(), changed.value());
        }
    }
}
