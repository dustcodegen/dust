import 'package:flutter/material.dart';

import 'src/dev/lab.dart';

void main() => runApp(const DustUiLab());

class DustUiLab extends StatelessWidget {
  const DustUiLab({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Dust UI Lab',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(colorSchemeSeed: const Color(0xff14665c)),
    home: const LabScreen(),
  );
}
