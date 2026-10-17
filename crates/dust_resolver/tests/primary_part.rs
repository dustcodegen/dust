//! Source-preserving resolver contract for already-resolved primary parts.

use dust_parser_dart::{ParseBackend, ParseOptions, ParsedDirective};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{BindingContext, PrimaryPart, SymbolCatalog, resolve_library_with_bindings};
use dust_text::{FileId, SourceText, TextRange};

#[test]
fn resolved_selection_preserves_original_directives_and_uri() {
    let file = SourceText::new(
        FileId::new(147),
        "part 'foreign/model.g.dart';\npart r'%6dodel.g.dart';\n@Derive([ToString()]) class Model {}",
    );
    let parsed = TreeSitterDartBackend::new().parse_file(&file, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let library = &parsed.library;
    let ParsedDirective::Part { uri, span, .. } = &library.directives[1] else {
        panic!("part");
    };
    let mut catalog = SymbolCatalog::new();
    catalog.register_trait("ToString", "dust_dart::ToString");
    let bindings = BindingContext::new(&catalog, None);
    assert!(!bindings.has_namespace());
    let bindings = bindings.with_primary_part(PrimaryPart::Resolved { uri, span: *span });
    let result = resolve_library_with_bindings(
        FileId::new(147),
        "lib/model.dart",
        "lib/model.g.dart",
        library,
        &bindings,
        &[],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.part_uri.as_deref(), Some(uri.as_str()));
    assert_eq!(result.library.directives, library.directives);

    for primary in [
        PrimaryPart::Missing,
        PrimaryPart::Resolved {
            uri,
            span: TextRange::empty(0_u32),
        },
        PrimaryPart::Resolved {
            uri: "model.g.dart",
            span: *span,
        },
    ] {
        let bindings = BindingContext::new(&catalog, None).with_primary_part(primary);
        let result = resolve_library_with_bindings(
            FileId::new(147),
            "lib/model.dart",
            "lib/model.g.dart",
            library,
            &bindings,
            &[],
        );
        assert_eq!(result.library.part_uri, None);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].message,
            "missing generated `part` directive for Dust-enabled library"
        );
        let ParsedDirective::Part {
            span: first_span, ..
        } = library.directives[0]
        else {
            panic!("part");
        };
        assert_eq!(result.diagnostics[0].labels[0].range, first_span);
    }
}

#[test]
fn resolved_missing_part_does_not_change_partless_consumers() {
    let source = SourceText::new(FileId::new(147), "@Standalone() class App {}");
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Standalone", "example::Standalone");
    let bindings = BindingContext::new(&catalog, None).with_primary_part(PrimaryPart::Missing);
    let result = resolve_library_with_bindings(
        FileId::new(147),
        "lib/app.dart",
        "lib/app.g.dart",
        &parsed.library,
        &bindings,
        &["example::Standalone"],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.part_uri, None);
}
