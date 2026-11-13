use super::*;
use crate::namespace_sources::{index::Source, loader::Unit};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::NamespaceGraph;
use dust_text::{FileId, SourceText};

const FILE: &str = "file:///workspace/lib/model.dart";
const PACKAGE: &str = "package:example/model.dart";
const MODEL: &str = "/workspace/lib/model.dart";
const TEXT: &str = "class Model {}";

fn unit(uri: &str, path: &str, text: &str) -> Unit {
    let file_id = FileId::new(1);
    let text: Arc<str> = text.into();
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(file_id, Arc::clone(&text)),
        ParseOptions::default(),
    );
    Unit {
        source: Source {
            uri: uri.into(),
            path: path.into(),
            language_version: Default::default(),
        },
        file_id,
        text,
        content_digest: Default::default(),
        parsed: parsed.library,
        diagnostics: parsed.diagnostics,
        namespace_diagnostics: parsed.namespace_diagnostics,
        targets: BTreeMap::new(),
        dependencies: BTreeSet::new(),
        generated_owner: None,
        generated_library: false,
    }
}

fn fixture() -> NamespaceSources {
    NamespaceSources {
        graph: NamespaceGraph::new(),
        units: BTreeMap::from([(FILE.into(), Ok(unit(FILE, MODEL, TEXT)))]),
        roots: BTreeMap::from([(MODEL.into(), FILE.into())]),
        fingerprint_index: Default::default(),
        metadata_hash: 0,
        metadata_inputs: BTreeMap::from([("/sdk/version".into(), hash_text("3.13.0"))]),
        parser_facts_changed: false,
        binding_errors: BTreeMap::new(),
        primary_suffix: ".g.dart".into(),
    }
}

#[test]
fn equal_path_aliases_collapse_only_when_their_original_digests_agree() {
    let mut sources = fixture();
    sources
        .units
        .insert(PACKAGE.into(), Ok(unit(PACKAGE, MODEL, TEXT)));
    let snapshot = sources.input_snapshot().unwrap();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[Path::new(MODEL)], hash_text(TEXT));
    let cached = sources.units[FILE].as_ref().unwrap().content_digest.get();
    assert_eq!(cached, Some(&hash_text(TEXT)));

    sources
        .units
        .insert(PACKAGE.into(), Ok(unit(PACKAGE, MODEL, "class Changed {}")));
    assert!(sources.input_snapshot().is_none());
}

#[test]
fn metadata_conflicts_are_rejected_and_equal_paths_keep_the_first_key_spelling() {
    let mut sources = fixture();
    let original = PathBuf::from("/workspace/lib/./model.dart");
    assert_eq!(original, Path::new(MODEL));
    sources
        .metadata_inputs
        .insert(original.clone(), hash_text(TEXT));
    let snapshot = sources.input_snapshot().unwrap();
    let (retained, digest) = snapshot.get_key_value(Path::new(MODEL)).unwrap();
    assert_eq!(retained.as_os_str(), original.as_os_str());
    assert_eq!(*digest, hash_text(TEXT));

    sources
        .metadata_inputs
        .insert(original, hash_text("different metadata"));
    assert!(sources.input_snapshot().is_none());
}

#[test]
fn generated_units_are_filtered_but_any_retained_read_failure_disables_the_proof() {
    let mut sources = fixture();
    let mut owned = unit("file:///owned.dart", MODEL, "conflicting owned placeholder");
    owned.generated_owner = Some(FILE.into());
    let mut generated = unit(
        "file:///generated.dart",
        MODEL,
        "conflicting generated placeholder",
    );
    generated.generated_library = true;
    sources.units.insert(owned.source.uri.clone(), Ok(owned));
    sources
        .units
        .insert(generated.source.uri.clone(), Ok(generated));
    let snapshot = sources.input_snapshot().unwrap();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[Path::new(MODEL)], hash_text(TEXT));

    sources.units.insert(
        "file:///missing.dart".into(),
        Err("source read failed".into()),
    );
    assert!(sources.input_snapshot().is_none());
}
