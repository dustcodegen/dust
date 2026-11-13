import 'dart:io';

import 'package:dio/dio.dart';
import 'package:server_client_app/websocket_client.dart';
import 'package:server_client_app/websocket_contracts.dart';

/// Sends one typed command and closes the generated client's connection.
///
/// Run `dart run example/websocket_client.dart [base-url]` from this package
/// after starting `dart run bin/websocket_example.dart` in another terminal.
Future<void> main(List<String> args) async {
  final baseUrl = args.isEmpty ? 'http://127.0.0.1:8080' : args.single;
  final dio = Dio(BaseOptions(baseUrl: baseUrl));
  try {
    final socket = await ChatClient(dio).chat(
      'lobby',
      sequence: 1,
      token: 'Bearer fixture-token', // Local fixture credential only.
    );
    try {
      final reply = socket.messages.first;
      await socket.send(const ChatCommand(text: 'hello'));
      final notice = await reply;
      stdout.writeln('${notice.user} in ${notice.room}: ${notice.text}');
    } finally {
      await socket.close();
    }
  } finally {
    dio.close();
  }
}
