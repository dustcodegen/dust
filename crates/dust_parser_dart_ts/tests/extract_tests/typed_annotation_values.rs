//! Listed production grammar contracts for typed annotation callees.

use dust_parser_dart::{ParseBackend, ParseOptions, ParsedAnnotationValueRootKind};
use dust_text::SourceText;

#[test]
fn typed_nested_constructor_shapes() {
    for (written, raw, short, prefix, arguments) in [
        (
            "Validate<Object?>()",
            "Validate<Object?>",
            "Validate",
            None,
            0,
        ),
        (
            "d.Validate<List<Map<other.Type, int?>>>()",
            "d.Validate<List<Map<other.Type, int?>>>",
            "Validate",
            Some("d"),
            0,
        ),
        (
            "const d /* .( */ . Foo<List<int>> . named(1)",
            "d /* .( */ . Foo<List<int>> . named",
            "named",
            Some("d.Foo"),
            1,
        ),
        (
            "d /* ) */ . Validate /* ( */ <int> ()",
            "d /* ) */ . Validate /* ( */ <int>",
            "Validate",
            Some("d"),
            0,
        ),
        (
            "d /* outer /* nested . */ inner */ . Validate<void Function<Q>(Q)>()",
            "d /* outer /* nested . */ inner */ . Validate<void Function<Q>(Q)>",
            "Validate",
            Some("d"),
            0,
        ),
        (
            "const Foo<({int value})>.new()",
            "Foo<({int value})>.new",
            "new",
            Some("Foo"),
            0,
        ),
        (
            "const constPrefix<int>()",
            "constPrefix<int>",
            "constPrefix",
            None,
            0,
        ),
    ] {
        for (before, after) in [("[", "]"), ("", ""), ("value: ", "")] {
            let text = format!("@Meta({before}{written}{after}) class C {{}}");
            let source = SourceText::new(dust_text::FileId::new(0), text.as_str());
            let result = dust_parser_dart_ts::TreeSitterDartBackend::new()
                .parse_file(&source, ParseOptions::default());
            assert_eq!(result.diagnostics, [], "{written}");
            let parsed = result.library.classes[0].annotations[0]
                .parsed_arguments
                .as_ref()
                .unwrap();
            let value = if parsed.named.is_empty() {
                parsed.positional[0].value.as_ref().unwrap()
            } else {
                parsed.named[0].value.as_ref().unwrap()
            };
            let value = if let ParsedAnnotationValueRootKind::List(values) = &value.kind {
                &values[0]
            } else {
                value
            };
            assert_eq!(value.source, written);
            assert_eq!(source.slice(value.span), Some(written));
            let ParsedAnnotationValueRootKind::Constructor {
                name,
                short_name,
                prefix: actual_prefix,
                arguments: args,
            } = &value.kind
            else {
                panic!("expected constructor: {value:?}")
            };
            assert_eq!(
                (name.as_str(), short_name.as_str(), actual_prefix.as_deref()),
                (raw, short, prefix)
            );
            assert_eq!(args.positional.len(), arguments);
            assert_eq!(args.named, []);
        }
    }
}
