use dust_emitter::{AuxiliaryWriteResult, WriteResult};
use dust_plugin_api::{LibraryAnalysisSnapshot, SymbolPlan};

use super::*;
use crate::build::process::OutputSnapshot;

fn fixture(primary: &str) -> (tempfile::TempDir, WorkspacePlan, WriteResult, BuildArtifact) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("pubspec.yaml"), "name: output_proof\n").unwrap();
    crate::namespace_sources::test_workspace::prepare(root.path(), root.path(), "output_proof");
    fs::create_dir_all(root.path().join("lib")).unwrap();
    let workspace = dust_workspace::load_workspace(root.path()).unwrap();
    let artifact = BuildArtifact {
        source_path: root.path().join("lib/source.dart"),
        output_path: root.path().join("lib/source.g.dart"),
        auxiliary_output_paths: vec![
            root.path().join("lib/first.dart"),
            root.path().join("lib/second.dart"),
        ],
        ..artifact()
    };
    let mut output = WriteResult {
        source: primary.to_owned(),
        output_hash: 0,
        symbols: SymbolPlan::default(),
        diagnostics: Vec::new(),
        changed: false,
        written: false,
        suppress_primary_output: false,
        output_path: artifact.output_path.clone(),
        auxiliary_outputs: artifact
            .auxiliary_output_paths
            .iter()
            .map(|path| AuxiliaryWriteResult {
                source: "auxiliary output".to_owned(),
                changed: false,
                written: false,
                output_path: path.clone(),
            })
            .collect(),
    };
    output.output_hash = dust_emitter::hash_output_set(
        std::iter::once((output.output_path.as_path(), output.source.as_str())).chain(
            output
                .auxiliary_outputs
                .iter()
                .map(|output| (output.output_path.as_path(), output.source.as_str())),
        ),
    );
    (root, workspace, output, artifact)
}

fn cache(workspace: &WorkspacePlan, artifact: &BuildArtifact, expected: u64) -> WorkspaceCache {
    let mut cache = WorkspaceCache::load(&workspace.cache_root).unwrap();
    cache.insert(
        &workspace.cache_root,
        &artifact.source_path,
        dust_cache::CacheEntry {
            source_hash: 1,
            package_config_hash: 2,
            tool_hash: 3,
            expected_output_hash: expected,
            auxiliary_output_paths: artifact.auxiliary_output_paths.clone(),
            suppress_primary_output: false,
            workspace_analysis_hash: 0,
            analysis_snapshot: LibraryAnalysisSnapshot::default(),
        },
    );
    cache
}

fn snapshot(
    workspace: &WorkspacePlan,
    cache: &WorkspaceCache,
    artifact: &BuildArtifact,
    fresh: &OutputSnapshots,
) -> Option<BTreeMap<PathBuf, Option<u64>>> {
    output_snapshot(
        workspace,
        cache,
        std::slice::from_ref(artifact),
        3,
        2,
        OutputEvidence {
            fresh,
            previous: None,
        },
    )
}

#[test]
fn fresh_output_facts_avoid_rereads_but_live_changes_and_deletion_reject_reuse() {
    let (_root, workspace, output, artifact) = fixture("fresh primary");
    let cache = cache(&workspace, &artifact, output.output_hash);
    let fresh = BTreeMap::from([(
        artifact.source_path.clone(),
        OutputSnapshot::from_output(&output, true).unwrap(),
    )]);
    let files = snapshot(&workspace, &cache, &artifact, &fresh).unwrap();
    assert_eq!(
        files[&artifact.output_path],
        Some(hash_text(&output.source))
    );
    // No output exists yet: proof construction can use trusted bytes, reuse cannot.
    assert!(!matches(&files));
    assert!(snapshot(&workspace, &cache, &artifact, &Default::default()).is_none());
    fs::write(&output.output_path, &output.source).unwrap();
    for auxiliary in &output.auxiliary_outputs {
        fs::write(&auxiliary.output_path, &auxiliary.source).unwrap();
    }
    assert!(matches(&files));
    fs::write(&output.output_path, "changed after emission").unwrap();
    assert!(!matches(&files));
    fs::remove_file(&output.output_path).unwrap();
    assert!(!matches(&files));
}

#[test]
fn fresh_facts_require_the_cache_hash_and_both_complete_ordered_path_sets() {
    let (_root, workspace, output, artifact) = fixture("fresh primary");
    let cache = cache(&workspace, &artifact, output.output_hash);
    for mode in 0..4 {
        let mut changed = output.clone();
        match mode {
            0 => changed.output_hash ^= 1,
            1 => changed.output_path = workspace.package_root.join("lib/foreign.g.dart"),
            2 => changed.auxiliary_outputs.reverse(),
            _ => {
                changed.auxiliary_outputs.pop();
            }
        }
        let fresh = BTreeMap::from([(
            artifact.source_path.clone(),
            OutputSnapshot::from_output(&changed, true).unwrap(),
        )]);
        assert!(snapshot(&workspace, &cache, &artifact, &fresh).is_none());
    }
    let fresh = BTreeMap::from([(
        artifact.source_path.clone(),
        OutputSnapshot::from_output(&output, true).unwrap(),
    )]);
    let mut wrong_cache = cache.clone();
    let mut entry = wrong_cache
        .get(&workspace.cache_root, &artifact.source_path)
        .unwrap()
        .clone();
    entry.auxiliary_output_paths.reverse();
    wrong_cache.insert(&workspace.cache_root, &artifact.source_path, entry);
    assert!(snapshot(&workspace, &wrong_cache, &artifact, &fresh).is_none());
}

#[test]
fn empty_suppressed_failed_and_check_outputs_keep_live_presence_validation() {
    let (_root, workspace, mut output, artifact) = fixture("");
    let cache = cache(&workspace, &artifact, output.output_hash);
    assert!(OutputSnapshot::from_output(&output, true).is_none());
    for auxiliary in &output.auxiliary_outputs {
        fs::write(&auxiliary.output_path, &auxiliary.source).unwrap();
    }
    let absent = snapshot(&workspace, &cache, &artifact, &Default::default()).unwrap();
    assert_eq!(absent[&output.output_path], None);
    fs::write(&output.output_path, "").unwrap();
    let empty = snapshot(&workspace, &cache, &artifact, &Default::default()).unwrap();
    assert_eq!(empty[&output.output_path], Some(hash_text("")));
    assert!(!matches(&absent));
    output.source = "fresh primary".to_owned();
    assert!(OutputSnapshot::from_output(&output, false).is_none());
    output.suppress_primary_output = true;
    assert!(OutputSnapshot::from_output(&output, true).is_none());
    output.suppress_primary_output = false;
    output
        .diagnostics
        .push(dust_diagnostics::Diagnostic::error("failed emission"));
    assert!(OutputSnapshot::from_output(&output, true).is_none());
}
