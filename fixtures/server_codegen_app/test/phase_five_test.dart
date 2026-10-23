import 'package:dust_server/server.dart';
import 'package:server_codegen_app/server_codegen_app.dart';
import 'package:test/test.dart';

void main() {
  test('describe recovers generated route metadata after mounting', () {
    final route = buildApp()
        .describe()
        .singleWhere((route) => route.path == '/api/v1/todos/layers');
    final metadata = route.metadataOf<RouteMetadata>();

    expect(metadata?.operationId, 'traceLayers');
    expect(metadata?.summary, 'Show middleware order');
    expect(
      metadata?.description,
      'Returns the layer names observed by the generated handler.',
    );
    expect(metadata?.tags, ['todos', 'diagnostics']);
    expect(metadata?.deprecated, isTrue);
    expect(metadata?.hidden, isTrue);
  });

  test('omitted operation id defaults to the handwritten function name', () {
    final route = buildApp()
        .describe()
        .singleWhere((route) => route.path == '/api/v1/health');
    final metadata = route.metadataOf<RouteMetadata>();

    expect(metadata?.operationId, 'health');
    expect(metadata?.summary, 'Check fixture health');
    expect(metadata?.tags, ['operations']);
    expect(metadata?.deprecated, isFalse);
    expect(metadata?.hidden, isFalse);
  });
}
