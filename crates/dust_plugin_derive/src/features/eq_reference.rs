use dust_dart_emit::library_declaration_names;
use dust_ir::{CombinatorKindIr, DartFileIr, ImportIr};

/// Collection equality type exported by supported runtime barrels.
const HELPER: &str = "DeepCollectionEquality";
/// Public libraries that provably export Dust's collection equality type.
const HELPER_EXPORTS: &[&str] = &[
    "package:collection/collection.dart",
    "package:dust_dart/derive.dart",
    "package:dust_dart/serde.dart",
    "package:dust_dart/db.dart",
    "package:dust_dart/http.dart",
    "package:dust_dart/dust_dart.dart",
];

/// Returns the proven visible reference to Dust's collection equality helper.
pub(crate) fn equality_reference(library: &DartFileIr) -> Option<String> {
    // Low-level plugin tests construct semantic IR without source directives.
    if library.import_directives.is_empty() {
        return Some(HELPER.to_owned());
    }

    let declarations = library_declaration_names(library);
    let candidates = library
        .import_directives
        .iter()
        .filter(|import| helper_is_visible(import));

    if !declarations.contains(HELPER) && candidates.clone().any(|import| import.prefix.is_none()) {
        return Some(HELPER.to_owned());
    }

    candidates
        .filter_map(|import| import.prefix.as_deref())
        .find(|prefix| !declarations.contains(*prefix))
        .map(|prefix| format!("{prefix}.{HELPER}"))
}

/// Returns whether one import proves the helper is synchronously visible.
fn helper_is_visible(import: &ImportIr) -> bool {
    if import.is_deferred || !HELPER_EXPORTS.contains(&import.uri.as_str()) {
        return false;
    }
    if let Some(namespace) = &import.namespace {
        if !namespace.conditional_uris.is_empty() {
            return false;
        }
        return namespace
            .combinators
            .iter()
            .all(|clause| match clause.kind {
                CombinatorKindIr::Show => clause.names.iter().any(|name| name == HELPER),
                CombinatorKindIr::Hide => clause.names.iter().all(|name| name != HELPER),
            });
    }
    (import.show.is_empty() || import.show.iter().any(|name| name == HELPER))
        && import.hide.iter().all(|name| name != HELPER)
}

#[cfg(test)]
mod tests {
    use dust_ir::{NameIr, SpanIr, TopLevelVariableIr, TypeIr};
    use dust_text::{FileId, TextRange};

    use super::*;

    #[test]
    fn qualifies_prefix_only_and_known_reexported_imports() {
        let prefixed = library_with([import("package:dust_dart/derive.dart", Some("d"))]);
        assert_eq!(
            equality_reference(&prefixed).as_deref(),
            Some("d.DeepCollectionEquality")
        );

        let reexported = library_with([import("package:dust_dart/dust_dart.dart", Some("dust"))]);
        assert_eq!(
            equality_reference(&reexported).as_deref(),
            Some("dust.DeepCollectionEquality")
        );
    }

    #[test]
    fn preserves_unprefixed_output_for_mixed_imports() {
        let library = library_with([
            import("package:dust_dart/derive.dart", Some("d")),
            import("package:dust_dart/derive.dart", None),
        ]);
        assert_eq!(equality_reference(&library).as_deref(), Some(HELPER));
    }

    #[test]
    fn respects_visibility_and_rejects_unproven_barrels() {
        let mut hidden = import("package:dust_dart/derive.dart", Some("d"));
        hidden.hide.push(HELPER.to_owned());
        assert_eq!(equality_reference(&library_with([hidden])), None);

        let unknown = library_with([import("package:app/annotations.dart", Some("d"))]);
        assert_eq!(equality_reference(&unknown), None);
    }

    #[test]
    fn uses_prefixed_helper_when_local_declaration_shadows_unprefixed_name() {
        let mut library = library_with([
            import("package:dust_dart/derive.dart", None),
            import("package:dust_dart/derive.dart", Some("d")),
        ]);
        library.variables.push(TopLevelVariableIr {
            name: NameIr {
                source: HELPER.to_owned(),
                short: HELPER.to_owned(),
                prefix: None,
                span: span(),
            },
            ty: TypeIr::dynamic(),
            initializer: None,
            annotations: Vec::new(),
            span: span(),
        });
        assert_eq!(
            equality_reference(&library).as_deref(),
            Some("d.DeepCollectionEquality")
        );
    }

    fn library_with<const N: usize>(imports: [ImportIr; N]) -> DartFileIr {
        let mut library =
            DartFileIr::empty(".", "test", "lib/model.dart", "lib/model.g.dart", span());
        library.import_directives.extend(imports);
        library
    }

    fn import(uri: &str, prefix: Option<&str>) -> ImportIr {
        ImportIr {
            namespace: None,
            uri: uri.to_owned(),
            prefix: prefix.map(str::to_owned),
            show: Vec::new(),
            hide: Vec::new(),
            is_deferred: false,
            span: span(),
        }
    }

    fn span() -> SpanIr {
        SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
    }
}
