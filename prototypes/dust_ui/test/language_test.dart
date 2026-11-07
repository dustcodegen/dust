import 'package:dust_ui_prototype/language.dart';
import 'package:test/test.dart';

void main() {
  test('parses named properties, visual children, and typed event blocks', () {
    final root = parseUi(r'''
col(id: "product", gap: 12, state: {expanded: false, quantity: 2}) {
  // A trailing comma is legal in property and value lists.
  text(value: "Linen \"shirt\"\nSize M", size: 18, visible: state.expanded,)
  button(onTap: { set(expanded: !state.expanded) }) {
    row(gap: 8) { text(value: "Details") }
  }
  form(post: "/ui/cart/items", target: "cart") {
    input(name: "quantity", type: hidden, value: state.quantity)
    select(name: "size", label: "Size", options: ["S", "M",], initial: "M")
    button(type: submit) { text(value: "Add") }
  }
  col(id: "cart") {}
}
''');
    expect(root.name, 'col');
    expect(root.props['state'], {'expanded': false, 'quantity': 2});
    expect(root.children.first.props['value'], 'Linen "shirt"\nSize M');
    expect(
      root.children.first.props['visible'],
      isA<UiStateRef>().having((value) => value.name, 'name', 'expanded'),
    );
    final block = root.children[1].props['onTap']! as UiActionBlock;
    final action = block.actions.single;
    expect(action.name, 'set');
    expect(
      action.props['expanded'],
      isA<UiStateRef>().having((value) => value.negated, 'negated', true),
    );
    expect(() => block.actions.clear(), throwsUnsupportedError);
    expect(root.children[2].children[1].props['options'], ['S', 'M']);
  });

  test('popups have independent state, form, ID, and target scopes', () {
    final root = parseUi('''
form(id: "cart", post: "/ui/form", target: "cart") {
  button(onTap: {
    popup(label: "Nested form") {
      col(id: "cart", state: {expanded: false}) {
        form(post: "/ui/modal", target: "cart") {
          input(name: "note", label: "Note")
          button(type: submit) { text(value: "Save") }
        }
        button(onTap: { dismiss() }) { text(value: "Close") }
      }
    }
  }) { text(value: "Open") }
  button(type: submit) { text(value: "Submit") }
}
''');
    final block = root.children.first.props['onTap']! as UiActionBlock;
    expect(block.actions.single.name, 'popup');
    expect(
      () => parseUi('''
col(state: {expanded: false}) {
  button(onTap: {
    popup(label: "Independent modal") {
      text(value: "Invalid binding", visible: state.expanded)
    }
  }) { text(value: "Open") }
}
'''),
      throwsA(_diagnostic('Unknown state key')),
    );
  });

  test('reports the property column with CRLF line endings', () {
    expect(
      () => parseUi(
        'col {\r\n  text(value: "Good")\r\n  text(value: "Bad", size: -1)\r\n}',
      ),
      throwsA(
        _diagnostic('Property "size"')
            .having((error) => error.line, 'line', 3)
            .having((error) => error.column, 'column', 22),
      ),
    );
  });

  test('full documents resolve forward targets and reject missing targets', () {
    final source = '''
col {
  button(get: "/ui/cart", target: "cart") { text(value: "Cart") }
  col(id: "cart")
}
''';
    expect(parseUi(source).children.last.props['id'], 'cart');
    final fragment = parseUi(
      'button(get: "/ui/cart", target: "cart") { text(value: "Cart") }',
      fragment: true,
    );
    expect(() => validateUi(fragment), throwsA(_diagnostic('does not exist')));
    expect(() => validateUi(fragment, fragment: true), returnsNormally);
  });

  test('fragment mode still checks targets in independent inline modals', () {
    expect(
      () => parseUi('''
col(id: "cart") {
  button(onTap: {
    popup(label: "Independent target") {
      button(get: "/ui/cart", target: "cart") { text(value: "Refresh") }
    }
  }) { text(value: "Open") }
}
''', fragment: true),
      throwsA(_diagnostic('does not exist in this surface')),
    );
  });

  test('revalidates a recomposed tree before applying partial updates', () {
    final document = parseUi('col { col(id: "cart") col(id: "other") }');
    final replacement = parseUi('col(id: "cart") { col(id: "other") }');
    expect(
      () => validateUi(
        document.withChildren([replacement, document.children[1]]),
      ),
      throwsA(_diagnostic('Duplicate ID "other"')),
    );
    expect(document.children.first.children, isEmpty);
  });

  test('normalizes whole numbers for integer controls on native and web', () {
    final document = parseUi('''
form(id: "cart", post: "/ui/cart", target: "cart") {
  input(name: "note", label: "Note", lines: 2.0, maxLength: 2e2)
  button(type: submit) { text(value: "Save") }
}
''');
    final props = document.children.first.props;
    expect(props['lines'], isA<int>().having((value) => value, 'value', 2));
    expect(
      props['maxLength'],
      isA<int>().having((value) => value, 'value', 200),
    );
  });

  test('state assignments may use names that are also event properties', () {
    expect(
      () => parseUi('''
col(state: {onTap: false}) {
  button(onTap: { set(onTap: !state.onTap) }) { text(value: "Toggle") }
}
'''),
      returnsNormally,
    );
  });

  final invalidDocuments = {
    '': 'Expected an element',
    'col {} row {}': 'exactly one root',
    'col { text(value: "unfinished")': 'Expected "}"',
    'text(value: "unfinished)': 'Unterminated string',
    r'text(value: "\q")': 'Invalid string escape',
    'text("Old syntax")': 'Expected a property name',
    'text()': 'requires property "value"',
    'text(value: "First", "Second")': 'Expected a property name',
    'col(gap: 1e309)': 'Number must be finite',
    'col(state: {quantity: 9007199254740993})': 'exceeds the supported range',
    'col(gap: 8, gap: 12)': 'Duplicate property "gap"',
    'col(state: {a: true, a: false})': 'Duplicate property "a"',
    'col { if(value: true) { text(value: "No") } }': 'Unsupported element "if"',
    'col { col(id: "cart") col(id: "cart") }': 'Duplicate ID',
    'col(background: purple)': 'Unknown property',
    'col { spacer() }': 'spacer must be a child of row',
    'text(value: "Cart", visible: state.unknown)': 'Unknown state key',
    'col(state: {a: 1}) { text(value: "Bad", visible: !state.a) }':
        'Only Boolean',
    'col(state: {a: true}) { text(value: state.a) }': 'expects a string',
    'col(state: {a: false}) { '
            'button(onTap: { set(a: "true") }) { text(value: "Set") } }':
        'Invalid value type',
    'button(get: "/ui/cart") { text(value: "Go") }':
        'exactly one of target or open',
    'button(get: "/ui/cart", target: "missing") { text(value: "Go") }':
        'does not exist',
    'button(get: "//elsewhere/ui", target: "cart") { text(value: "Go") }':
        'root-relative',
    'button(get: "/ui/../cart", target: "cart") { text(value: "Go") }':
        'root-relative',
    'button(get: "/ui/%2e%2e/cart", target: "cart") { text(value: "Go") }':
        'root-relative',
    r'button(get: "/ui/cart\n", target: "cart") { text(value: "Go") }':
        'root-relative',
    'button(type: submit) { text(value: "Go") }': 'must be inside a form',
    'button(onTap: {}) { text(value: "Go") }': 'exactly one action',
    'button(onTap: true) { text(value: "Go") }': 'expects an action block',
    'button(onTap: { dismiss() dismiss() }) { text(value: "Go") }':
        'exactly one action',
    'button(onTap: { dismiss() })': 'exactly one visual child',
    'button(onTap: { dismiss() }) { text(value: "A") text(value: "B") }':
        'exactly one visual child',
    'button(onTap: { dismiss() }) { onTap { dismiss() } }':
        'Unsupported element',
    'button(onTap: { dismiss() }, label: "Legacy") { text(value: "A") }':
        'Unknown property "label"',
    'button(onTap: { dismiss() }) { col { input(name: "note", label: "Note") } }':
        'only noninteractive visuals',
    'button(onTap: { dismiss() }) { col { '
            'button(onTap: { dismiss() }) { text(value: "Nested") } } }':
        'only noninteractive visuals',
    'button(onTap: { dismiss() }) { col(state: {a: true}) }':
        'only noninteractive visuals',
    'input(name: "note", label: "Note")': 'must be inside a form',
    'form(post: "/ui/items", target: "cart") {}': 'requires a submit',
    'form(post: "/ui/items", target: "cart") { '
            'input(name: "item", type: hidden, value: "shirt") '
            'input(name: "item", type: hidden, value: "shirt") '
            'button(type: submit) { text(value: "Submit") } }':
        'Duplicate form field',
    'form(post: "/ui/items", target: "cart") { '
            'select(name: "size", label: "Size", options: ["M"], initial: "L") '
            'button(type: submit) { text(value: "Submit") } }':
        'Initial selection must be one of the options',
  };

  for (final entry in invalidDocuments.entries) {
    test('rejects ${entry.key}', () {
      expect(() => parseUi(entry.key), throwsA(_diagnostic(entry.value)));
    });
  }

  test('bounds source size, value nesting, and global node count', () {
    expect(
      () => parseUi(' ' * (64 * 1024 + 1)),
      throwsA(_diagnostic('source exceeds')),
    );
    expect(
      () => parseUi('${'col {' * 33}${'}' * 33}'),
      throwsA(_diagnostic('UI nesting exceeds')),
    );
    expect(
      () => parseUi('col(state: {a: ${'[' * 33}0${']' * 33}})'),
      throwsA(_diagnostic('Value nesting exceeds')),
    );
    expect(
      () => parseUi('col { ${'text(value: "x") ' * 512} }'),
      throwsA(_diagnostic('more than 512 nodes')),
    );
    final modal =
        'popup(label: "Budget") { col { ${'text(value: "x") ' * 510} } }';
    expect(
      () => parseUi('button(onTap: { $modal }) { text(value: "Open") }'),
      throwsA(_diagnostic('more than 512 nodes')),
    );
  });
}

TypeMatcher<UiSyntaxException> _diagnostic(String message) =>
    isA<UiSyntaxException>().having(
      (error) => error.message,
      'message',
      contains(message),
    );
