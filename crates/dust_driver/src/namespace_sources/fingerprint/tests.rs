use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::NamespaceGraph;
use dust_text::{FileId, SourceText};

use super::{NamespaceFingerprints, NamespaceSources, hash_text};
use crate::namespace_sources::{index::Source, loader::Unit};

const CORE: &str = "dart:core";
const PART: &str = "dart:core/part.dart";
const FAILED: &str = "dart:failed";
const SUFFIX: &str = "package:app/model.g.dart";

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
            unit(SUFFIX, 68, "", &[], Some("file:///a.dart")),
        ]),
        roots: BTreeMap::new(),
        metadata_hash: 9_321,
        binding_errors: BTreeMap::new(),
        primary_suffix: ".g.dart".into(),
    };
    for (name, id, dependencies) in [
        ("a.dart", 101, &[CORE, "dart:absent-a", SUFFIX][..]),
        ("b.dart", 103, &[CORE, "dart:absent-a"][..]),
        ("c.dart", 104, &[CORE, "dart:absent-b"][..]),
        ("plain.dart", 105, &[SUFFIX][..]),
    ] {
        let uri = format!("file:///{name}");
        sources.roots.insert(name.into(), uri.clone());
        let (uri, unit) = unit(&uri, id, "// root source", dependencies, None);
        sources.units.insert(uri, unit);
    }
    sources
}

fn concatenated_fingerprint(sources: &NamespaceSources, path: &Path) -> u64 {
    let mut text = format!("{}\0", sources.metadata_hash);
    if let Some(root) = sources.roots.get(path) {
        for uri in sources.read_set(root) {
            text.push_str(uri);
            text.push('\0');
            match sources.units.get(uri) {
                Some(Ok(unit)) => {
                    text.push_str(&unit.file_id.raw().to_string());
                    text.push('\0');
                    text.push_str(&unit.text);
                    if let Some(owner) = &unit.generated_owner {
                        text.push_str(owner);
                    }
                }
                Some(Err(error)) => text.push_str(error),
                None => text.push_str("missing namespace source"),
            }
            text.push('\0');
        }
    }
    hash_text(&text)
}

#[test]
fn shared_and_distinct_sdk_prefixes_match_concatenated_fingerprints() {
    let sources = snapshot();
    let mut fingerprints = NamespaceFingerprints::new(&sources);
    for path in ["a.dart", "b.dart", "c.dart", "plain.dart", "unknown.dart"] {
        let path = Path::new(path);
        assert_eq!(
            fingerprints.fingerprint(path),
            concatenated_fingerprint(&sources, path),
            "{path:?}"
        );
    }
    assert_eq!(fingerprints.sdk_prefixes.len(), 3);
    assert!(
        fingerprints
            .sdk_prefixes
            .contains_key(&["dart:absent-a", CORE, PART, FAILED][..])
    );
    assert!(
        fingerprints
            .source_with_fingerprint(Path::new("unknown.dart"))
            .is_none()
    );
}

#[test]
fn identical_sdk_uris_get_fresh_hashes_in_a_new_snapshot() {
    let before = snapshot();
    let mut after = snapshot();
    after.units.get_mut(CORE).unwrap().as_mut().unwrap().text = "// edited SDK".into();

    let path = Path::new("a.dart");
    let previous = NamespaceFingerprints::new(&before).fingerprint(path);
    let current = NamespaceFingerprints::new(&after).fingerprint(path);
    assert_ne!(previous, current);
    assert_eq!(current, concatenated_fingerprint(&after, path));
}
