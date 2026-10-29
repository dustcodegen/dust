@Routes('/health', tags: ['public'])
library;

import 'package:dust_server/annotations.dart';

part 'health.g.dart';

/// Returns the loopback fixture health response.
@GET('/')
Future<String> health() async => 'ok';
