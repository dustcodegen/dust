import 'dart:async';

import 'package:dio/dio.dart';
import 'package:dust_dart/ws.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_client_app/websocket_client.dart';
import 'package:server_client_app/websocket_contracts.dart';

void main() {
  const baseUrl = String.fromEnvironment(
    'CHAT_BASE_URL',
    defaultValue: 'http://127.0.0.1:8080',
  );
  final client = ChatClient(Dio(BaseOptions(baseUrl: baseUrl)));
  runApp(
    MaterialApp(
      home: Scaffold(body: GeneratedChatExample(client: client)),
    ),
  );
}

/// An isolated native example consuming a backend-derived typed client.
///
/// Uses the server fixture's explicit authorization header. Browser connectors
/// reject that header; compiling this entrypoint does not prove browser auth.
class GeneratedChatExample extends StatefulWidget {
  /// Creates one connection for this widget's lifetime.
  const GeneratedChatExample({required this.client, super.key});

  /// Client configured with the server's HTTP or WebSocket base URL.
  final ChatClient client;

  @override
  State<GeneratedChatExample> createState() => _GeneratedChatExampleState();
}

class _GeneratedChatExampleState extends State<GeneratedChatExample> {
  WebSocketConnection<ChatCommand, ChatNotice>? _socket;
  StreamSubscription<ChatNotice>? _messages;
  String _status = 'Connecting';
  String _text = '';

  @override
  void initState() {
    super.initState();
    // shortcut: client is fixed for this widget, use a new key to reconnect.
    unawaited(_connect());
  }

  Future<void> _connect() async {
    try {
      final socket = await widget.client.chat(
        'lobby',
        sequence: 1,
        token: 'Bearer fixture-token',
      );
      if (!mounted) {
        await socket.close();
        return;
      }
      _socket = socket;
      _messages = socket.messages.listen(
        (notice) {
          if (mounted) setState(() => _text = notice.text);
        },
        onError: (Object error, StackTrace stackTrace) {
          if (mounted) setState(() => _status = 'Message failed');
        },
        onDone: () {
          if (mounted) setState(() => _status = 'Closed');
        },
      );
      setState(() => _status = 'Connected');
    } on Object {
      if (mounted) setState(() => _status = 'Connection failed');
    }
  }

  Future<void> _send() async {
    final socket = _socket;
    if (socket == null) return;
    try {
      await socket.send(const ChatCommand(text: 'hello'));
    } on Object {
      if (mounted) setState(() => _status = 'Send failed');
    }
  }

  @override
  void dispose() {
    unawaited(_messages?.cancel());
    unawaited(_socket?.close());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(_status),
          if (_text.isNotEmpty) Text(_text),
          FilledButton(
            onPressed: _status == 'Connected' ? _send : null,
            child: const Text('Send hello'),
          ),
        ],
      ),
    );
  }
}
