/// Returns whether a value can be emitted as a non-reserved Dart identifier.
pub(super) fn is_emittable_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(is_identifier_part) && !is_reserved_word(value)
}

/// Returns whether a character may start a Dart identifier.
fn is_identifier_start(value: char) -> bool {
    value == '_' || value == '$' || value.is_ascii_alphabetic()
}

/// Returns whether a character may continue a Dart identifier.
fn is_identifier_part(value: char) -> bool {
    is_identifier_start(value) || value.is_ascii_digit()
}

/// Returns whether a value is a Dart reserved or contextual word.
fn is_reserved_word(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "base"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "covariant"
            | "default"
            | "deferred"
            | "do"
            | "dynamic"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "extension"
            | "external"
            | "factory"
            | "false"
            | "final"
            | "finally"
            | "for"
            | "Function"
            | "get"
            | "hide"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "interface"
            | "is"
            | "late"
            | "library"
            | "mixin"
            | "new"
            | "null"
            | "of"
            | "on"
            | "operator"
            | "part"
            | "required"
            | "rethrow"
            | "return"
            | "sealed"
            | "set"
            | "show"
            | "static"
            | "super"
            | "switch"
            | "sync"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typedef"
            | "var"
            | "void"
            | "when"
            | "while"
            | "with"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_identifiers_and_rejects_invalid_or_reserved_values() {
        for valid in ["routes", "_routes2", "$routes"] {
            assert!(is_emittable_identifier(valid));
        }
        for invalid in ["", "2routes", "route-name", "switch"] {
            assert!(!is_emittable_identifier(invalid));
        }
    }
}
