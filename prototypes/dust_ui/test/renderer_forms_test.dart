import 'dart:async';

import 'package:dust_ui_prototype/language.dart';
import 'package:dust_ui_prototype/renderer.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

Widget _host(UiNode document, Future<UiNode> Function(UiRequest) request) =>
    MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(
          child: DustUiView(document: document, request: request),
        ),
      ),
    );

UiNode _status() =>
    parseUi('col(id: "status") { text(value: "Saved") }', fragment: true);

void main() {
  testWidgets('hidden ancestors skip fields and preserve their drafts', (
    tester,
  ) async {
    final document = parseUi('''
col(state: {details: false}) {
  button(onTap: { set(details: !state.details) }) { text(value: "Details") }
  form(post: "/ui/save", target: "status") {
    col(gap: 8) {
      input(name: "token", type: hidden, value: "always")
      col(visible: state.details) {
        row {
          input(name: "note", label: "Note", required: true)
          select(name: "wrap", label: "Wrap", options: ["Blue", "Red"], required: true)
        }
        input(name: "branch", type: hidden, value: "active")
      }
      button(type: submit) { text(value: "Save") }
    }
  }
  col(id: "status") { text(value: "Ready") }
}
''');
    final requests = <UiRequest>[];
    await tester.pumpWidget(
      _host(document, (request) async {
        requests.add(request);
        return _status();
      }),
    );

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(find.text('Required'), findsNothing);
    expect(requests.single.body, {'token': 'always'});

    await tester.tap(find.text('Details'));
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(find.text('Required'), findsNWidgets(2));
    expect(requests, hasLength(1));
    await tester.enterText(find.byType(TextFormField), 'Happy birthday');
    await tester.tap(find.byType(DropdownButtonFormField<String>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Red').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Details'));
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(requests.last.body, {'token': 'always'});
    expect(find.text('Required'), findsNothing);

    await tester.tap(find.text('Details'));
    await tester.pump();
    expect(find.text('Happy birthday'), findsOneWidget);
    expect(find.text('Red'), findsOneWidget);
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(requests.last.body, {
      'token': 'always',
      'note': 'Happy birthday',
      'wrap': 'Red',
      'branch': 'active',
    });
  });

  testWidgets('direct POST sends an empty body while submit captures fields', (
    tester,
  ) async {
    final document = parseUi('''
col {
  form(post: "/ui/save", target: "status") {
    input(name: "name", label: "Name", required: true)
    button(post: "/ui/ping", target: "status") {
      row { text(value: "Direct") text(value: "POST") }
    }
    button(type: submit) { text(value: "Submit form") }
  }
  col(id: "status") { text(value: "Ready") }
}
''');
    final requests = <UiRequest>[];
    await tester.pumpWidget(
      _host(document, (request) async {
        requests.add(request);
        return _status();
      }),
    );
    await tester.tap(find.text('Direct'));
    await tester.pumpAndSettle();
    expect(requests.single.method, 'POST');
    expect(requests.single.path, '/ui/ping');
    expect(requests.single.body, isEmpty);
    expect(find.text('Required'), findsNothing);
    await tester.tap(find.text('Submit form'));
    await tester.pumpAndSettle();
    expect(requests, hasLength(1));
    expect(find.text('Required'), findsOneWidget);
    await tester.enterText(find.byType(TextFormField), 'Maya');
    await tester.tap(find.text('Submit form'));
    await tester.pumpAndSettle();
    expect(requests.last.path, '/ui/save');
    expect(requests.last.body, {'name': 'Maya'});
  });

  testWidgets('failed POST preserves drafts and serializes target requests', (
    tester,
  ) async {
    final document = parseUi('''
col(gap: 12, state: {gift: false}) {
  button(onTap: { set(gift: !state.gift) }) { text(value: "Gift") }
  form(id: "order", post: "/ui/save", target: "status") {
    col(gap: 8) {
      input(name: "gift", type: hidden, value: state.gift)
      select(name: "size", label: "Size", options: ["S", "M", "L"], initial: "M", required: true)
      input(name: "note", label: "Note")
      button(type: submit) { text(value: "Add to cart") }
    }
  }
  col(id: "status") { text(value: "Empty cart") }
  button(get: "/ui/cart/summary", target: "status") { text(value: "Refresh") }
}
''');
    final requests = <UiRequest>[];
    final responses = <Completer<UiNode>>[];
    await tester.pumpWidget(
      _host(document, (request) {
        requests.add(request);
        final response = Completer<UiNode>();
        responses.add(response);
        return response.future;
      }),
    );
    await tester.tap(find.text('Gift'));
    await tester.pump();
    await tester.enterText(find.byType(TextFormField), 'A handwritten note');
    await tester.tap(find.text('Add to cart'));
    await tester.pump();
    expect(requests.single.body, {
      'gift': true,
      'size': 'M',
      'note': 'A handwritten note',
    });
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Refresh'))
          .onPressed,
      isNull,
    );
    responses.single.completeError(StateError('Offline'));
    await tester.pumpAndSettle();
    expect(find.text('Empty cart'), findsOneWidget);
    expect(find.text('Could not complete the request.'), findsOneWidget);
    expect(find.text('A handwritten note'), findsOneWidget);
    await tester.tap(find.text('Add to cart'));
    await tester.pump();
    responses.last.complete(_status());
    await tester.pumpAndSettle();
    expect(find.text('Saved'), findsOneWidget);
    expect(find.text('A handwritten note'), findsOneWidget);
  });
}
