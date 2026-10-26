# `dust_dart` Guide

Use `dust_dart` for Dart-only annotations and runtime types. It also works in
Flutter applications; do not add `dust_flutter` unless the task needs Flutter
routing, state, or i18n.

## Choose the focused library

| Need | Import | Required authoring shape |
| :--- | :--- | :--- |
| Data class | `package:dust_dart/derive.dart` | `@Derive(...)`, matching `part`, and `with _$Type` |
| JSON | `package:dust_dart/serde.dart` | `Serialize()` and/or `Deserialize()` derive |
| Validation | `package:dust_dart/derive.dart` | class-level `Validate()` plus field rules |
| HTTP client | `package:dust_dart/http.dart` | `@HttpClient` abstract interface and redirecting factory |
| Functional types | `package:dust_dart/fp.dart` | handwritten `Option` or `Result` use; no generation |
| Database declarations | `package:dust_dart/db.dart` | row derives, database/DAO roots, and static SQL |

Add only the derive capabilities the caller uses. Keep existing fields, JSON
names, validation behavior, HTTP paths, and method signatures stable. Infer
field types from the declared Dart fields; do not add redundant annotation type
arguments.

For JSON, use Dust's generated `serialize`, `toJson`, and forwarding `fromJson`
contract instead of maintaining a second handwritten decoder. For HTTP, keep
one verb annotation per method and use explicit parameter annotations such as
`@Path`, named `@Query`, and `@Body`.

Run `dust build`, `dust check`, `dart analyze`, and `dart test`. In a Flutter
app, use `flutter analyze` and `flutter test` for the application checks.

Source documentation:

- [Data classes](https://github.com/dustcodegen/dust/blob/main/docs/usage/derive.md)
- [JSON](https://github.com/dustcodegen/dust/blob/main/docs/usage/serde.md)
- [Validation](https://github.com/dustcodegen/dust/blob/main/docs/usage/validation.md)
- [HTTP clients](https://github.com/dustcodegen/dust/blob/main/docs/usage/http.md)
- [Database declarations](https://github.com/dustcodegen/dust/blob/main/docs/usage/db.md)
