import 'package:flutter/material.dart';

/// A continuously animated UI, independent of native database work.
final class ProbeUi extends StatefulWidget {
  /// Reports each built animation frame to the test.
  const ProbeUi({required this.onFrame, super.key});

  /// Counts caller-side UI progress.
  final VoidCallback onFrame;

  @override
  State<ProbeUi> createState() => _ProbeUiState();
}

final class _ProbeUiState extends State<ProbeUi>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulse = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 1),
  )..repeat();

  @override
  void dispose() {
    _pulse.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp(
        home: Scaffold(
          body: AnimatedBuilder(
            animation: _pulse,
            builder: (context, child) {
              widget.onFrame();
              return Center(
                child: Text(
                  'dust SQLite probe ${_pulse.value.toStringAsFixed(2)}',
                  key: const ValueKey('sqlite-probe'),
                ),
              );
            },
          ),
        ),
      );
}
