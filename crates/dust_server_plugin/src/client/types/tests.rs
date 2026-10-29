use std::collections::BTreeSet;

use dust_ir::{DartFileIr, ImportIr, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
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

#[test]
fn type_collection_accepts_public_reachable_shapes_and_rejects_opaque_ones() {
    let mut prefixes = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for ty in [
        TypeIr::string(),
        TypeIr::dynamic(),
        TypeIr::named("String"),
        TypeIr::named("models.Todo"),
        TypeIr::generic("List", vec![TypeIr::named("nested.Item")]),
        TypeIr::named("models._Secret"),
        TypeIr::named("_Local"),
        TypeIr::named("UnprefixedModel"),
        TypeIr::function("void Function()"),
        TypeIr::record("(String, int)"),
        TypeIr::unknown(),
    ] {
        collect_type(&ty, &mut prefixes, &mut diagnostics);
    }

    assert_eq!(
        prefixes,
        BTreeSet::from(["models".to_owned(), "nested".to_owned()])
    );
    assert_eq!(diagnostics.len(), 6);
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("private"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("prefixed package import"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("fully resolved"))
    );
}

#[test]
fn prefix_resolution_requires_one_plain_client_safe_package_import() {
    let mut file = DartFileIr::empty(".", "server", "lib/api.dart", "lib/api.g.dart", span());
    file.import_directives = vec![
        import("package:models/models.dart", Some("models")),
        import("package:a/a.dart", Some("duplicate")),
        import("package:b/b.dart", Some("duplicate")),
    ];
    let mut diagnostics = Vec::new();

    assert_eq!(
        import_for_prefix(&file, "models", &mut diagnostics),
        Some("import 'package:models/models.dart' as models;".to_owned())
    );
    assert_eq!(import_for_prefix(&file, "missing", &mut diagnostics), None);
    assert_eq!(
        import_for_prefix(&file, "duplicate", &mut diagnostics),
        None
    );
    assert_eq!(diagnostics.len(), 2);

    let mut deferred = import("package:models/models.dart", Some("models"));
    deferred.is_deferred = true;
    let mut hidden = import("package:models/models.dart", Some("models"));
    hidden.hide.push("Secret".to_owned());
    let mut shown = import("package:models/models.dart", Some("models"));
    shown.show.push("Todo".to_owned());
    for invalid in [
        import("dart:io", Some("io")),
        import("package:dust_server/server.dart", Some("server")),
        deferred,
        hidden,
        shown,
    ] {
        let mut diagnostics = Vec::new();
        assert_eq!(render_import(&invalid, &mut diagnostics), None);
        assert_eq!(diagnostics.len(), 1);
    }
}
