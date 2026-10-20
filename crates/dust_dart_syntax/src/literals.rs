/// Parses a single or double quoted Dart string literal without unescaping.
pub fn parse_string_literal(source: &str) -> Option<String> {
    let source = source.trim();
    let source = source
        .strip_prefix('r')
        .or_else(|| source.strip_prefix('R'))
        .unwrap_or(source);
    let bytes = source.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    if source.len() < 2 || first != last || !matches!(first, b'\'' | b'"') {
        return None;
    }
    Some(source[1..source.len() - 1].to_owned())
}

/// Parses a Dart boolean literal.
pub fn parse_bool_literal(source: &str) -> Option<bool> {
    match source.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Parses a static Dart string literal and rejects interpolation.
///
/// Raw strings (`r'...'`) and triple-quoted strings are supported. Adjacent
/// string literal concatenation is intentionally rejected so checked SQL can be
/// validated from one exact source literal. Malformed escapes and unpaired UTF-16
/// surrogates return `None`, never a lossy or guessed value.
pub fn parse_static_dart_string_literal(source: &str) -> Option<String> {
    let source = source.trim();
    let (raw, source) = source
        .strip_prefix('r')
        .map_or((false, source), |s| (true, s));
    let quote = source.chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let delimiter = if source.starts_with(&quote.to_string().repeat(3)) {
        quote.to_string().repeat(3)
    } else {
        quote.to_string()
    };
    let multiline = delimiter.len() == 3;
    let body = &source[delimiter.len()..];
    let mut chars = if multiline {
        skip_first_line(body)
    } else {
        body
    }
    .chars();
    let mut units = Vec::new();
    loop {
        if let Some(tail) = chars.as_str().strip_prefix(&delimiter) {
            return tail
                .trim()
                .is_empty()
                .then(|| String::from_utf16(&units).ok())?;
        }
        let ch = chars.next()?;
        if !raw && ch == '$' {
            return None;
        }
        let code = if !raw && ch == '\\' {
            escaped_code(&mut chars, multiline)?
        } else {
            source_character(ch, &mut chars, multiline)? as u32
        };
        if let Ok(unit) = u16::try_from(code) {
            units.push(unit);
        } else {
            units.extend_from_slice(char::from_u32(code)?.encode_utf16(&mut [0; 2]));
        }
    }
}

/// Dart ignores a whitespace-only first physical line, even in raw literals.
fn skip_first_line(source: &str) -> &str {
    let mut chars = source.chars();
    while let Some(mut ch) = chars.next() {
        if ch == '\\' {
            let Some(escaped) = chars.next() else {
                return source;
            };
            ch = escaped;
        }
        match ch {
            ' ' | '\t' => {}
            '\n' => return chars.as_str(),
            '\r' => return chars.as_str().strip_prefix('\n').unwrap_or(chars.as_str()),
            _ => return source,
        }
    }
    source
}

/// Only physical line breaks are normalized; `\r` stays a carriage return.
fn source_character(ch: char, chars: &mut std::str::Chars<'_>, multiline: bool) -> Option<char> {
    if matches!(ch, '\r' | '\n') {
        if !multiline {
            return None;
        }
        if ch == '\r' && chars.as_str().starts_with('\n') {
            chars.next();
        }
        Some('\n')
    } else {
        Some(ch)
    }
}

/// Escape values can be UTF-16 surrogates; validate pairing only after decoding.
fn escaped_code(chars: &mut std::str::Chars<'_>, multiline: bool) -> Option<u32> {
    Some(match chars.next()? {
        'n' => '\n' as u32,
        'r' => '\r' as u32,
        't' => '\t' as u32,
        'b' => 8,
        'f' => 12,
        'v' => 11,
        'x' => hex_digits(chars, 2)?,
        'u' if chars.as_str().starts_with('{') => {
            chars.next();
            let mut code = 0;
            let mut count = 0;
            loop {
                let ch = chars.next()?;
                if ch == '}' {
                    if count == 0 || code > 0x10ffff {
                        return None;
                    }
                    break code;
                }
                count += 1;
                if count > 6 {
                    return None;
                }
                code = code * 16 + ch.to_digit(16)?;
            }
        }
        'u' => hex_digits(chars, 4)?,
        '\r' | '\n' if !multiline => return None,
        ch => ch as u32,
    })
}

/// Fixed-width escapes consume exactly their declared number of hex digits.
fn hex_digits(chars: &mut std::str::Chars<'_>, count: usize) -> Option<u32> {
    (0..count).try_fold(0, |code, _| Some(code * 16 + chars.next()?.to_digit(16)?))
}

#[cfg(test)]
mod tests {
    use super::{parse_bool_literal, parse_static_dart_string_literal, parse_string_literal};

    #[test]
    fn parses_simple_literals() {
        assert_eq!(parse_string_literal(" 'hello' "), Some("hello".to_owned()));
        assert_eq!(
            parse_string_literal(r#"R"hello""#),
            Some("hello".to_owned())
        );
        assert_eq!(parse_string_literal("''"), Some(String::new()));
        assert_eq!(parse_bool_literal(" true "), Some(true));
        assert_eq!(parse_bool_literal("false"), Some(false));
        assert_eq!(parse_bool_literal("yes"), None);
    }

    #[test]
    fn rejects_invalid_string_literals() {
        assert_eq!(parse_string_literal(""), None);
        assert_eq!(parse_string_literal("'"), None);
        assert_eq!(parse_string_literal("'unterminated"), None);
        assert_eq!(parse_string_literal("'mismatch\""), None);
        assert_eq!(parse_string_literal("aa"), None);
        assert_eq!(parse_string_literal("identifier"), None);
    }

    #[test]
    fn parses_static_dart_strings() {
        assert_eq!(
            parse_static_dart_string_literal(r"r'''SELECT * FROM users WHERE id = $1'''"),
            Some("SELECT * FROM users WHERE id = $1".to_owned())
        );
        assert_eq!(
            parse_static_dart_string_literal("'SELECT * FROM users'"),
            Some("SELECT * FROM users".to_owned())
        );
        assert_eq!(
            parse_static_dart_string_literal("'SELECT * FROM $table'"),
            None
        );
        assert_eq!(
            parse_static_dart_string_literal("'SELECT * ' 'FROM users'"),
            None
        );
        assert_eq!(
            parse_static_dart_string_literal(r#""escaped \" quote""#),
            Some("escaped \" quote".to_owned())
        );
        assert_eq!(
            parse_static_dart_string_literal(r#""line\nnext\t\"quote\"\\path\$1""#),
            Some("line\nnext\t\"quote\"\\path$1".to_owned())
        );
        assert_eq!(
            parse_static_dart_string_literal(r#""\r\b\f\v""#),
            Some("\r\u{0008}\u{000C}\u{000B}".to_owned())
        );
        assert_eq!(
            parse_static_dart_string_literal(r#"r"$raw""#),
            Some("$raw".to_owned())
        );
        assert_eq!(parse_static_dart_string_literal(""), None);
        assert_eq!(parse_static_dart_string_literal("identifier"), None);
        assert_eq!(parse_static_dart_string_literal("'unterminated"), None);
        assert_eq!(parse_static_dart_string_literal(r#""dangling\"#), None);
    }
}
