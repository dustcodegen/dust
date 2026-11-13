# Dust UI Lab

Dust UI Lab is an experimental authoring and preview tool for source-driven
Flutter screens. A UI response is UTF-8 source with one visual element at the
root. The demo server uses Dart to choose the empty or populated cart and
expand item rows before it sends the document.

## Run the lab

From this directory, with Flutter 3.44 or later on PATH:

```sh
flutter pub get
dart run bin/dust_ui.dart check demo/product.dustui
dart run bin/dust_ui.dart preview
```

Open the loopback URL printed by the command. The first step resolves the
prototype's dependencies. The preview command builds Flutter Web and runs the
lab and mock routes from the same local origin. Stop it with Ctrl+C.

- Edit the source pane to update the preview. Select a diagnostic to move the
  editor cursor to its line and column. A syntax or catalog error keeps the
  last valid preview interactive.
- Copy the source to save browser edits in your editor. Reload source from disk
  picks up external edits and replaces any edits in the browser. To start with
  another file, run `dart run bin/dust_ui.dart preview --source path/to/file.dustui`.
- A valid source edit remounts the preview. The mock cart lives in memory until
  the server stops. Reloading the bundled product fixture restores its initial
  cart summary. **Refresh cart** and **View cart** read the current server cart.

Try **Product details** and **Size guide** first. Choose a size, enter a note,
and select **Add to cart**. The POST response replaces the cart summary.
**View cart** fetches another source document and displays it as a dialog.

## Document contract

```text
col(gap: 12, state: {expanded: false}) {
  button(
    expanded: state.expanded,
    onTap: { set(expanded: !state.expanded) }
  ) {
    text(value: "Details")
  }

  text(value: "Pure linen.", visible: state.expanded)
}
```

Every argument is a named property. The trailing block holds visual children.
A button holds one visual child, which can be a text or a layout tree.
Its optional `accessibilityLabel` supplies a screen-reader label.

- Elements are `col`, `row`, `zstack`, `text`, `spacer`, `button`, `form`,
  `input`, and `select`. Unsupported elements and properties produce diagnostics.
- State belongs to the nearest declaring container. References are
  `state.name` and `!state.name` for Boolean negation. An `onTap` property holds
  one action. Actions are `set`, `popup`, and `dismiss`.
- Use `button(get: "/ui/cart", open: popup) { text(value: "Cart") }` for a
  fetched dialog, or specify `target: "cart"` to replace an element on the
  current surface. Replacement source must carry the same ID. Each popup
  has its own state scope and ID namespace.
- A form POST collects named inputs, including explicit hidden values.
  Editable inputs use `initial`. Invisible branches retain drafts and skip
  field validation and submission. A standalone POST button sends an empty
  object. A submit button collects the fields in its containing form.
- The checker validates targets in a complete screen. Use
  `dart run bin/dust_ui.dart check --fragment path/to/region.dustui` for a
  response fragment whose targets may belong to the containing screen.
  The renderer validates the combined tree before installing a response.
- The lab accepts bounded documents and sends requests only to its local
  `/ui/` endpoints. The mock routes use an in-memory cart and validate POST
  fields. Server Dart expands templates, loops, and branches into the
  resolved source sent to the Flutter client.

## Workstreams

[Milestone](https://github.com/dustcodegen/dust/milestone/4) ·
[Parent issue](https://github.com/dustcodegen/dust/issues/227)

| Track | Owns | Verify |
| --- | --- | --- |
| [Language #229](https://github.com/dustcodegen/dust/issues/229) | Parser and catalog in `lib/src/language/` | Parser and CLI tests |
| [Runtime #228](https://github.com/dustcodegen/dust/issues/228) | Flutter rendering in `lib/src/runtime/` and the mock cart contract | Widget, route and cart-flow checks |
| [Tooling #230](https://github.com/dustcodegen/dust/issues/230) | `bin/`, preview editor, and local server | CLI, server and browser checks |

Run the local checks before handing off changes:

```sh
dart format --output=none --set-exit-if-changed bin lib test
flutter analyze --no-pub
flutter test --no-pub
```

The prototype is outside the release package workspace. Dust's local
pre-commit gate still applies before a repository commit.
