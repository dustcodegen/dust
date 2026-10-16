use dust_ir::{
    CombinatorKindIr, ConditionalUriIr, ExprSourceIr, NamespaceDirectiveIr, SpanIr,
    VisibilityCombinatorIr,
};
use dust_parser_dart::{ParsedCombinatorKind, ParsedNamespaceDirective, ParsedUriLiteral};
use dust_text::FileId;

/// Preserves an exact URI literal and attaches its owning file identity.
pub(super) fn lower_uri_literal(file_id: FileId, literal: &ParsedUriLiteral) -> ExprSourceIr {
    ExprSourceIr {
        source: literal.source.clone(),
        span: SpanIr::new(file_id, literal.span),
    }
}

/// Copies namespace facts without evaluating conditions or flattening visibility.
pub(super) fn lower_namespace(
    file_id: FileId,
    namespace: &ParsedNamespaceDirective,
) -> NamespaceDirectiveIr {
    NamespaceDirectiveIr {
        default_uri: lower_uri_literal(file_id, &namespace.default_uri),
        conditional_uris: namespace
            .conditional_uris
            .iter()
            .map(|branch| ConditionalUriIr {
                condition: ExprSourceIr {
                    source: branch.condition_source.clone(),
                    span: SpanIr::new(file_id, branch.condition_span),
                },
                uri: lower_uri_literal(file_id, &branch.uri),
                span: SpanIr::new(file_id, branch.span),
            })
            .collect(),
        combinators: namespace
            .combinators
            .iter()
            .map(|combinator| VisibilityCombinatorIr {
                kind: match combinator.kind {
                    ParsedCombinatorKind::Show => CombinatorKindIr::Show,
                    ParsedCombinatorKind::Hide => CombinatorKindIr::Hide,
                },
                names: combinator.names.clone(),
                span: SpanIr::new(file_id, combinator.span),
            })
            .collect(),
    }
}
