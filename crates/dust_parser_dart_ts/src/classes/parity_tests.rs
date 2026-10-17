//! Compare complete traditional class facts, including source spans.

use dust_parser_dart::ParsedClassSurface;
use dust_text::{FileId, SourceText};
use tree_sitter::Parser;

/// Each harness links exactly one grammar: both export the same native symbol.
fn classes(source: &SourceText) -> Vec<ParsedClassSurface> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source.as_str(), None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    let expected_kind = if env!("CARGO_PKG_NAME") == "dust_legacy_parser_parity" {
        "class_declaration"
    } else {
        "class_definition"
    };
    let mut cursor = tree.root_node().walk();
    let class = tree
        .root_node()
        .named_children(&mut cursor)
        .find(|node| matches!(node.kind(), "class_declaration" | "class_definition"))
        .unwrap();
    assert_eq!(
        class.kind(),
        expected_kind,
        "the harness linked the wrong grammar"
    );
    super::extract_classes(tree.root_node(), source)
}

/// Emits full facts for the local cross-process parity gate, never empty success.
fn assert_parity(case: &str, text: &str, count: usize) {
    let source = SourceText::new(FileId::new(0), text);
    let classes = classes(&source);
    assert_eq!(classes.len(), count);
    println!("DUST_PARSER_PARITY:{case}:{classes:?}");
}

#[test]
fn named_mixin_applications_keep_complete_header_facts() {
    assert_parity(
        "named-mixins",
        "class Base<T> {} mixin Mix<T> {} @Tag() abstract class Alias<T /* binder */ extends /* bound */ Object> = Base<List<T>> with Mix<T>;",
        2,
    );
}

#[test]
fn traditional_classes_fields_and_constructors() {
    assert_parity(
        "traditional",
        r#"
class User<T> {
  const User(this.id, {this.tags = const {}, this.page = 1});
  const User.named(this.id, [this.page = 2, this.tags = const {}]);
  final String id;
  final Map<String, List<T?>> tags;
  final int page;
  int count = 0, maximum = 10;
}
"#,
        1,
    );
}

#[test]
fn class_modifiers_inheritance_and_super_parameters() {
    assert_parity(
        "modifiers",
        r#"
mixin Audit {}
abstract interface class Contract {}
abstract mixin class Shared {}
sealed class Base { const Base(this.id); final String id; }
final class Child extends Base with Audit {
  const Child(super.id);
}
class Tagged with Audit {}
"#,
        5,
    );
}

#[test]
fn metadata_and_comments_stay_with_their_declarations() {
    assert_parity(
        "metadata",
        r#"
@d.Derive([d.ToString()])
class User {
  @Deprecated('legacy')
  @d.SerDe(rename: 'user_id', defaultValue: null)
  /// Metadata remains attached across comments.
  final String? id = null;
  @Deprecated('old')
  const User(this.id);
  @d.Query('SELECT 1')
  // A comment is trivia, not a declaration.
  Future<int> load(@d.Validate() String value);
  Future<void> unannotated();
}
"#,
        1,
    );
}

#[test]
fn methods_bodies_modifiers_and_parameter_groups() {
    assert_parity(
        "methods",
        r#"
abstract class Methods {
  external static Future<void> load();
  static int answer() => 42;
  String describe() { return 'ok'; }
  bool operator ==(Object other) => identical(this, other);
  Future<List<String?>> query(String sql, {required int limit, bool active = true});
  void optional([String name = 'guest']) {}
}
"#,
        1,
    );
}

#[test]
fn factories_and_redirecting_targets() {
    assert_parity(
        "factories",
        r#"
abstract class User<T> {
  const factory User(String id, {int page}) = impl.User<T>.named;
  factory User.empty() => throw UnimplementedError();
  factory User.other(String id) { throw UnimplementedError(); }
}
"#,
        1,
    );
}

#[test]
fn constructor_parameter_annotations_and_nested_defaults() {
    assert_parity(
        "parameters",
        r#"
class User {
  const User(@Tag() this.id, {
    @Tag('page') required this.page,
    @Tag() this.tags = const <String>['a', 'b'],
    @Tag() String? label,
  });
  final String id;
  final int page;
  final List<String> tags;
}
"#,
        1,
    );
}

#[test]
fn operator_names_and_adjacent_bodies() {
    assert_parity(
        "operators",
        r#"
class Bits {
  Bits operator ~() => this;
  Bits operator +(Bits other) => other;
  int operator [](int index) => index;
  void operator []=(int index, int value) {}
  int get value => 1;
  set value(int next) {}
  void nested() { void local() {} }
  void commented() /* signature gap */ { print('commented'); }
  void lineCommented() // signature gap
    => print('line');
  void after() => print('last');
}
"#,
        1,
    );
}
