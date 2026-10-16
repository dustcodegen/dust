import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:primary_flutter_consumer/counter.dart';
import 'package:primary_flutter_consumer/pages.dart';
import 'package:primary_flutter_consumer/route.dart';

void main() {
  test('primary state defaults and generated copy remain typed', () {
    const state = CounterState();
    expect(state.count, 0);
    expect(state.copyWith(count: 7), const CounterState(count: 7));
  });

  test('generated routes decode primary path and query fields', () {
    final route = parsePrimaryRoute(Uri.parse('/counter/7?tab=details'));
    expect(route, isA<CounterRoute>());
    final counter = route as CounterRoute;
    expect(counter.id, 7);
    expect(counter.tab, 'details');
    expect(counter.location, '/counter/7?tab=details');
  });

  testWidgets('primary widget preserves fields and forwards its key', (
    tester,
  ) async {
    const key = ValueKey<String>('counter');
    await tester.pumpWidget(
      const MaterialApp(
        home: CounterPage(key: key, id: 7, tab: 'details'),
      ),
    );
    expect(find.byKey(key), findsOneWidget);
    expect(find.text('7:details'), findsOneWidget);
  });

  testWidgets('generated scope watches a primary ViewModel state change', (
    tester,
  ) async {
    late Counter counter;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CounterScope(
          args: (_) => const CounterArgs(),
          create: (_, args) => counter = Counter(args),
          child: Builder(
            builder: (context) => Text('${context.watchCounter().value.count}'),
          ),
        ),
      ),
    );
    expect(find.text('0'), findsOneWidget);
    counter.increment();
    await tester.pump();
    expect(find.text('1'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
