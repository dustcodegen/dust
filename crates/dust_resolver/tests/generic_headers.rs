//! Resolution must preserve parser facts before generic emission consumes them.

use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText};

#[test]
fn preserves_generic_header_facts_for_traditional_and_primary_models() {
    for text in [
        "class Box<T extends Comparable<T>> with Audit<T>, _$Box<T> { const Box(this.value); final T value; }",
        "class const Box<T extends Comparable<T>>(final T value) with Audit<T>, _$Box<T>;",
    ] {
        let source = SourceText::new(FileId::new(93), text);
        let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
        assert_eq!(parsed.diagnostics, []);
        let parsed_class = &parsed.library.classes[0];
        assert_eq!(parsed_class.type_parameters.len(), 1);
        assert_eq!(parsed_class.mixins.len(), 2);
        let resolved = resolve_library(
            FileId::new(93),
            "lib/box.dart",
            "lib/box.g.dart",
            &parsed.library,
            &SymbolCatalog::new(),
        );
        assert_eq!(resolved.diagnostics, []);
        assert_eq!(
            resolved.library.classes[0].type_parameters,
            parsed_class.type_parameters
        );
        assert_eq!(resolved.library.classes[0].mixins, parsed_class.mixins);
    }
}
