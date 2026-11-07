part of 'view.dart';

class _FormValues {
  _FormValues(this.slot);

  final _Slot slot;
  final key = GlobalKey<FormState>();
  final _values = <String, Object?>{};

  Map<String, Object?>? capture() {
    final form = key.currentState;
    if (form == null || !form.validate()) return null;
    _values.clear();
    form.save();
    return Map.of(_values);
  }

  Widget field(
    UiNode node,
    Map<String, Object?> scope, {
    required bool active,
  }) {
    final name = node.props['name']! as String;
    void save(Object? value) {
      if (active) _values[name] = value;
    }

    if (_symbol(node.props['type']) == 'hidden') {
      return FormField<Object?>(
        builder: (_) => const SizedBox.shrink(),
        onSaved: (_) => save(_resolve(node.props['value'], scope)),
      );
    }

    String? validate(String? value) =>
        active &&
            node.props['required'] == true &&
            (value == null || value.trim().isEmpty)
        ? 'Required'
        : null;

    final decoration = InputDecoration(
      labelText: node.props['label'] as String?,
      hintText: node.props['placeholder'] as String?,
    );
    final initial = node.props['initial'] as String?;
    if (node.name == 'select') {
      return DropdownButtonFormField<String>(
        decoration: decoration,
        initialValue: initial,
        isExpanded: true,
        items: [
          for (final item
              in (node.props['options']! as List<Object?>).cast<String>())
            DropdownMenuItem(value: item, child: Text(item)),
        ],
        onChanged: (_) {},
        onSaved: save,
        validator: validate,
      );
    }
    return TextFormField(
      decoration: decoration,
      initialValue: initial,
      maxLength: node.props['maxLength'] as int?,
      maxLines: node.props['lines'] as int? ?? 1,
      onSaved: (value) => save(value ?? ''),
      validator: validate,
    );
  }
}
