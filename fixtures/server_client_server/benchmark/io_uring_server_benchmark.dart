import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:dust_server/server.dart';
import 'package:server_client_server/client_edges.dart';

import 'io_uring_probe_bindings.dart';

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) {
    stderr.writeln('usage: io_uring_server_benchmark.dart path/to/server.so');
    exitCode = 64;
    return;
  }

  final bindings = IoUringProbeBindings(DynamicLibrary.open(arguments.single));
  final handler = Transport().routes().handler;
  late Pointer<Void> server;
  late NativeCallable<IoUringRequestNative> callback;
  callback = NativeCallable<IoUringRequestNative>.listener(
    (int id, Pointer<Uint8> pointer, int length) {
      final requestBytes = Uint8List.fromList(pointer.asTypedList(length));
      unawaited(_handle(bindings, server, handler, id, requestBytes));
    },
  );

  server = bindings.start(callback.nativeFunction, 8080);
  if (server == nullptr) {
    callback.close();
    stderr.writeln('failed to start io_uring server');
    exitCode = 1;
    return;
  }
  stdout.writeln('ready 127.0.0.1:${bindings.port(server)}');

  await ProcessSignal.sigterm.watch().first;
  bindings.free(server);
  callback.close();
}

Future<void> _handle(
  IoUringProbeBindings bindings,
  Pointer<Void> server,
  Handler handler,
  int id,
  Uint8List raw,
) async {
  try {
    final request = _decodeRequest(raw);
    final response = await handler(request);
    final body = await _readBody(response);
    _respond(bindings, server, id, response.statusCode, body);
  } on Object catch (error, stackTrace) {
    stderr
      ..writeln('io_uring Dart handler failed: $error')
      ..writeln(stackTrace);
    _respond(
      bindings,
      server,
      id,
      500,
      utf8.encode('{"error":"internal server error"}'),
    );
  }
}

Request _decodeRequest(Uint8List raw) {
  final headerEnd = _headerEnd(raw);
  if (headerEnd < 0) throw const FormatException('missing HTTP headers');
  final lines = ascii.decode(raw.sublist(0, headerEnd)).split('\r\n');
  final requestLine = lines.first.split(' ');
  if (requestLine.length != 3) {
    throw const FormatException('invalid HTTP request line');
  }
  final headers = <String, String>{};
  for (final line in lines.skip(1)) {
    final separator = line.indexOf(':');
    if (separator < 1) throw const FormatException('invalid HTTP header');
    headers[line.substring(0, separator).trim().toLowerCase()] =
        line.substring(separator + 1).trim();
  }
  return Request(
    requestLine[0],
    Uri.parse('http://127.0.0.1${requestLine[1]}'),
    headers: headers,
    body: raw.sublist(headerEnd + 4),
  );
}

int _headerEnd(Uint8List bytes) {
  for (var index = 0; index <= bytes.length - 4; index++) {
    if (bytes[index] == 13 &&
        bytes[index + 1] == 10 &&
        bytes[index + 2] == 13 &&
        bytes[index + 3] == 10) {
      return index;
    }
  }
  return -1;
}

Future<Uint8List> _readBody(Response response) async {
  final builder = BytesBuilder(copy: false);
  await response.read().forEach(builder.add);
  return builder.takeBytes();
}

void _respond(
  IoUringProbeBindings bindings,
  Pointer<Void> server,
  int id,
  int status,
  List<int> body,
) {
  final pointer = bindings.allocate(body.length);
  if (body.isNotEmpty) pointer.asTypedList(body.length).setAll(0, body);
  if (!bindings.respond(server, id, status, pointer, body.length)) {
    stderr.writeln('failed to deliver response for request $id');
  }
}
