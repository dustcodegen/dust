import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_client_app/query_contracts.dart';
import 'package:server_client_server/client_edges.dart';

const _iterations = 20000;
const _warmupIterations = 2000;
final List<String> _payloads = [
  jsonEncode({
    'display_name': 'Ship Dust',
    'order': 'newest',
    'metadata': {'source': 'benchmark'},
    'tags': ['dart', 'rust'],
  }),
  jsonEncode({
    'display_name': 'Profile Dust',
    'order': 'oldest',
    'metadata': {'source': 'benchmark'},
    'tags': ['server'],
  }),
];

final CreateTransport Function(Map<String, Object?>) _deserializer =
    const $CreateTransportDeserializer().deserialize;
final _extractor = ValidatedExtractable<CreateTransport>(
  JsonExtractable<CreateTransport>(_deserializer),
);

Future<void> main() async {
  final controller = Transport();
  final generatedRoute = controller.routes().handler;
  final routeOnly = Router.module(
    routes: [Route('POST', '/transport/typed', (_) async => Response.ok('ok'))],
  ).handler;

  await _measure('handler_only', (_) async => Response.ok('ok'));
  await _measure('route_only', routeOnly);
  await _measure(
    'generated_handler_without_router',
    (request) => _generatedHandler(controller, request),
  );
  await _measure('generated_route', generatedRoute);
  await _measure(
    'fused_shelf_floor',
    (request) => _fusedHandler(controller, request),
  );
}

Future<Response> _generatedHandler(
  Transport controller,
  Request request,
) async {
  final extracted = await _extractor.extract(request);
  if (extracted case Err(:final error)) return error.intoResponse();
  final input = (extracted as Ok<CreateTransport, Rejection>).value;

  return guard(
    () async => responseFrom(await controller.createTyped(input), status: 201),
  );
}

Future<Response> _fusedHandler(Transport controller, Request request) async {
  final json = jsonDecode(await request.readAsString());
  final input = _deserializer(json! as Map<String, Object?>);
  if (!input.validate().isValid) {
    return const Rejection.unprocessable(
      {},
      message: 'Validation failed',
    ).intoResponse();
  }
  return responseFrom(await controller.createTyped(input), status: 201);
}

Future<void> _measure(
  String name,
  FutureOr<Response> Function(Request request) handler,
) async {
  var checksum = 0;
  for (var index = 0; index < _warmupIterations; index++) {
    checksum += (await handler(_request(index))).statusCode;
  }

  final stopwatch = Stopwatch()..start();
  for (var index = 0; index < _iterations; index++) {
    checksum += (await handler(_request(index))).statusCode;
  }
  stopwatch.stop();

  final micros = stopwatch.elapsedMicroseconds / _iterations;
  stdout.writeln('$name ${micros.toStringAsFixed(3)} us/op checksum=$checksum');
}

Request _request(int index) {
  return Request(
    'POST',
    Uri.parse('http://localhost/transport/typed'),
    headers: const {'content-type': 'application/json'},
    body: _payloads[index & 1],
  );
}
