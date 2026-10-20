//! The shape of one i18n call, as the lowering reads it.

use super::*;

/// Resolves constant constructor calls across both grammar field layouts.
pub(super) fn const_object_call_shape(
    call: Node<'_>,
    source: &SourceText,
) -> Option<I18nCallShape> {
    let type_node = call
        .child_by_field_name("type")
        .or_else(|| direct_named_child(call, "type_identifier"))?;
    let kind = match node_text(type_node, source).as_str() {
        "Text" => I18nCallKind::HardcodedText,
        "TranslatedText" => {
            let constructor = call
                .child_by_field_name("constructor")
                .or_else(|| direct_named_child(call, "identifier"));
            if constructor.is_some_and(|node| node_text(node, source) == "dynamic") {
                return None;
            }
            I18nCallKind::Translation(I18nTranslationKind::TranslatedText)
        }
        _ => return None,
    };
    Some(I18nCallShape {
        kind,
        span: text_range(call.start_byte(), call.end_byte()),
    })
}

/// Matched i18n call metadata.
pub(super) struct I18nCallShape {
    /// Recognized call kind.
    pub(super) kind: I18nCallKind,
    /// Full source span for the call.
    pub(super) span: TextRange,
}

impl I18nCallShape {
    /// Returns the public translation API kind when this is a translation call.
    pub(super) fn translation_kind(&self) -> Option<I18nTranslationKind> {
        match self.kind {
            I18nCallKind::Translation(kind) => Some(kind),
            I18nCallKind::HardcodedText => None,
        }
    }
}

/// Recognized call kinds relevant to i18n scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum I18nCallKind {
    /// A public i18n translation API call.
    Translation(I18nTranslationKind),
    /// A direct Flutter `Text("literal")` call.
    HardcodedText,
}
