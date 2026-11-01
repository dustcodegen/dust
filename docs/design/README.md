# Design notes

These documents record contributor work that is not fully represented by the
current public guides. Current application behavior belongs in
[Using Dust](../usage/README.md), not in a design note.

## Active or incomplete work

### Annotation namespaces

- [Annotation namespace ownership](./annotation-namespaces.md): approved policy;
  implementation in progress.
- [Lexical annotation ownership](./annotation-lexical-scopes.md): approved
  design; implementation in progress.
- [Source-fact annotation namespace graph](./annotation-namespace-graph.md):
  production integration in progress.
- [Directive namespace facts](./directive-namespace-facts.md): facts layer
  implemented; binding unfinished.
- [Producer-owned generated library inputs](./generated-library-ownership.md):
  implementation awaiting full acceptance.
- [Namespace interface diagnostics](./namespace-interface-diagnostics.md):
  implementation in progress.

### Dart language support

- [Class interfaces in IR](./class-interfaces.md): proposal.
- [Effective types for shorthand function formals](./function-formal-types.md):
  approved; implementation and acceptance checks in progress.
- [Generic model generation](./generic-models.md): approved design;
  implementation in progress.

## Retained decisions

- [Database query design](./db-queries.md): settled ownership and runtime
  boundaries; current usage is in the [Database guide](../usage/db.md).
- [Server runtime axum parity](./server-axum-parity.md): historical naming
  decision; current usage is in the [server guides](../dust_server/README.md).
