use crate::support::parse;
use dust_parser_dart::{ParameterKind, ParsedLexicalScopeKind as Scope};
use dust_text::TextRange;

#[test]
fn raw_return_types_and_induced_fields_retain_separate_callback_facts() {
    let source = "class const Probe(final bool? predicate(String value)?, final bool Function(String value) explicit);";
    let result = parse(313, source);
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(class.fields.len(), 2);
    let parameter = &class.constructors[0].params[0];
    assert_eq!(parameter.type_source.as_deref(), Some("bool?"));
    let ty = parameter.parsed_type.as_ref().unwrap();
    assert_eq!(ty.source, "bool?");
    assert_eq!(
        &source[ty.span.start().to_usize()..ty.span.end().to_usize()],
        "bool?"
    );
    let formal = parameter.function_formal.as_ref().unwrap();
    assert_eq!(formal.return_type.as_ref(), Some(ty));
    assert!(formal.nullable);
    assert_eq!(formal.name.name, "predicate");
    let name_start = source.find("predicate").unwrap() as u32;
    assert_eq!(formal.name.span, TextRange::new(name_start, name_start + 9));
    assert_eq!(formal.parameters.len(), 1);
    assert_eq!(formal.parameters[0].name.name, "value");
    assert!(formal.parameters[0].is_required);
    assert_eq!(class.fields[0].function_formal.as_ref(), Some(formal));
    assert_eq!(class.fields[1].function_formal, None);
    assert_eq!(class.constructors[0].params[1].function_formal, None);
}

#[test]
fn all_callable_paths_keep_only_owned_parameters_and_generic_bounds() {
    let signature = "bool predicate<T extends Object>(@tag bool next(String value), [int? count])";
    for source in [
        format!("class Probe {{ Probe({signature}); }}"),
        format!("class Probe {{ void call({signature}) {{}} }}"),
        format!("void call({signature}) {{}}"),
    ] {
        let parsed = parse(313, &source);
        assert_eq!(parsed.diagnostics, [], "{source}");
        let formal = if let Some(function) = parsed.library.functions.first() {
            assert_eq!(function.params.len(), 1);
            function.params[0].function_formal.as_ref().unwrap()
        } else {
            let class = &parsed.library.classes[0];
            assert_eq!(class.fields.len(), 0);
            if let Some(method) = class.methods.first() {
                assert_eq!(method.params.len(), 1);
                method.params[0].function_formal.as_ref().unwrap()
            } else {
                assert_eq!(class.constructors[0].params.len(), 1);
                class.constructors[0].params[0]
                    .function_formal
                    .as_ref()
                    .unwrap()
            }
        };
        assert_eq!(formal.type_parameters.len(), 1);
        let binder = &formal.type_parameters[0];
        assert_eq!(binder.name, "T");
        assert_eq!(binder.bound.as_ref().unwrap().source, "Object");
        assert_eq!(
            &source[binder.name_span.start().to_usize()..binder.name_span.end().to_usize()],
            "T"
        );
        assert_eq!(formal.parameters.len(), 2);
        assert_eq!(formal.annotations().len(), 1);
        assert_eq!(formal.annotations()[0].name, "tag");
        assert_eq!(formal.parameters[0].annotations[0].name, "tag");
        let nested = formal.parameters[0].function_formal.as_ref().unwrap();
        assert_eq!(nested.name.name, "next");
        assert_eq!(nested.parameters.len(), 1);
        assert_eq!(nested.parameters[0].name.name, "value");
        assert_eq!(formal.parameters[1].kind, ParameterKind::Positional);
        assert!(!formal.parameters[1].is_required);
        assert_eq!(
            formal.parameters[1].parsed_type.as_ref().unwrap().source,
            "int?"
        );
    }
}

#[test]
fn defaults_do_not_leak_between_owned_signatures() {
    let parsed = parse(
        313,
        "class Probe { Probe({bool predicate(String value) = fallback}); }",
    );
    assert_eq!(parsed.diagnostics, []);
    let outer = &parsed.library.classes[0].constructors[0].params[0];
    assert_eq!(outer.default_value_source.as_deref(), Some("fallback"));
    assert_eq!(
        outer.function_formal.as_ref().unwrap().parameters[0].default_value_source,
        None
    );
    let parsed = parse(
        313,
        "void call(bool predicate({required String value, int? count, String word = 'yes'})) {}",
    );
    assert_eq!(parsed.diagnostics, []);
    let parameters = &parsed.library.functions[0].params[0]
        .function_formal
        .as_ref()
        .unwrap()
        .parameters;
    assert_eq!(
        parameters
            .iter()
            .map(|p| (p.kind, p.is_required))
            .collect::<Vec<_>>(),
        [
            (ParameterKind::Named, true),
            (ParameterKind::Named, false),
            (ParameterKind::Named, false),
        ]
    );
    assert_eq!(parameters[2].default_value_source.as_deref(), Some("'yes'"));
}

#[test]
fn callback_binders_are_visible_only_inside_their_own_signatures() {
    for source in [
        "class Probe(@outside final T callback<T>(@T T value, U nested<U>(@U U item)));",
        "class Probe { Probe(@outside T callback<T>(@T T value, U nested<U>(@U U item))); }",
        "class Probe { void call(@outside T callback<T>(@T T value, U nested<U>(@U U item))) {} }",
        "void call(@outside T callback<T>(@T T value, U nested<U>(@U U item))) {}",
        "void call<V>(@outside T callback<T>(@T T value, U nested<U>(@U U item))) {}",
    ] {
        let parsed = parse(313, source);
        assert_eq!(parsed.diagnostics, []);
        let scopes = &parsed.library.lexical_scopes;
        let t = scopes
            .iter()
            .position(|scope| {
                scope
                    .name
                    .as_ref()
                    .is_some_and(|name| name.name == "callback")
            })
            .unwrap();
        let u = scopes
            .iter()
            .position(|scope| {
                scope
                    .name
                    .as_ref()
                    .is_some_and(|name| name.name == "nested")
            })
            .unwrap();
        assert_eq!(scopes[t].kind, Scope::Callable);
        assert_eq!(scopes[t].declarations[0].name.name, "T");
        assert!(scopes[t].visibility.start().to_usize() > source.find("@outside").unwrap());
        assert!(scopes[t].visibility.start().to_usize() < source.find("@T").unwrap());
        assert_eq!(scopes[u].parent, Some(t));
        assert_eq!(scopes[u].declarations[0].name.name, "U");
        assert!(scopes[u].visibility.start().to_usize() < source.find("@U").unwrap());
    }
}
