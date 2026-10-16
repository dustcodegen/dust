//! Exact shared static-literal values and deliberately unsupported boundaries.

use dust_dart_syntax::parse_static_dart_string_literal as parse;

/// One corpus is evaluated by Rust and both real Dart SDK endpoints.
const VALUES: &str = include_str!("../../../fixtures/static_strings/values.txt");

#[test]
fn real_dart_utf16_only_values_are_explicitly_unsupported() {
    let corpus = include_str!("../../../fixtures/static_strings/utf16_only.txt");
    for record in corpus.trim_end_matches('\n').split("\n---\n") {
        let (header, literal) = record.split_once('\n').unwrap();
        let (_, units) = header.split_once('\t').unwrap();
        let units = units
            .split_whitespace()
            .map(|unit| u16::from_str_radix(unit, 16).unwrap())
            .collect::<Vec<_>>();
        assert!(String::from_utf16(&units).is_err());
        assert_eq!(parse(literal), None);
    }
}

#[test]
fn decodes_the_shared_real_dart_corpus() {
    for record in VALUES.trim_end_matches('\n').split("\n---\n") {
        let (header, literal) = record.split_once('\n').unwrap();
        let fields = header.split('\t').collect::<Vec<_>>();
        let name = fields[0];
        for (index, newline) in ["\n", "\r\n", "\r"].into_iter().enumerate() {
            let expected = fields
                .get(index + 1)
                .unwrap_or(&fields[1])
                .split_whitespace()
                .map(|unit| u16::from_str_radix(unit, 16).unwrap())
                .collect::<Vec<_>>();
            let literal = literal.replace('\n', newline);
            assert_eq!(
                parse(&literal).map(|value| value.encode_utf16().collect::<Vec<_>>()),
                Some(expected),
                "{name} ({newline:?}): {literal:?}"
            );
        }
    }
}

#[test]
fn rejects_malformed_and_nonstatic_literals_without_lossy_values() {
    for literal in [
        "",
        "identifier",
        "'",
        "'''",
        r"'''\",
        "'unterminated",
        "'a\"",
        r"'\",
        r"'\x'",
        r"'\x0'",
        r"'\xgg'",
        r"'\u'",
        r"'\u123'",
        r"'\u012z'",
        r"'\u{}'",
        r"'\u{1'",
        r"'\u{z}'",
        r"'\u{0000000}'",
        r"'\u{110000}'",
        r"'\uD800'",
        r"'\uDC00'",
        r"'\uD800a'",
        r"'\uD800\uD800'",
        r"'\u{DFFF}'",
        r"'$table'",
        r"'${1}'",
        r"'\u005c$table'",
        "'a' 'b'",
        "'a' // tail",
        "R'raw'",
        "'a\nb'",
        "r'a\rb'",
        "'a\\\nb'",
        "'a\\\rb'",
        "r'a\r\nb'",
        "\"\"\"a\"\"\"\"",
    ] {
        assert_eq!(parse(literal), None, "{literal:?}");
    }
}
