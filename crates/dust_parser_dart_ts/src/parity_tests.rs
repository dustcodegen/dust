//! Traditional library facts must survive the grammar migration exactly.

use dust_parser_dart::{ParseBackend, ParseOptions, ParsedDartFileSurface};
use dust_text::{FileId, SourceText};

/// Complete translation scans, including diagnostics and source spans.
mod i18n;
/// Complete query facts, including context and unsafe SQL markers.
mod queries;

/// Exercises the real backend, including its declaration fast path and diagnostics.
fn parse(text: &str) -> ParsedDartFileSurface {
    let source = SourceText::new(FileId::new(0), text);
    let result = super::TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(!result.library.is_empty());
    result.library
}

/// Emits every fact and span for the cross-process exact comparison.
fn contract(case: &str, library: ParsedDartFileSurface) {
    println!("DUST_PARSER_PARITY:{case}:{library:?}");
}

#[test]
fn named_library_and_prefixed_directives() {
    let library = parse(
        r#"
@Deprecated('legacy')
library models.user;
import 'package:app/user.dart' deferred as user show User, UserId hide Internal;
export 'src/public.dart';
part 'user.g.dart';
part of models.user;
"#,
    );
    assert_eq!(library.directives.len(), 5);
    contract("directives", library);
}

#[test]
fn conditional_directives_and_combinators() {
    let library = parse(
        r#"
import 'base.dart' if (dart.library.io) 'io.dart' as platform show Public hide Private;
import 'plain.dart' hide Other;
export 'base.dart' if (dart.library.html) 'web.dart' show Public;
"#,
    );
    assert_eq!(library.directives.len(), 3);
    contract("conditional-directives", library);
}

#[test]
fn uri_part_of() {
    let library = parse("part of 'parent.dart';");
    assert_eq!(library.directives.len(), 1);
    contract("uri-part", library);
}

#[test]
fn unnamed_library_metadata() {
    let library = parse("@Deprecated('old') library;");
    assert_eq!(library.directives.len(), 1);
    contract("unnamed-library", library);
}

#[test]
fn annotated_plain_and_enhanced_enums() {
    let library = parse(
        r#"
@d.SerDe()
enum Role { @d.SerDe(rename: 'staff') staff, owner }
enum Color {
  @Deprecated('red') red('#f00'), blue.named('#00f');
  const Color(this.hex);
  const Color.named(this.hex);
  final String hex;
}
"#,
    );
    assert_eq!(library.enums.len(), 2);
    assert_eq!(library.enums[0].variants.len(), 2);
    assert_eq!(library.enums[1].variants.len(), 2);
    contract("enums", library);
}

#[test]
fn mixins_extensions_and_representations() {
    let library = parse(
        r#"
@Deprecated('mixin')
mixin Auditable {
  @Tag('id') final String auditId;
  int count = 1, maximum = 2;
  @Tag('method')
  void run() { final int local = 1; }
  final String clean;
}
@Deprecated('extension')
extension UserFormatting on User { String get label => 'user'; }
extension on String { int get size => length; }
@Deprecated('extension-type')
extension type const UserId(String value) {}
extension type Token<T>.named(List<T?> value) {}
extension type Box<T>(T value) {}
extension type Gap /* name trivia */ (String value) {}
"#,
    );
    assert_eq!(library.mixins.len(), 1);
    assert_eq!(library.mixins[0].fields.len(), 4);
    assert_eq!(library.extensions.len(), 2);
    assert_eq!(library.extension_types.len(), 4);
    contract("declarations", library);
}

#[test]
fn top_level_variable_types_initializers_and_metadata() {
    let library = parse(
        r#"
@Deprecated('names')
final List<String> names = const ['a', 'b'];
const timeout = 30, retry = 2;
@Tag('counts')
int first = value + 1, second = helper(1, 2);
external String host, backup;
String? label;
int withComment = /* initializer trivia */ source.value;
final title = '🌿';
"#,
    );
    assert_eq!(library.variables.len(), 10);
    contract("variables", library);
}

#[test]
fn functions_and_typedef_shapes() {
    let library = parse(
        r#"
@Deprecated('greet')
String greet(User user, {String prefix = 'Hi'}) => '$prefix ${user.name}';
Future<List<String?>> load([int page = 1]) async => [];
@Deprecated('map')
typedef UserMap = Map<String, List<User?>>;
typedef Callback<T> = void Function(T value);
typedef String LegacyCallback(int value);
"#,
    );
    assert_eq!(library.functions.len(), 2);
    assert_eq!(library.typedefs.len(), 3);
    contract("functions", library);
}
