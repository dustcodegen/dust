import 'dart:async';
import 'dart:convert';
import 'dart:developer';
import 'dart:io';

import 'package:flutter/widgets.dart';

/// Observes real OS events; never injects synthetic lifecycle notifications.
final class ProbeLifecycle extends WidgetsBindingObserver {
  /// Driver handshake state, independent of widget text/log formatting.
  String phase = 'starting';

  /// Completes after the OS backgrounds this process.
  final paused = Completer<void>();

  /// Completes only after a pause followed by an OS resume.
  final resumed = Completer<void>();

  /// Retains all observed transitions in the evidence report.
  final states = <String>[];

  /// Exposes test-only status, never a native handle or database operation.
  void registerStatus() => registerExtension(
        'ext.dust.sqliteProbe',
        (method, parameters) async => ServiceExtensionResponse.result(
          jsonEncode({
            'phase': phase,
            'pid': pid,
            'platform': Platform.operatingSystem,
            'states': states,
          }),
        ),
      );

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    states.add(state.name);
    if (state == AppLifecycleState.paused && !paused.isCompleted) {
      phase = 'paused';
      paused.complete();
    }
    if (state == AppLifecycleState.resumed &&
        paused.isCompleted &&
        !resumed.isCompleted) {
      phase = 'resumed';
      resumed.complete();
    }
  }
}
