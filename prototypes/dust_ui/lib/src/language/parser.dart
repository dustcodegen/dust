import 'lexer.dart';
import 'model.dart';
import 'validator.dart';

/// Parses and validates one UI document or region fragment.
UiNode parseUi(String source, {bool fragment = false}) {
  if (source.length > 64 * 1024) {
    throw const UiSyntaxException('UI source exceeds 64 Ki code units.', 1, 1);
  }
  final root = _Parser(tokenize(source)).parse();
  validateUi(root, fragment: fragment);
  return root;
}

final class _Parser {
  _Parser(this.tokens);

  final List<Token> tokens;
  int index = 0;
  int nodes = 0;

  Token get current => tokens[index];

  UiNode parse() {
    final root = _node(0);
    if (current.kind != TokenKind.eof) {
      _fail('Expected exactly one root element.');
    }
    return root;
  }

  UiNode _node(int depth) {
    if (depth >= maxUiDepth) _fail('UI nesting exceeds $maxUiDepth levels.');
    if (++nodes > maxUiNodes) _fail('UI has more than $maxUiNodes nodes.');
    final name = _takeIdentifier('Expected an element or action name.');
    final props = <String, Object?>{};
    final propertyLocations = <String, ({int line, int column})>{};
    if (_take('(') && !_take(')')) {
      do {
        final key = _namedValue(props, 0, actionDepth: depth + 1);
        propertyLocations[key.text] = (line: key.line, column: key.column);
      } while (_take(',') && !_is(')'));
      _expect(')');
    }

    final children = <UiNode>[];
    if (_take('{')) {
      while (!_take('}')) {
        if (current.kind == TokenKind.eof) _fail('Expected "}".');
        children.add(_node(depth + 1));
      }
    }
    return UiNode(
      name: name.text,
      props: props,
      propertyLocations: propertyLocations,
      children: children,
      line: name.line,
      column: name.column,
    );
  }

  Token _namedValue(
    Map<String, Object?> values,
    int depth, {
    int? actionDepth,
  }) {
    final key = _takeIdentifier('Expected a property name.');
    _expect(':');
    if (values.containsKey(key.text)) {
      throw UiSyntaxException(
        'Duplicate property "${key.text}".',
        key.line,
        key.column,
      );
    }
    values[key.text] = key.text == 'onTap' && actionDepth != null && _is('{')
        ? _actionBlock(actionDepth)
        : _value(depth);
    return key;
  }

  UiActionBlock _actionBlock(int depth) {
    if (depth >= maxUiDepth) _fail('UI nesting exceeds $maxUiDepth levels.');
    _expect('{');
    final actions = <UiNode>[];
    while (!_take('}')) {
      if (current.kind == TokenKind.eof) _fail('Expected "}".');
      actions.add(_node(depth));
    }
    return UiActionBlock(actions);
  }

  Object? _value(int depth) {
    if (depth >= maxUiDepth) _fail('Value nesting exceeds $maxUiDepth levels.');
    if (_take('{')) {
      final values = <String, Object?>{};
      if (!_take('}')) {
        do {
          _namedValue(values, depth + 1);
        } while (_take(',') && !_is('}'));
        _expect('}');
      }
      return Map<String, Object?>.unmodifiable(values);
    }
    if (_take('[')) {
      final values = <Object?>[];
      if (!_take(']')) {
        do {
          values.add(_value(depth + 1));
        } while (_take(',') && !_is(']'));
        _expect(']');
      }
      return List<Object?>.unmodifiable(values);
    }
    if (current.kind == TokenKind.literal) return tokens[index++].value;
    final negated = _take('!');
    final name = _takeIdentifier('Expected a property value.').text;
    if (name == 'state' && _take('.')) {
      return UiStateRef(
        _takeIdentifier('Expected a state key after "state.".').text,
        negated: negated,
      );
    }
    if (negated) _fail('Expected a state reference after "!".');
    return switch (name) {
      'true' => true,
      'false' => false,
      'null' => null,
      _ => UiSymbol(name),
    };
  }

  Token _takeIdentifier(String message) {
    if (current.kind != TokenKind.identifier) _fail(message);
    return tokens[index++];
  }

  bool _is(String text) =>
      current.kind == TokenKind.punctuation && current.text == text;

  bool _take(String text) {
    if (!_is(text)) return false;
    index++;
    return true;
  }

  void _expect(String text) {
    if (!_take(text)) _fail('Expected "$text".');
  }

  Never _fail(String message) =>
      throw UiSyntaxException(message, current.line, current.column);
}
