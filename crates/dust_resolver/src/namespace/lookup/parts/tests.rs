use super::*;

#[test]
fn name_ordered_part_lookup_keeps_missing_names_duplicates_and_original_spans() {
    const OWNER: &str = "package:example/owner.dart";
    const PART: &str = "package:example/piece.dart";
    let part_id = FileId::new(2);
    let label = SourceLabel::new(FileId::new(1), TextRange::new(4_u32, 12_u32), "part");
    let owner = Unit {
        file_id: FileId::new(1),
        lexical_scopes: Vec::new(),
        library_name: None,
        declarations: BTreeMap::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        parts: vec![(Ok(PART.into()), label.clone())],
        part_of: None,
    };
    let part = Unit {
        file_id: part_id,
        lexical_scopes: Vec::new(),
        library_name: None,
        declarations: [
            ("Beta", 2, 8),
            ("Middle", 20, 28),
            ("Middle", 40, 48),
            ("Omega", 60, 68),
        ]
        .map(|(name, start, end)| {
            (
                Declaration {
                    uri: PART.into(),
                    name: name.into(),
                    file_id: part_id,
                    start,
                    end,
                },
                DeclarationKind::Concrete,
            )
        })
        .into(),
        imports: Vec::new(),
        exports: Vec::new(),
        parts: Vec::new(),
        part_of: Some(PartOwner::Uri(Ok(OWNER.into()), label)),
    };
    let graph = NamespaceGraph {
        units: BTreeMap::from([(OWNER.into(), owner), (PART.into(), part)]),
        ..Default::default()
    };
    for name in ["Alpha", "Gamma", "November", "Zulu"] {
        assert_eq!(graph.local_world(OWNER, name), World::default(), "{name}");
    }
    let world = graph.local_world(OWNER, "Middle");
    assert!(world.problems.is_empty());
    assert_eq!(
        world
            .declarations
            .iter()
            .map(|declaration| (
                declaration.uri.as_str(),
                declaration.name.as_str(),
                declaration.file_id,
                declaration.start,
                declaration.end,
            ))
            .collect::<Vec<_>>(),
        [
            (OWNER, "Middle", part_id, 20, 28),
            (OWNER, "Middle", part_id, 40, 48),
        ]
    );
    assert_eq!(
        world
            .declarations
            .iter()
            .map(Declaration::label)
            .map(|label| (label.file_id, label.range))
            .collect::<Vec<_>>(),
        [
            (part_id, TextRange::new(20_u32, 28_u32)),
            (part_id, TextRange::new(40_u32, 48_u32))
        ]
    );
}
