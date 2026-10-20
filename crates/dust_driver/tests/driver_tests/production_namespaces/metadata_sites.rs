use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn ownership_preflight_covers_every_represented_metadata_site_and_nested_group() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    let sites = [
        ("library", "MARKER library probe;"),
        ("class", "MARKER class Other {}"),
        ("field", "class Other { MARKER final int value=0; }"),
        ("constructor", "class Other { MARKER const Other(); }"),
        (
            "constructor parameter",
            "class Other { const Other(MARKER int value); }",
        ),
        ("method", "class Other { MARKER void call() {} }"),
        (
            "method parameter",
            "class Other { void call(MARKER int value) {} }",
        ),
        ("enum", "MARKER enum Other { value }"),
        ("enum variant", "enum Other { MARKER value }"),
        ("mixin", "MARKER mixin Other {}"),
        ("mixin field", "mixin Other { MARKER final int value=0; }"),
        ("extension", "MARKER extension Other on int {}"),
        (
            "extension type",
            "MARKER extension type Other(int value) {}",
        ),
        ("typedef", "MARKER typedef Other=String;"),
        ("variable", "MARKER final int other=0;"),
        ("function", "MARKER void other() {}"),
        ("function parameter", "void other(MARKER int value) {}"),
        (
            "constructor callback",
            "class Other { Other(bool predicate(MARKER String value)); }",
        ),
        (
            "method callback",
            "class Other { void call(bool predicate(MARKER String value)) {} }",
        ),
        (
            "function callback",
            "void other(bool predicate(MARKER String value)) {}",
        ),
        (
            "nested callback",
            "void other(bool predicate(bool nested(MARKER String value))) {}",
        ),
    ];
    for annotation in ["@Ghost()", "@Derive([Ghost()])"] {
        for (site, declaration) in sites {
            let declaration = declaration.replace("MARKER", annotation);
            let edited = if site == "library" {
                format!("{declaration}\n{source}")
            } else {
                format!("{source}\n{declaration}")
            };
            write_file(&model, &edited);
            let result = build(root);
            assert!(result.has_errors(), "{site}: {annotation}");
            assert_eq!(
                result
                    .diagnostics
                    .iter()
                    .map(|error| error.message.as_str())
                    .collect::<Vec<_>>(),
                ["annotation name `Ghost` is not visible in this namespace"],
                "{site}: {annotation}"
            );
            assert_eq!(
                fs::read(&output_path).unwrap(),
                output,
                "{site}: {annotation}"
            );
            assert_eq!(
                fs::read(&cache_path).unwrap(),
                cache,
                "{site}: {annotation}"
            );
        }
    }
}
