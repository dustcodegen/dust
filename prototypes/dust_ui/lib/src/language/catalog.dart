import 'model.dart';

const _container = {
  'id': ValueKind.name,
  'gap': ValueKind.spacing,
  'padding': ValueKind.spacing,
  'align': ValueKind.alignment,
  'state': ValueKind.state,
  'visible': ValueKind.boundBool,
};

const visualProperties = {
  'col': _container,
  'row': _container,
  'zstack': {
    'id': ValueKind.name,
    'padding': ValueKind.spacing,
    'align': ValueKind.stackAlignment,
    'state': ValueKind.state,
    'visible': ValueKind.boundBool,
  },
  'text': {
    'value': ValueKind.boundText,
    'size': ValueKind.textSize,
    'weight': ValueKind.weight,
    'visible': ValueKind.boundBool,
  },
  'spacer': <String, ValueKind>{},
  'button': {
    'get': ValueKind.endpoint,
    'post': ValueKind.endpoint,
    'target': ValueKind.name,
    'open': ValueKind.popup,
    'type': ValueKind.submit,
    'disabled': ValueKind.boundBool,
    'expanded': ValueKind.boundBool,
    'accessibilityLabel': ValueKind.label,
    'onTap': ValueKind.actionBlock,
  },
  'form': {
    'id': ValueKind.name,
    'post': ValueKind.endpoint,
    'target': ValueKind.name,
  },
  'input': {
    'name': ValueKind.name,
    'type': ValueKind.hidden,
    'label': ValueKind.label,
    'initial': ValueKind.text,
    'value': ValueKind.boundScalar,
    'required': ValueKind.boolean,
    'placeholder': ValueKind.text,
    'maxLength': ValueKind.length,
    'lines': ValueKind.lines,
  },
  'select': {
    'name': ValueKind.name,
    'label': ValueKind.label,
    'options': ValueKind.options,
    'initial': ValueKind.text,
    'required': ValueKind.boolean,
  },
};

enum ValueKind {
  text('a string'),
  boundText('a string or string state reference'),
  actionBlock('an action block'),
  label('a nonempty string'),
  boolean('a Boolean'),
  boundBool('a Boolean or Boolean state reference'),
  boundScalar('a string, number, Boolean, or state reference'),
  name('a name of up to 64 letters, digits, underscores, or hyphens'),
  spacing('a number from 0 to 4096'),
  textSize('a positive number up to 256'),
  length('an integer from 1 to 4096'),
  lines('an integer from 1 to 20'),
  alignment('start, center, or end'),
  stackAlignment('start, center, end, or bottomEnd'),
  weight('normal or bold'),
  endpoint('a root-relative endpoint such as "/ui/cart"'),
  popup('popup'),
  submit('submit'),
  hidden('hidden'),
  state('a map of initial strings, numbers, or Booleans'),
  options('a nonempty list of distinct, nonempty strings');

  const ValueKind(this.description);

  final String description;

  bool get allowsReference =>
      this == ValueKind.boundBool ||
      this == ValueKind.boundScalar ||
      this == ValueKind.boundText;

  bool accepts(Object? value) => switch (this) {
    text || boundText => value is String,
    actionBlock => value is UiActionBlock,
    label => value is String && value.trim().isNotEmpty,
    boolean || boundBool => value is bool,
    boundScalar => isStateValue(value),
    name => value is String && _matches(_name, value),
    spacing => value is num && value >= 0 && value <= 4096,
    textSize => value is num && value > 0 && value <= 256,
    length => value is int && value >= 1 && value <= 4096,
    lines => value is int && value >= 1 && value <= 20,
    alignment => _oneOf(value, const {'start', 'center', 'end'}),
    stackAlignment => _oneOf(value, const {
      'start',
      'center',
      'end',
      'bottomEnd',
    }),
    weight => _oneOf(value, const {'normal', 'bold'}),
    endpoint =>
      value is String && value.length <= 1024 && _matches(_path, value),
    popup => _oneOf(value, const {'popup'}),
    submit => _oneOf(value, const {'submit'}),
    hidden => _oneOf(value, const {'hidden'}),
    state =>
      value is Map<String, Object?> &&
          value.entries.every(
            (entry) =>
                _matches(_stateName, entry.key) && isStateValue(entry.value),
          ),
    options =>
      value is List<Object?> &&
          value.isNotEmpty &&
          value.every(
            (option) => option is String && option.trim().isNotEmpty,
          ) &&
          value.toSet().length == value.length,
  };
}

bool isStateValue(Object? value) =>
    value is String || value is bool || (value is num && value.isFinite);

final _stateName = RegExp(r'[A-Za-z_][A-Za-z_0-9]*');
final _name = RegExp(r'[A-Za-z_][A-Za-z_0-9-]{0,63}');
final _path = RegExp(r'/(?:[A-Za-z0-9_-]+/)*[A-Za-z0-9_-]+');

bool _matches(RegExp pattern, String value) =>
    pattern.matchAsPrefix(value)?.end == value.length;

bool _oneOf(Object? value, Set<String> names) =>
    value is UiSymbol && names.contains(value.name);
