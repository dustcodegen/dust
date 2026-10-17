#![cfg(test)]

use super::lower_library;
use dust_ir::{DartFileIr, LoweringOutcome, SpanIr, SuperclassIr, TypeIr};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText, TextRange};

fn lower(text: &str) -> LoweringOutcome<DartFileIr> {
    let source = SourceText::new(FileId::new(133), text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut resolved = resolve_library(
        FileId::new(133),
        "lib/child.dart",
        "lib/child.g.dart",
        &parsed.library,
        &SymbolCatalog::new(),
    );
    assert_eq!(resolved.diagnostics, []);
    for class in &mut resolved.library.classes {
        class.requires_lowering_diagnostics = true;
    }
    lower_library(&mut resolved.library)
}

#[test]
fn inherited_fields_and_super_formals_use_the_actual_superclass_argument() {
    for child in [
        "class Child extends Base<int> { const Child(super.value); }",
        "class const Child(super.value) extends Base<int>;",
    ] {
        let source = format!("class Base<T> {{ const Base(this.value); final T value; }} {child}");
        let outcome = lower(&source);
        assert_eq!(outcome.diagnostics, []);
        let child = &outcome.value.classes[1];
        let start = source.find("Base<int>").unwrap() as u32;
        assert_eq!(
            child.superclass,
            Some(SuperclassIr {
                ty: TypeIr::generic("Base", vec![TypeIr::int()]),
                span: SpanIr::new(FileId::new(133), TextRange::new(start, start + 9)),
            })
        );
        assert_eq!(child.fields[0].ty, TypeIr::int());
        assert_eq!(child.constructors[0].params[0].ty, TypeIr::int());
        assert_eq!(
            child.fields[0].span,
            outcome.value.classes[0].fields[0].span
        );
    }
}

#[test]
fn inheritance_substitutes_reordered_nested_arguments_at_each_edge() {
    let outcome = lower(
        "class Base<A, B> { const Base(this.value); final Map<A, List<B?>> value; }
         class Mid<X, Y> extends Base<Y, X> { const Mid(super.value); }
         class Child extends Mid<int, String> { const Child(super.value); }",
    );
    assert_eq!(outcome.diagnostics, []);
    let child = &outcome.value.classes[2];
    let expected = TypeIr::map_of(TypeIr::string(), TypeIr::list_of(TypeIr::int().nullable()));
    assert_eq!(child.fields[0].ty, expected);
    assert_eq!(child.constructors[0].params[0].ty, expected);
}

#[test]
fn substitution_retains_child_binder_ownership_bounds_and_nullability() {
    let outcome = lower(
        "class Base<T, U> { const Base(this.value, this.other); final List<T?>? value; final U other; }
         class Child<U extends num, T extends List<U>> extends Base<T, U> {
           const Child(super.value, super.other);
         }",
    );
    assert_eq!(outcome.diagnostics, []);
    let child = &outcome.value.classes[1];
    assert_eq!(
        child
            .fields
            .iter()
            .map(|f| f.ty.clone())
            .collect::<Vec<_>>(),
        [
            TypeIr::list_of(TypeIr::named("T").nullable()).nullable(),
            TypeIr::named("U"),
        ]
    );
    assert_eq!(child.type_parameters[0].bound, Some(TypeIr::num()));
    assert_eq!(
        child.type_parameters[1].bound,
        Some(TypeIr::list_of(TypeIr::named("U")))
    );

    let shadowed = lower(
        "class Base<String> { const Base(this.value); final String value; }
         class Child extends Base<int?> { const Child(super.value); }",
    );
    assert_eq!(shadowed.diagnostics, []);
    assert_eq!(
        shadowed.value.classes[1].fields[0].ty,
        TypeIr::int().nullable()
    );
}

#[test]
fn overrides_preserve_field_order_without_substituting_discarded_opaque_types() {
    let outcome = lower(
        "class Base<T> { final int first = 0; final T Function(T) value; const Base(this.value); }
         class Child extends Base<int> { final int last = 1; final int Function(int) value; const Child(this.value) : super(value); }",
    );
    assert_eq!(outcome.diagnostics, []);
    let fields = &outcome.value.classes[1].fields;
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["first", "value", "last"]
    );
    assert_eq!(fields[1].ty, TypeIr::function("int Function(int)"));
}

#[test]
fn prefixed_parent_identity_never_binds_to_a_same_named_local_class() {
    let outcome = lower(
        "class Base<T> { final T value; const Base(this.value); }
         class Child extends p.Base<int> { final int own = 0; }",
    );
    assert_eq!(outcome.diagnostics, []);
    assert_eq!(
        outcome.value.classes[1]
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["own"]
    );
}

#[test]
fn unsupported_substitution_reports_application_and_original_field_spans() {
    for (ty, shape) in [
        ("T Function(T)", "opaque function type"),
        ("(T, int)", "opaque record type"),
        ("Map<String, List<T Function(T)>>", "opaque function type"),
        ("T<int>", "applied type parameter"),
    ] {
        let source = format!(
            "class Base<T> {{ final {ty} value; const Base(this.value); }} class Child extends Base<int> {{ const Child(super.value); }}"
        );
        let outcome = lower(&source);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diagnostic = &outcome.diagnostics[0];
        assert!(diagnostic.is_error());
        assert_eq!(
            diagnostic.message,
            format!("Dust cannot substitute inherited {shape} on `Child.value` through `Base` yet")
        );
        assert_eq!(diagnostic.labels.len(), 2);
        let label = &diagnostic.labels[0];
        assert_eq!(
            &source[label.range.start().to_usize()..label.range.end().to_usize()],
            "Base<int>"
        );
        assert_eq!(
            diagnostic.labels[1].range,
            outcome.value.classes[0].fields[0].span.range
        );
    }
}

#[test]
fn raw_and_wrong_arity_applications_fail_clearly_instead_of_guessing_bounds() {
    for (application, count) in [("Base", 0), ("Base<int, String>", 2)] {
        let outcome = lower(&format!(
            "class Base<T extends num> {{ final T value; const Base(this.value); }} class Child extends {application} {{ const Child(super.value); }}"
        ));
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].message,
            format!(
                "Dust requires 1 explicit superclass argument(s) for `Base` on `Child`; found {count}"
            )
        );
        assert_eq!(outcome.diagnostics[0].labels.len(), 1);
    }
}

#[test]
fn complete_type_arguments_can_be_opaque_without_rewriting_their_source() {
    let outcome = lower(
        "class Base<T> { final T value; const Base(this.value); }
         class Child extends Base<int Function(String)> { const Child(super.value); }",
    );
    assert_eq!(outcome.diagnostics, []);
    assert_eq!(
        outcome.value.classes[1].fields[0].ty,
        TypeIr::function("int Function(String)")
    );

    let concrete = lower(
        "class Base { final int Function(int) value; const Base(this.value); }
         class Child extends Base { const Child(super.value); }",
    );
    assert_eq!(concrete.diagnostics, []);
    assert_eq!(
        concrete.value.classes[1].fields[0].ty,
        TypeIr::function("int Function(int)")
    );
}

#[test]
fn unresolved_inherited_types_fail_but_concrete_dynamic_and_qualified_types_stay_exact() {
    let outcome = lower(
        "class Base<T> { final value; const Base(this.value); }
         class Child extends Base<int> { const Child(super.value); }",
    );
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(
        outcome.diagnostics[0].message,
        "Dust cannot substitute inherited unresolved type on `Child.value` through `Base` yet"
    );

    let known = lower(
        "class Base<T> { final dynamic value; final int count = 0; final p.T? other; const Base(this.value, this.other); }
         class Child extends Base<int> { const Child(super.value, super.other); }",
    );
    assert_eq!(known.diagnostics, []);
    assert_eq!(
        known.value.classes[1]
            .fields
            .iter()
            .map(|f| f.ty.clone())
            .collect::<Vec<_>>(),
        [
            TypeIr::dynamic(),
            TypeIr::int(),
            TypeIr::named("p.T").nullable(),
        ]
    );
}

#[test]
fn actual_arguments_are_substituted_simultaneously_in_the_child_scope() {
    let swapped = lower(
        "class Base<T, U> { final T first; final U second; const Base(this.first, this.second); }
         class Child<T, U> extends Base<U, T> { const Child(super.first, super.second); }",
    );
    assert_eq!(swapped.diagnostics, []);
    assert_eq!(
        swapped.value.classes[1]
            .fields
            .iter()
            .map(|f| f.ty.clone())
            .collect::<Vec<_>>(),
        [TypeIr::named("U"), TypeIr::named("T")]
    );

    let shadowed = lower(
        "class Base<T> { final T value; const Base(this.value); }
         class Child<String> extends Base<String> { const Child(super.value); }",
    );
    assert_eq!(shadowed.diagnostics, []);
    assert_eq!(
        shadowed.value.classes[1].fields[0].ty,
        TypeIr::named("String")
    );
    assert_eq!(
        shadowed.value.classes[1].superclass.as_ref().unwrap().ty,
        TypeIr::generic("Base", vec![TypeIr::named("String")])
    );
}

#[test]
fn superclass_cycles_still_report_the_existing_error() {
    let outcome = lower("class A extends B {} class B extends A {}");
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(
        outcome.diagnostics[0].message,
        "cyclic superclass chain detected while lowering `A`"
    );
    assert!(outcome.diagnostics[0].is_error());
}
