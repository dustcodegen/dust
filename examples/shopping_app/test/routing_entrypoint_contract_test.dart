import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/route.dart';

void main() {
  test('route.dart exposes the shopping cart navigation contract', () {
    final routes = <ShoppingRoute>[
      const CartRoute(),
      const CheckoutRoute(),
      const OrderConfirmationRoute(orderId: 'ORDER 1/2'),
    ];

    expect(
      routes.map(shoppingRouteLocation).toList(),
      const [
        '/cart',
        '/checkout',
        '/order-confirmation/ORDER%201%2F2',
      ],
    );

    expect(
      parseShoppingRoute(Uri.parse('/cart')).runtimeType,
      CartRoute,
    );
    expect(
      parseShoppingRoute(Uri.parse('/checkout')).runtimeType,
      CheckoutRoute,
    );

    final confirmation = parseShoppingRoute(
      Uri.parse('/order-confirmation/ORDER%201%2F2'),
    );
    expect(confirmation.runtimeType, OrderConfirmationRoute);
    expect(
      (confirmation as OrderConfirmationRoute).orderId,
      'ORDER 1/2',
    );
  });

  test('route.dart is the only handwritten owner of the generated barrel', () {
    expect(
      _generatedBarrelReferences(),
      const [
        (
          file: 'lib/route.dart',
          directive: 'import',
          uri: 'route/routes.g.dart',
        ),
        (
          file: 'lib/route.dart',
          directive: 'export',
          uri: 'route/routes.g.dart',
        ),
      ],
    );
  });

  test('native projects register the documented shopping link scheme', () {
    final android = File(
      'android/app/src/main/AndroidManifest.xml',
    ).readAsStringSync();
    final ios = File('ios/Runner/Info.plist').readAsStringSync();

    expect(android, contains('android:name="flutter_deeplinking_enabled"'));
    expect(android, contains('android:scheme="shopping"'));
    expect(android, contains('android:host="open"'));
    expect(ios, contains('<key>FlutterDeepLinkingEnabled</key>'));
    expect(ios, contains('<string>shopping</string>'));
  });

  test('Apple projects use the supported deployment floors', () {
    final iosPodfile = File('ios/Podfile').readAsStringSync();
    final iosProject = File(
      'ios/Runner.xcodeproj/project.pbxproj',
    ).readAsStringSync();
    final macosPodfile = File('macos/Podfile').readAsStringSync();
    final macosProject = File(
      'macos/Runner.xcodeproj/project.pbxproj',
    ).readAsStringSync();

    expect(iosPodfile, contains("platform :ios, '15.0'"));
    expect(iosProject, contains('IPHONEOS_DEPLOYMENT_TARGET = 15.0;'));
    expect(macosPodfile, contains("platform :osx, '12.0'"));
    expect(macosProject, contains('MACOSX_DEPLOYMENT_TARGET = 12.0;'));
  });
}

List<({String directive, String file, String uri})>
    _generatedBarrelReferences() {
  final root = Directory.current;
  final files = ['lib', 'test']
      .map((path) => Directory('${root.path}/$path'))
      .where((directory) => directory.existsSync())
      .expand((directory) => directory.listSync(recursive: true))
      .whereType<File>()
      .where((file) => file.path.endsWith('.dart'))
      .where((file) => !_isGenerated(file))
      .toList()
    ..sort((a, b) => a.path.compareTo(b.path));

  final references = <({String directive, String file, String uri})>[];
  final directive = RegExp(r"^\s*(import|export)\s+'([^']+)';");
  for (final file in files) {
    final relativePath = file.path.substring(root.path.length + 1);
    final lines = file.readAsLinesSync();
    for (final line in lines) {
      final match = directive.firstMatch(line);
      if (match == null) continue;

      final uri = match.group(2)!;
      if (uri != 'route/routes.g.dart') continue;

      references.add((
        file: relativePath,
        directive: match.group(1)!,
        uri: uri,
      ));
    }
  }
  return references;
}

bool _isGenerated(File file) => file.path.endsWith('.g.dart');
