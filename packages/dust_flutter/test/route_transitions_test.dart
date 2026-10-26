import 'package:dust_flutter/route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('generated transition route forwards page configuration', (
    tester,
  ) async {
    final transition = _TransitionBuilder();
    late PageRoute<void> route;
    late Widget builtPage;
    late Widget builtTransition;

    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            final page = generatedPage<void>(
              key: const ValueKey('detail'),
              location: '/detail',
              name: 'detail',
              child: const Text('detail'),
              transition: transition,
              fullscreenDialog: true,
              maintainState: false,
            );
            route = page.createRoute(context) as PageRoute<void>;
            builtPage = route.buildPage(
              context,
              kAlwaysCompleteAnimation,
              kAlwaysDismissedAnimation,
            );
            builtTransition = route.buildTransitions(
              context,
              kAlwaysCompleteAnimation,
              kAlwaysDismissedAnimation,
              const Text('transition child'),
            );
            return const SizedBox();
          },
        ),
      ),
    );

    expect(route.settings.name, 'detail');
    expect(route.fullscreenDialog, isTrue);
    expect(route.maintainState, isFalse);
    expect(route.transitionDuration, const Duration(milliseconds: 120));
    expect(route.reverseTransitionDuration, const Duration(milliseconds: 80));
    expect(route.barrierColor, isNull);
    expect(route.barrierLabel, isNull);
    expect(route.delegatedTransition, isNull);
    expect(builtPage, isA<Semantics>());
    expect(builtTransition, isA<FadeTransition>());
  });

  testWidgets('bottom-to-top transition respects reduced motion', (
    tester,
  ) async {
    final child = Container();
    late Widget transition;

    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: Builder(
            builder: (context) {
              transition = BottomToTopPageTransitionsBuilder().buildTransitions(
                MaterialPageRoute<void>(builder: (_) => child),
                context,
                kAlwaysCompleteAnimation,
                kAlwaysDismissedAnimation,
                child,
              );
              return const SizedBox();
            },
          ),
        ),
      ),
    );

    expect(transition, same(child));
  });

  testWidgets('bottom-to-top transition slides when motion is enabled', (
    tester,
  ) async {
    late Widget transition;

    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            transition = BottomToTopPageTransitionsBuilder().buildTransitions(
              MaterialPageRoute<void>(builder: (_) => const SizedBox()),
              context,
              kAlwaysDismissedAnimation,
              kAlwaysDismissedAnimation,
              const Text('child'),
            );
            return const SizedBox();
          },
        ),
      ),
    );

    expect(transition, isA<SlideTransition>());
  });

  testWidgets('no-transition builder returns its child', (tester) async {
    final child = Container();
    late Widget transition;

    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            transition = const GeneratedNoTransitionBuilder().buildTransitions(
              MaterialPageRoute<void>(builder: (_) => child),
              context,
              kAlwaysCompleteAnimation,
              kAlwaysDismissedAnimation,
              child,
            );
            return const SizedBox();
          },
        ),
      ),
    );

    expect(transition, same(child));
  });
}

final class _TransitionBuilder extends PageTransitionsBuilder {
  @override
  Duration get transitionDuration => const Duration(milliseconds: 120);

  @override
  Duration get reverseTransitionDuration => const Duration(milliseconds: 80);

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    return FadeTransition(opacity: animation, child: child);
  }
}
