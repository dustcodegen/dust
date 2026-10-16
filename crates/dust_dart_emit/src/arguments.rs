/// Returns the call-site label of a named constructor parameter.
///
/// Dart strips one leading underscore from private initializing/declaring formals;
/// field lookup must continue to use the original name.
pub fn named_argument_name(name: &str) -> &str {
    name.strip_prefix('_').unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::named_argument_name;

    #[test]
    fn preserves_public_labels_and_strips_only_one_underscore() {
        for (source, expected) in [
            ("name", "name"),
            ("_name", "name"),
            ("_display_name", "display_name"),
            ("_日本語", "日本語"),
            ("_$value", "$value"),
            ("__private", "_private"),
        ] {
            assert_eq!(named_argument_name(source), expected);
        }
    }
}
