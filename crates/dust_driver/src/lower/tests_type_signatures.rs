#![cfg(test)]

use super::lower_library;
use dust_ir::TypeIr;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText};

#[test]
fn preserves_function_local_binders_through_real_class_member_lowering() {
    let text = "class Box<String> { Box(this.transform); final String Function<String>(String) transform; void accept(String Function<String>(String) value) {} }";
    let source = SourceText::new(FileId::new(99), text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut resolved = resolve_library(
        FileId::new(99),
        "lib/box.dart",
        "lib/box.g.dart",
        &parsed.library,
        &SymbolCatalog::new(),
    );
    assert_eq!(resolved.diagnostics, []);
    let lowered = lower_library(&mut resolved.library);
    assert_eq!(lowered.diagnostics, []);
    let class = &lowered.value.classes[0];
    let expected = TypeIr::function("String Function<String>(String)");
    assert_eq!(class.fields[0].ty, expected);
    assert_eq!(class.constructors[0].params[0].ty, expected);
    assert_eq!(class.methods[0].params[0].ty, expected);
}
