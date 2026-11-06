use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::NamespaceGraph;
use dust_text::{FileId, SourceText};

use super::{NamespaceSources, hash_text};
use crate::namespace_sources::{index::Source, loader::Unit};

const CORE: &str = "dart:core";
const PART: &str = "dart:core/part.dart";
const FAILED: &str = "dart:failed";
const MISSING: &str = "dart:absent";
const PACKAGE: &str = "package:app/model.dart";
const GENERATED: &str = "package:app/model.g.dart";

fn unit(
    uri: &str,
    id: u32,
    text: &str,
    dependencies: &[&str],
    owner: Option<&str>,
) -> (String, Result<Unit, String>) {
    let file_id = FileId::new(id);
    let text: Arc<str> = text.into();
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(file_id, Arc::clone(&text)),
        ParseOptions::default(),
    );
    (
        uri.into(),
        Ok(Unit {
            source: Source {
                uri: uri.into(),
                path: PathBuf::from(format!("source-{id}.dart")),
                language_version: Default::default(),
            },
            file_id,
            text,
            content_digest: Default::default(),
            parsed: parsed.library,
            diagnostics: parsed.diagnostics,
            namespace_diagnostics: parsed.namespace_diagnostics,
            targets: BTreeMap::new(),
            dependencies: dependencies.iter().map(|uri| (*uri).to_owned()).collect(),
            generated_owner: owner.map(str::to_owned),
            generated_library: false,
        }),
    )
}

fn snapshot() -> NamespaceSources {
    let mut sources = NamespaceSources {
        graph: NamespaceGraph::new(),
        units: BTreeMap::from([
            unit(CORE, 7, "class Object {} // µ", &[PART, FAILED], None),
            unit(PART, 42, "", &[], Some(CORE)),
            (FAILED.into(), Err("unreadable SDK unit".into())),
            unit(PACKAGE, 68, "class Model {}", &[GENERATED], None),
            unit(GENERATED, 69, "", &[], Some("file:///a.dart")),
        ]),
        roots: BTreeMap::new(),
        metadata_hash: 9_321,
        binding_errors: BTreeMap::new(),
        primary_suffix: ".g.dart".into(),
    };
    for (name, id, dependencies) in [
        ("a.dart", 101, &[CORE, MISSING, PACKAGE][..]),
        ("b.dart", 103, &[CORE, MISSING][..]),
        ("unrelated.dart", 105, &[][..]),
    ] {
        let uri = format!("file:///{name}");
        sources.roots.insert(name.into(), uri.clone());
        let (uri, unit) = unit(&uri, id, "// root source", dependencies, None);
        sources.units.insert(uri, unit);
    }
    sources
}

/// Returns a source to mutate before its snapshot is fingerprinted.
fn source_unit<'a>(sources: &'a mut NamespaceSources, uri: &str) -> &'a mut Unit {
    sources.units.get_mut(uri).unwrap().as_mut().unwrap()
}

#[test]
fn reachable_inputs_invalidate_only_dependent_roots() {
    type Mutation = fn(&mut NamespaceSources);
    let cases: &[(&str, Mutation)] = &[
        ("root text", |sources| {
            source_unit(sources, "file:///a.dart").text = "// root edited".into();
        }),
        ("SDK text", |sources| {
            source_unit(sources, CORE).text = "class Object {} // ß".into();
        }),
        ("package text", |sources| {
            source_unit(sources, PACKAGE).text = "class Order {}".into();
        }),
        ("diagnostic file ID", |sources| {
            source_unit(sources, PACKAGE).file_id = FileId::new(102);
        }),
        ("generated owner", |sources| {
            source_unit(sources, GENERATED).generated_owner = Some("file:///b.dart".into());
        }),
        ("failed read", |sources| {
            sources
                .units
                .insert(FAILED.into(), Err("changed read error".into()));
        }),
        ("missing source", |sources| {
            let (uri, value) = unit(MISSING, 106, "", &[], None);
            sources.units.insert(uri, value);
        }),
        ("dependency URI", |sources| {
            const ALIAS: &str = "package:app/alias.dart";
            let value = sources.units.remove(PACKAGE).unwrap();
            sources.units.insert(ALIAS.into(), value);
            let root = source_unit(sources, "file:///a.dart");
            root.dependencies.remove(PACKAGE);
            root.dependencies.insert(ALIAS.into());
        }),
    ];
    let before = snapshot();
    let affected = Path::new("a.dart");
    let unrelated = Path::new("unrelated.dart");
    let original = before.fingerprint(affected);
    let unaffected = before.fingerprint(unrelated);
    for (name, mutate) in cases {
        let mut after = snapshot();
        mutate(&mut after);
        assert_ne!(after.fingerprint(affected), original, "{name}");
        assert_eq!(after.fingerprint(unrelated), unaffected, "{name}");
    }
}

#[test]
fn cyclic_dependencies_keep_every_reachable_uri_in_sorted_order() {
    let mut sources = snapshot();
    source_unit(&mut sources, PACKAGE)
        .dependencies
        .insert(CORE.into());
    source_unit(&mut sources, PART)
        .dependencies
        .insert(CORE.into());
    assert_eq!(
        sources
            .read_set("file:///a.dart")
            .into_iter()
            .collect::<Vec<_>>(),
        [
            MISSING,
            CORE,
            PART,
            FAILED,
            "file:///a.dart",
            PACKAGE,
            GENERATED
        ]
    );
}

#[test]
fn workspace_metadata_affects_every_root() {
    let before = snapshot();
    let mut after = snapshot();
    after.metadata_hash += 1;
    for name in ["a.dart", "b.dart", "unrelated.dart"] {
        let path = Path::new(name);
        assert_ne!(before.fingerprint(path), after.fingerprint(path));
    }
}

#[test]
fn source_content_digests_are_lazy_and_shared_within_a_snapshot() {
    let sources = snapshot();
    let core = sources.units[CORE].as_ref().unwrap();
    let package = sources.units[PACKAGE].as_ref().unwrap();
    assert!(core.content_digest.get().is_none());
    assert!(package.content_digest.get().is_none());

    let b = Path::new("b.dart");
    let first = sources.fingerprint(b);
    assert_eq!(core.content_digest.get(), Some(&hash_text(&core.text)));
    assert!(package.content_digest.get().is_none());

    sources.fingerprint(Path::new("a.dart"));
    assert_eq!(
        package.content_digest.get(),
        Some(&hash_text(&package.text))
    );
    assert_eq!(sources.fingerprint(b), first);
    assert_eq!(snapshot().fingerprint(b), first);
}
