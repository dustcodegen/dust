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

void main() {
  testWidgets('nested button content supports state and both popup paths', (
    tester,
  ) async {
    final document = parseUi('''
col(gap: 8, state: {expanded: false, caption: "Details"}) {
  button(expanded: state.expanded, onTap: { set(expanded: !state.expanded) }) {
    row {
      text(value: state.caption)
      spacer()
      text(value: "Toggle")
    }
  }
  text(value: "The shirt is pure linen.", visible: state.expanded)
  button(onTap: {
    popup(label: "Size guide") {
      col {
        text(value: "Size M")
        button(onTap: { dismiss() }) { text(value: "Done") }
      }
    }
  }) { text(value: "Size guide") }
  button(get: "/ui/cart", open: popup, accessibilityLabel: "Open shopping cart") {
    row { text(value: "Cart") text(value: "0") }
  }
}
''');
    final requests = <UiRequest>[];
    final response = Completer<UiNode>();
    await tester.pumpWidget(
      _host(document, (request) {
        requests.add(request);
        return response.future;
      }),
    );

    expect(find.text('The shirt is pure linen.'), findsNothing);
    await tester.tap(find.text('Details'));
    await tester.pump();
    expect(find.text('The shirt is pure linen.'), findsOneWidget);
    expect(requests, isEmpty);

    await tester.tap(find.text('Size guide'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text('Size M'), findsOneWidget);
    await tester.tap(find.text('Done'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);

    await tester.tap(find.text('Cart'));
    await tester.pump();
    expect(find.text('Working…'), findsOneWidget);
    expect(requests.single.method, 'GET');
    expect(requests.single.path, '/ui/cart');
    expect(requests.single.body, isEmpty);
    response.complete(parseUi('text(value: "Cart is empty")', fragment: true));
    await tester.pumpAndSettle();
    expect(find.text('Cart is empty'), findsOneWidget);
    expect(
      tester.widget<AlertDialog>(find.byType(AlertDialog)).semanticLabel,
      'Open shopping cart',
    );
    await tester.tap(find.byTooltip('Close dialog'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.text('The shirt is pure linen.'), findsOneWidget);
    expect(find.text('Cart'), findsOneWidget);
  });

  testWidgets('popup targets stay isolated from the parent surface', (
    tester,
  ) async {
    final document = parseUi('''
col {
  col(id: "summary") { text(value: "Parent summary") }
  button(onTap: {
    popup(label: "Summary") {
      col {
        col(id: "summary") { text(value: "Modal summary") }
        button(get: "/ui/summary", target: "summary") {
          text(value: "Refresh modal")
        }
      }
    }
  }) { text(value: "Open summary") }
}
''');
    await tester.pumpWidget(
      _host(
        document,
        (_) async => parseUi(
          'col(id: "summary") { text(value: "Updated modal") }',
          fragment: true,
        ),
      ),
    );
    await tester.tap(find.text('Open summary'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Refresh modal'));
    await tester.pumpAndSettle();
    expect(find.text('Updated modal'), findsOneWidget);
    expect(find.text('Parent summary'), findsOneWidget);
    await tester.tap(find.byTooltip('Close dialog'));
    await tester.pumpAndSettle();
    expect(find.text('Parent summary'), findsOneWidget);
    expect(find.text('Updated modal'), findsNothing);
  });

  testWidgets('fetched popups must be complete surfaces', (tester) async {
    final document = parseUi('''
button(get: "/ui/cart", open: popup) { text(value: "Open cart") }
''');
    await tester.pumpWidget(
      _host(
        document,
        (_) async => parseUi('''
button(get: "/ui/missing", target: "missing") { text(value: "Missing target") }
''', fragment: true),
      ),
    );
    await tester.tap(find.text('Open cart'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.text('Could not complete the request.'), findsOneWidget);
    expect(find.text('Open cart'), findsOneWidget);
  });

  testWidgets('stale responses cannot update a newly applied document', (
    tester,
  ) async {
    final document = parseUi('''
col {
  col(id: "cart") { text(value: "Old cart") }
  button(get: "/ui/cart/summary", target: "cart") { text(value: "Refresh") }
}
''');
    final replacement = parseUi(
      'col(id: "cart") { text(value: "New document") }',
    );
    final response = Completer<UiNode>();
    Future<UiNode> request(UiRequest _) => response.future;
    await tester.pumpWidget(_host(document, request));
    await tester.tap(find.text('Refresh'));
    await tester.pump();
    await tester.pumpWidget(_host(replacement, request));
    response.complete(
      parseUi(
        'col(id: "cart") { text(value: "Stale response") }',
        fragment: true,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('New document'), findsOneWidget);
    expect(find.text('Stale response'), findsNothing);
  });
}
