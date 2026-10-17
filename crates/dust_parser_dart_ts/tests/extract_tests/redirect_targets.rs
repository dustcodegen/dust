use dust_parser_dart::{
    ParsedRedirectIdentifierSurface, ParsedRedirectTargetSurface, ParsedTypeKind, ParsedTypeSurface,
};
use dust_text::TextRange;

use crate::support::parse;

fn span(source: &str, fragment: &str) -> TextRange {
    let start = source.rfind(fragment).unwrap() as u32;
    TextRange::new(start, start + fragment.len() as u32)
}

#[test]
fn redirect_targets_keep_applied_types_constructor_identity_and_exact_spans() {
    for header in ["sealed class Event<T>", "sealed class Event<T>()"] {
        for suffix in ["", ".create", ".new"] {
            let target =
                format!("p /* left */ . /* right */ Done<Map /* args */ <String, T?>>{suffix}");
            let source = format!(
                "/* π */ {header} {{ const factory Event.done(String label) = /* before */ {target} /* after */; }}"
            );
            let parsed = parse(134, &source);
            assert_eq!(parsed.diagnostics, []);
            let constructor = parsed.library.classes[0].constructors.last().unwrap();
            let argument = ParsedTypeSurface {
                source: "Map /* args */ <String, T?>".into(),
                span: span(&source, "Map /* args */ <String, T?>"),
                nullable: false,
                kind: ParsedTypeKind::Named {
                    name: "Map".into(),
                    args: vec![
                        ParsedTypeSurface {
                            source: "String".into(),
                            span: span(&source, "String"),
                            kind: ParsedTypeKind::Builtin("String".into()),
                            nullable: false,
                        },
                        ParsedTypeSurface {
                            source: "T?".into(),
                            span: span(&source, "T?"),
                            kind: ParsedTypeKind::Named {
                                name: "T".into(),
                                args: vec![],
                            },
                            nullable: true,
                        },
                    ],
                },
            };
            let target_fact = constructor
                .redirected_target
                .as_ref()
                .expect("structured redirect target");
            assert_eq!(target_fact.span, span(&source, &target));
            assert_eq!(
                target_fact.ty.source,
                "p /* left */ . /* right */ Done<Map /* args */ <String, T?>>"
            );
            assert_eq!(target_fact.ty.span, span(&source, &target_fact.ty.source));
            assert_eq!(
                target_fact
                    .type_name_parts
                    .iter()
                    .map(|part| part.name.as_str())
                    .collect::<Vec<_>>(),
                ["p", "Done"]
            );
            assert_eq!(target_fact.type_name_parts[1].span, span(&source, "Done"));
            let ParsedTypeKind::Named { name, args } = &target_fact.ty.kind else {
                panic!("named target")
            };
            assert_eq!(name, "p.Done");
            assert_eq!(args, &[argument]);
            let expected_suffix = match suffix {
                "" => None,
                ".create" => Some("create"),
                _ => Some("new"),
            };
            assert_eq!(
                target_fact.constructor,
                expected_suffix.map(|name| ParsedRedirectIdentifierSurface {
                    name: name.into(),
                    span: span(&source, name)
                })
            );
        }
    }
}

#[test]
fn plain_redirects_and_non_redirecting_constructors_do_not_invent_arguments() {
    let source = "class Event { const Event(); factory Event.body() => Event(); factory Event.done() = Done; }";
    let parsed = parse(134, source);
    assert_eq!(parsed.diagnostics, []);
    let constructors = &parsed.library.classes[0].constructors;
    assert_eq!(constructors[0].redirected_target, None);
    assert_eq!(constructors[1].redirected_target, None);
    assert_eq!(
        constructors[2].redirected_target,
        Some(ParsedRedirectTargetSurface {
            type_name_parts: vec![ParsedRedirectIdentifierSurface {
                name: "Done".into(),
                span: span(source, "Done")
            }],
            ty: ParsedTypeSurface {
                source: "Done".into(),
                span: span(source, "Done"),
                kind: ParsedTypeKind::Named {
                    name: "Done".into(),
                    args: vec![]
                },
                nullable: false
            },
            constructor: None,
            span: span(source, "Done"),
        })
    );
}
