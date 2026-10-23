use super::*;

#[test]
fn emits_every_route_metadata_field() {
    assert_eq!(
        render_metadata(
            &RouteMetadataPlan {
                operation_id: "readTodo".to_owned(),
                summary: Some("Read 'todo'".to_owned()),
                description: Some("Returns one todo".to_owned()),
                tags: vec!["todos".to_owned(), "read".to_owned()],
                deprecated: true,
                hidden: true,
            },
            "  ",
        ),
        "const RouteMetadata(\n    operationId: 'readTodo',\n    summary: 'Read \\'todo\\'',\n    description: 'Returns one todo',\n    tags: ['todos', 'read'],\n    deprecated: true,\n    hidden: true,\n  )"
    );
}
