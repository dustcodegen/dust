use dust_parser_dart::ParsedLexicalScopeKind as Scope;
use dust_text::TextRange;

use crate::support::parse;

#[test]
fn primary_headers_and_body_metadata_share_a_scope_but_outer_metadata_does_not() {
    let source = "@foreign() class Model(@local final int value) { static const local = Object(); @local void call(@local int value) {} }";
    let parsed = parse(313, source);
    assert_eq!(parsed.diagnostics, []);
    assert_eq!(parsed.library.lexical_scopes.len(), 2);
    let scope = &parsed.library.lexical_scopes[1];
    assert_eq!(scope.parent, Some(0));
    assert_eq!(scope.kind, Scope::Type);
    assert_eq!(scope.name.as_ref().unwrap().name, "Model");
    assert_eq!(scope.span, TextRange::new(0_u32, source.len() as u32));
    let start = source.find('(').unwrap();
    assert!(scope.visibility.start().to_usize() > start);
    assert_eq!(
        scope.visibility.start().to_usize(),
        source.find("(@local").unwrap()
    );
    assert_eq!(
        scope
            .declarations
            .iter()
            .map(|decl| decl.name.name.as_str())
            .collect::<Vec<_>>(),
        ["value", "local", "call"]
    );
}

#[test]
fn shorthand_callback_declaring_parameters_keep_their_member_names() {
    let source = "class Model(final void marker()) { @marker void call() {} }";
    let parsed = parse(313, source);
    assert_eq!(parsed.diagnostics, []);
    assert_eq!(
        parsed.library.lexical_scopes[1]
            .declarations
            .iter()
            .map(|decl| decl.name.name.as_str())
            .collect::<Vec<_>>(),
        ["marker", "call"]
    );
}
