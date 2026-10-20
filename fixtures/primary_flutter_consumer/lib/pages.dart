import 'package:flutter/material.dart';

import 'route.dart';

/// Initial route for the primary-constructor example.
@AppRoute('/', name: 'home')
class const HomePage({super.key}) extends StatelessWidget {
  /// Creates a home page.
  this;

  @override
  Widget build(BuildContext context) => const Text('Home');
}

/// Route with declaring path/query fields and a non-declaring super parameter.
@AppRoute('/counter/:id', name: 'counter')
class const CounterPage({
  /// Path identifier.
  required final int id,

  /// Optional query tab.
  final String? tab,
  super.key,
}) extends StatelessWidget {
  /// Creates a counter page.
  this;

  @override
  Widget build(BuildContext context) => Text('$id:${tab ?? 'overview'}');
}

/// Fallback for unmatched routes.
@AppRoute('/404', name: 'notFound')
class const MissingPage({super.key}) extends StatelessWidget {
  /// Creates the fallback page.
  this;

  @override
  Widget build(BuildContext context) => const Text('Not found');
}
