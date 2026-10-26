import 'dart:typed_data';

import 'package:dust_dart/serde.dart';
import 'package:shelf/shelf.dart';

import 'encoders.dart';
import 'into_response.dart';
import 'rejection.dart';

/// Turns whatever a typed handler returned into a response.
///
/// The rule is the one axum uses for `IntoResponse`: the handler says what it
/// produced, and the runtime decides how it reaches the wire. That is what
/// lets a handler end in `repository.find(id)` rather than in a call to an
/// encoder.
///
/// | Returned | Answer |
/// | :--- | :--- |
/// | `Response` | itself, untouched |
/// | `IntoResponse` | `intoResponse()`, which covers `Rejection` |
/// | `Ok(value)` | the value, dispatched again |
/// | `Err(error)` | the error, dispatched again, defaulting to 500 |
/// | `Some(value)` | the value, dispatched again |
/// | `None` | 404 |
/// | `null` | 204 |
/// | `Unit` | 204 |
/// | `String` | `text/plain`, sent as written |
/// | `Uint8List` or `List<int>` | `application/octet-stream` |
/// | `Stream<List<int>>` | streamed as it arrives, unbuffered |
/// | anything else | JSON, through `serialize` when it derives `Serialize` |
///
/// A bare [String] is text, not JSON — axum's rule. To send a JSON string
/// specifically, wrap it in a list or a map, or call `jsonResponse`.
///
/// [status] applies to successful text, bytes, streams, JSON, `Ok`, and `Some`.
/// Values that already own response semantics (`Response`, `IntoResponse`,
/// `Err`, `None`, `null`, and `Unit`) keep their own status.
Response responseFrom(Object? value, {int status = 200}) {
  return switch (value) {
    null => noContent(),
    final Response response => response,
    final IntoResponse into => into.intoResponse(),
    Ok(value: final inner) => responseFrom(inner, status: status),
    Err(error: final inner) => _errorResponse(inner),
    Some(value: final inner) => responseFrom(inner, status: status),
    None() => const Rejection.notFound('not found').intoResponse(),
    Unit() => noContent(),
    final String text => textResponse(text, status: status),
    final Uint8List bytes => bytesResponse(bytes, status: status),
    final List<int> bytes =>
      bytesResponse(Uint8List.fromList(bytes), status: status),
    final Stream<List<int>> body => streamed(body, status: status),
    _ => jsonResponse(value, status: status),
  };
}

/// Answers for the error side of a [Result].
///
/// A handler that returns `Err` has said the request failed, so an error with
/// no opinion about status becomes a 500 rather than a 200 carrying an error
/// object. Anything implementing [IntoResponse] keeps its own status.
Response _errorResponse(Object? error) {
  return switch (error) {
    null => const Rejection.internal().intoResponse(),
    final Response response => response,
    final IntoResponse into => into.intoResponse(),
    final Serializable body => jsonResponse(body, status: 500),
    _ => Rejection.internal(error.toString()).intoResponse(),
  };
}
