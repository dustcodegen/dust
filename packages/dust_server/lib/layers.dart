/// Middleware that ships in the box: capacity, deadlines, ids, access records.
///
/// Nothing is installed by default. An application adds what it wants:
///
/// ```dart
/// import 'package:dust_server/layers.dart';
///
/// final app = Router()
///   ..layer(const ConcurrencyLimit(maxConcurrentRequests: 64))
///   ..layer(const RequestTimeout(Duration(seconds: 30)))
///   ..layer(const RequestId())
///   ..layer(AccessLog(logger.info));
/// ```
///
/// Use `productionRouter` when the same explicit safety policy should wrap
/// handwritten and generated modules.
///
/// Everything here is also exported from `package:dust_server/server.dart`.
library;

export 'src/layers/layers.dart';
