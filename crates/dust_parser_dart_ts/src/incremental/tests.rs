use dust_parser_dart::{DartLanguageVersion, ParseBackend, ParseOptions, ParseResult};
use dust_text::{FileId, SourceText};

use super::*;
use crate::{TreeSitterDartBackend, extract_parse_result, extract_parse_result_with_projection};

/// Common unchanged declarations leave room for bounded identifier and literal edits.
const MODEL: &str = "import 'dart:core';\n\
// Unchanged context keeps small declaration edits below the reuse budget.\n\
// The complete parser surface, diagnostics and byte spans must stay exact.\n\
@Derive([CopyWith()])\n\
class Box1<T> {\n\
  const Box1(this.value);\n\
  final T value;\n\
  static const int marker = 1;\n\
  String label({String word = 'hello'}) => word;\n\
}\n";

/// Expected trees always come from an independent native parser with no old tree.
fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    parser
}

/// The original extraction path remains the complete parser-result oracle.
fn fresh(source: &SourceText, options: ParseOptions, mode: Option<&[&str]>) -> ParseResult {
    let tree = parser().parse(source.as_str(), None);
    extract_parse_result_with_projection(tree, source, options, mode)
}

/// Cache hits still rebuild diagnostics and spans under each current extraction policy.
fn assert_parity(text: &str, index: usize) {
    let backend = TreeSitterDartBackend::new();
    for (offset, version) in [
        DartLanguageVersion::DART_3_0,
        DartLanguageVersion::latest_known(),
    ]
    .into_iter()
    .enumerate()
    {
        let source = SourceText::new(FileId::new((index * 10 + offset + 1) as u32), text);
        let options = ParseOptions {
            language_version: version,
            ..Default::default()
        };
        assert_eq!(
            backend.parse_file(&source, options),
            fresh(&source, options, None)
        );
        for annotations in [&["Derive"][..], &["Other"][..]] {
            assert_eq!(
                backend.parse_namespace_file(&source, options, annotations),
                fresh(&source, options, Some(annotations))
            );
        }
    }
}

#[test]
fn complete_surfaces_match_fresh_across_edits_ids_options_and_projections() {
    TREES.with(|cache| cache.borrow_mut().entries.clear());
    let renamed = MODEL.replace("Box1", "LongerBox27");
    let numbered = renamed.replace("marker = 1", "marker = 12345");
    let unicode = numbered.replace("'hello'", "'မင်္ဂလာပါ café'");
    let invalid = unicode.replace("final T value;", "final T value = ;");
    let gated = unicode.replace("marker = 12345", "marker = 0") + "const optional = [?null];\n";
    assert!(line_edits(MODEL, &renamed).is_some());
    assert!(line_edits(&renamed, &numbered).is_some());
    assert!(line_edits(&numbered, &unicode).is_some());
    assert!(line_edits(&unicode, &invalid).is_some());
    assert!(
        fresh(
            &SourceText::new(FileId::new(99), invalid.as_str()),
            ParseOptions::default(),
            None
        )
        .has_errors()
    );
    for (index, text) in [
        MODEL, &renamed, &numbered, &unicode, &invalid, &unicode, &gated, MODEL,
    ]
    .into_iter()
    .enumerate()
    {
        assert_parity(text, index);
        TREES.with(|cache| {
            let cache = cache.borrow();
            assert!(cache.entries.len() <= MAX_TREES);
            assert!(
                cache
                    .entries
                    .iter()
                    .all(|entry| !entry.tree.root_node().has_error())
            );
            assert!(cache.entries.iter().all(|entry| entry.source != invalid));
        });
    }
}

#[test]
fn crlf_utf8_and_last_line_terminators_keep_complete_fresh_results() {
    TREES.with(|cache| cache.borrow_mut().entries.clear());
    let crlf = MODEL.replace('\n', "\r\n");
    let changed = crlf.replace("'hello'", "'héllo ကမ္ဘာ'");
    let unterminated = changed.trim_end_matches("\r\n");
    let empty_last_line = changed.clone() + "\r\n";
    let edits = line_edits(&crlf, &changed).unwrap();
    assert_eq!(edits.len(), 1);
    let old_line = crlf.lines().nth(8).unwrap();
    let new_line = changed.lines().nth(8).unwrap();
    assert_eq!(
        edits[0].old_end_position,
        Point::new(8, old_line.find("'hello'").unwrap() + 6)
    );
    assert_eq!(
        edits[0].new_end_position,
        Point::new(
            8,
            new_line.find("'héllo ကမ္ဘာ'").unwrap() + 1 + "héllo ကမ္ဘာ".len()
        )
    );
    let last = line_edits(&changed, unterminated).unwrap();
    assert_eq!(last[0].old_end_position, Point::new(10, 0));
    assert_eq!(last[0].new_end_position, Point::new(9, 1));
    let final_lf = changed.trim_end_matches("\r\n").to_owned() + "\n";
    let shortened = line_edits(&changed, &final_lf).unwrap();
    assert_eq!(shortened[0].old_end_position, Point::new(9, 2));
    assert_eq!(shortened[0].new_end_position, Point::new(9, 1));
    assert!(line_edits(&changed, &empty_last_line).is_none());
    for (index, text) in [
        crlf.as_str(),
        changed.as_str(),
        unterminated,
        &empty_last_line,
        &final_lf,
        &changed,
        MODEL,
        "\n",
        "",
        "\n\n",
        "\n",
    ]
    .into_iter()
    .enumerate()
    {
        assert_parity(text, index);
    }
}

#[test]
fn trimmed_edits_preserve_tokens_and_complete_results_at_utf8_boundaries() {
    let numbered = MODEL.replace("marker = 1", "marker = 12345");
    let cases = [
        (MODEL.to_owned(), numbered.clone(), "", "2345"),
        (numbered, MODEL.to_owned(), "2345", ""),
        (
            MODEL.replace("hello", "é"),
            MODEL.replace("hello", "ê"),
            "é",
            "ê",
        ),
        (
            MODEL.replace("hello", "á"),
            MODEL.replace("hello", "ǡ"),
            "á",
            "ǡ",
        ),
    ];
    for (index, (old, new, removed, added)) in cases.into_iter().enumerate() {
        TREES.with(|cache| cache.borrow_mut().entries.clear());
        let edits = line_edits(&old, &new).unwrap();
        assert_eq!(edits.len(), 1);
        let edit = &edits[0];
        assert_eq!(&old[edit.start_byte..edit.old_end_byte], removed);
        assert_eq!(&new[edit.start_byte..edit.new_end_byte], added);
        assert_eq!(edit.old_end_position.row, edit.start_position.row);
        assert_eq!(edit.new_end_position.row, edit.start_position.row);
        assert_parity(&old, index * 2);
        assert_parity(&new, index * 2 + 1);
        TREES.with(|cache| {
            let cache = cache.borrow();
            assert_eq!(cache.entries.len(), 1);
            assert_eq!(cache.entries.front().unwrap().source, new);
        });
    }
}

#[test]
fn multiple_trimmed_edits_keep_old_offsets_and_reverse_application_parity() {
    TREES.with(|cache| cache.borrow_mut().entries.clear());
    let changed = MODEL
        .replace("Box1", "Box123")
        .replace("marker = 1", "marker = 999");
    let edits = line_edits(MODEL, &changed).unwrap();
    assert_eq!(edits.len(), 3);
    assert_eq!(
        edits
            .iter()
            .map(|edit| edit.start_position.row)
            .collect::<Vec<_>>(),
        [4, 5, 7]
    );
    assert!(
        edits
            .windows(2)
            .all(|pair| pair[0].old_end_byte < pair[1].start_byte)
    );
    assert_parity(MODEL, 40);
    assert_parity(&changed, 41);
    assert_parity(MODEL, 42);
}

#[test]
fn incompatible_or_excessive_line_edits_use_fresh_parsing() {
    assert!(line_edits("a\nb\n", "a\n").is_none());
    assert!(line_edits("small\n", "other\n").is_none());
    let small_old = "// a\n".repeat(MAX_EDITS);
    let small_new = "// b\n".repeat(MAX_EDITS);
    assert_eq!(line_edits(&small_old, &small_new).unwrap().len(), MAX_EDITS);
    assert_parity(&small_old, 50);
    assert_parity(&small_new, 51);
    let prefix = "// unchanged padding padding padding padding\n".repeat(64);
    let old = prefix.clone() + &"// a\n".repeat(33);
    let allowed = prefix.clone() + &"// b\n".repeat(32) + "// a\n";
    let rejected = prefix + &"// b\n".repeat(33);
    assert_eq!(line_edits(&old, &allowed).unwrap().len(), MAX_EDITS);
    assert!(line_edits(&old, &rejected).is_none());
    for (index, text) in [&old, &allowed, &rejected, "class Different {}"]
        .into_iter()
        .enumerate()
    {
        assert_parity(text, index);
    }
}

#[test]
fn cache_bounds_exclude_large_dense_and_invalid_trees() {
    TREES.with(|cache| cache.borrow_mut().entries.clear());
    let mut parser = parser();
    for lines in 1..=5 {
        let source = "// line\n".repeat(lines);
        assert!(parse(&mut parser, &source).is_some());
    }
    TREES.with(|cache| {
        let cache = cache.borrow();
        assert_eq!(cache.entries.len(), MAX_TREES);
        assert_eq!(cache.entries.front().unwrap().lines, 5);
        assert_eq!(cache.entries.back().unwrap().lines, 2);
    });
    let large = " ".repeat(MAX_BYTES + 1);
    let dense = "class A{}".repeat(800);
    let invalid = "class Broken {";
    assert!(dense.len() <= MAX_BYTES);
    assert!(
        parse(&mut parser, &dense)
            .unwrap()
            .root_node()
            .descendant_count()
            > MAX_NODES
    );
    assert!(parse(&mut parser, &large).is_some());
    assert!(parse(&mut parser, invalid).unwrap().root_node().has_error());
    TREES.with(|cache| {
        assert_eq!(cache.borrow().entries.len(), MAX_TREES);
        assert!(
            cache
                .borrow()
                .entries
                .iter()
                .all(|entry| entry.source.starts_with("//"))
        );
    });
    let source = SourceText::new(FileId::new(90), large);
    assert_eq!(
        TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default()),
        extract_parse_result(
            parser.parse(source.as_str(), None),
            &source,
            ParseOptions::default()
        )
    );
}
