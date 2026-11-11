use super::*;

/// One canonical declaration before independent namespaces are combined.
fn declaration(uri: &str) -> World {
    World {
        declarations: BTreeSet::from([Declaration {
            uri: uri.into(),
            name: "Marker".into(),
            file_id: FileId::new(1),
            start: 0,
            end: 6,
        }]),
        ..Default::default()
    }
}

#[test]
fn neutral_alternatives_keep_all_declarations_and_missing_input_problems() {
    let neutral = BTreeSet::from([World::default()]);
    let alternatives = BTreeSet::from([
        declaration("package:app/marker.dart"),
        World {
            problems: BTreeSet::from([Problem::new(
                FileId::new(2),
                TextRange::empty(0_u32),
                "missing required input".into(),
            )]),
            ..Default::default()
        },
    ]);
    assert_eq!(combine(alternatives.clone(), &neutral), alternatives);
    assert_eq!(combine(neutral, &alternatives), alternatives);
}

#[test]
fn conditional_empty_branches_preserve_sdk_and_non_sdk_alternatives() {
    let sdk = declaration("dart:core");
    let foreign = declaration("package:app/marker.dart");
    assert_eq!(
        combine(
            BTreeSet::from([sdk.clone()]),
            &BTreeSet::from([World::default(), foreign.clone()])
        ),
        BTreeSet::from([sdk, foreign]),
    );
}
