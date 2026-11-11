use dust_parser_dart::DartLanguageVersion;

use super::*;

fn source() -> Source {
    Source {
        uri: "package:external/model.dart".into(),
        path: PathBuf::from("external/model.dart"),
        language_version: DartLanguageVersion::DART_3_12,
    }
}

fn input(text: &str) -> SourceInput {
    SourceInput {
        text: Arc::from(text),
        owner: None,
        generated_library: false,
    }
}

#[test]
fn workspace_roots_keep_complete_payloads_while_external_units_keep_namespaces() {
    let source = source();
    let input = input("class Model { Model(this.value); final int value; int method() => value; }");
    let root_policy = ParsePolicy::new(std::slice::from_ref(&source), &[], &BTreeMap::new());
    let root = input.parse(&source, FileId::new(1), &root_policy);
    let external = input.parse(&source, FileId::new(1), &ParsePolicy::default());
    assert_eq!(external.library.lexical_scopes, root.library.lexical_scopes);
    assert_eq!(
        external.library.classes[0].name,
        root.library.classes[0].name
    );
    assert_eq!(
        external.library.classes[0].span,
        root.library.classes[0].span
    );
    assert_eq!(root.library.classes[0].fields.len(), 1);
    assert_eq!(root.library.classes[0].methods.len(), 1);
    assert!(external.library.classes[0].fields.is_empty());
    assert!(external.library.classes[0].methods.is_empty());
}

#[test]
fn supported_external_member_metadata_keeps_complete_payloads() {
    let source = source();
    let input = input("class Model { @d.Validate() final int value = 1; int method() => value; }");
    let full = input.parse(
        &source,
        FileId::new(1),
        &ParsePolicy::new(std::slice::from_ref(&source), &[], &BTreeMap::new()),
    );
    let external = input.parse(
        &source,
        FileId::new(1),
        &ParsePolicy::new(&[], &["Validate"], &BTreeMap::new()),
    );
    assert_eq!(external, full);
}

#[test]
fn foreign_method_diagnostics_and_source_labels_survive_projection() {
    let source = source();
    let input = input("class Model { Object method() => .named(); }");
    let full = input.parse(
        &source,
        FileId::new(19),
        &ParsePolicy::new(std::slice::from_ref(&source), &[], &BTreeMap::new()),
    );
    let external = input.parse(&source, FileId::new(19), &ParsePolicy::default());
    assert!(!full.diagnostics.is_empty());
    assert_eq!(external.diagnostics, full.diagnostics);
    assert_eq!(external.namespace_diagnostics, full.namespace_diagnostics);
}

#[test]
fn preloaded_roots_retain_discovery_bytes_for_exact_live_validation() {
    let root = tempfile::tempdir().unwrap();
    let mut source = source();
    source.path = root.path().join("model.dart");
    let text = Arc::<str>::from("class Original {}");
    fs::write(&source.path, text.as_ref()).unwrap();
    let snapshots = BTreeMap::from([(source.path.clone(), Arc::clone(&text))]);
    let policy = ParsePolicy::new(std::slice::from_ref(&source), &[], &snapshots);
    let preloaded = policy.preloaded_input(&source).unwrap();
    assert!(Arc::ptr_eq(&preloaded.text, &text));
    assert!(preloaded.owner.is_none());
    assert!(!preloaded.generated_library);
    assert!(preloaded == SourceInput::read(&source, None, &BTreeSet::new()).unwrap());

    for current in ["class Modified {}", dust_plugin_api::GENERATED_HEADER] {
        fs::write(&source.path, current).unwrap();
        let live = SourceInput::read(&source, None, &BTreeSet::new()).unwrap();
        assert!(preloaded != live);
        assert_eq!(
            policy.preloaded_input(&source).unwrap().text.as_ref(),
            "class Original {}"
        );
    }
    fs::remove_file(&source.path).unwrap();
    assert!(SourceInput::read(&source, None, &BTreeSet::new()).is_err());
    source.path = root.path().join("dependency.dart");
    assert!(policy.preloaded_input(&source).is_none());
}
