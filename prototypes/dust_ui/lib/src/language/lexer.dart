import 'dart:convert';

import 'model.dart';

/// The token categories used by the document parser.
enum TokenKind { identifier, literal, punctuation, eof }

/// One token with its source coordinates.
final class Token {
  const Token(this.kind, this.text, this.line, this.column, [this.value]);

  final TokenKind kind;
  final String text;
  final int line;
  final int column;
  final Object? value;
}

/// Splits source into identifiers, JSON scalar literals, and punctuation.
List<Token> tokenize(String source) => _Lexer(source).scan();

final _identifier = RegExp(r'[A-Za-z_][A-Za-z_0-9]*');
final _number = RegExp(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?');
const _punctuation = '{}()[]:,.!';
const _largestExactInteger = 9007199254740991;

final class _Lexer {
  _Lexer(this.source);

  final String source;
  int offset = 0;
  int line = 1;
  int lineStart = 0;
  int get column => offset - lineStart + 1;

  List<Token> scan() {
    final tokens = <Token>[];
    while (offset < source.length) {
      final character = source[offset];
      if (' \t\r\n'.contains(character)) {
        offset++;
        if (character == '\r' || character == '\n') {
          if (character == '\r' &&
              offset < source.length &&
              source[offset] == '\n') {
            offset++;
          }
          line++;
          lineStart = offset;
        }
        continue;
      }
      if (source.startsWith('//', offset)) {
        while (offset < source.length &&
            source[offset] != '\n' &&
            source[offset] != '\r') {
          offset++;
        }
        continue;
      }
      final startColumn = column;
      if (_punctuation.contains(character)) {
        offset++;
        tokens.add(Token(TokenKind.punctuation, character, line, startColumn));
        continue;
      }
      if (character == '"') {
        tokens.add(_string());
        continue;
      }
      final identifier = _identifier.matchAsPrefix(source, offset);
      if (identifier != null) {
        offset = identifier.end;
        tokens.add(
          Token(TokenKind.identifier, identifier[0]!, line, startColumn),
        );
        continue;
      }
      final number = _number.matchAsPrefix(source, offset);
      if (number != null) {
        offset = number.end;
        final text = number[0]!;
        final value = num.tryParse(text);
        if (value == null || !value.isFinite) {
          throw UiSyntaxException('Number must be finite.', line, startColumn);
        }
        if (value.abs() > _largestExactInteger) {
          throw UiSyntaxException(
            'Number exceeds the supported range.',
            line,
            startColumn,
          );
        }
        final normalized = value % 1 == 0 ? value.toInt() : value;
        tokens.add(
          Token(TokenKind.literal, text, line, startColumn, normalized),
        );
        continue;
      }
      throw UiSyntaxException(
        'Unexpected character "$character".',
        line,
        column,
      );
    }
    tokens.add(Token(TokenKind.eof, '', line, column));
    return tokens;
  }

  Token _string() {
    final start = offset++;
    final startColumn = start - lineStart + 1;
    while (offset < source.length) {
      final character = source[offset++];
      if (character == '"') {
        final text = source.substring(start, offset);
        try {
          return Token(
            TokenKind.literal,
            text,
            line,
            startColumn,
            jsonDecode(text),
          );
        } on FormatException {
          throw UiSyntaxException('Invalid string escape.', line, startColumn);
        }
      }
      if (character.codeUnitAt(0) < 32) break;
      if (character == '\\' && offset < source.length) {
        if (source.codeUnitAt(offset) < 32) break;
        offset++;
      }
    }
    throw UiSyntaxException('Unterminated string.', line, startColumn);
  }
}
