use super::*;

#[test]
fn a_new_regular_generation_root_cannot_be_blessed_by_an_old_input_snapshot() {
    let (root, request) = workspace();
    let external = root.path().join("late_external.dart");
    let late = root.path().join("lib/late.dart");
    let source = "import 'package:dust_dart/derive.dart'; part 'late.g.dart'; @Derive([Eq()]) class Late with _$Late { const Late(); }";
    fs::write(&external, source).unwrap();
    std::os::unix::fs::symlink(&external, &late).unwrap();
    let item = root.path().join("lib/item.dart");
    let original = fs::read_to_string(&item).unwrap();
    fs::write(&item, format!("import 'late.dart';\n{original}")).unwrap();
    let result = crate::run_build(request.clone());
    assert_eq!(result.diagnostics, []);
    assert!(
        !result
            .build_artifacts
            .iter()
            .any(|artifact| artifact.source_path == late)
    );
    assert!(!root.path().join("lib/late.g.dart").exists());
    let context = crate::context::CachedDriverContext::load(
        &request.cwd,
        crate::build::RegistrySelection::for_build(request.db),
        request.sdk_source_root.as_deref(),
        request.jobs,
    )
    .unwrap();
    assert!(
        context
            .namespaces
            .input_snapshot()
            .unwrap()
            .contains_key(&late)
    );
    assert!(
        !context
            .namespaces
            .discovered_root_paths()
            .any(|path| path == late)
    );
    fs::remove_file(&late).unwrap();
    fs::write(&late, source).unwrap();
    super::super::save(
        &request,
        &context.workspace,
        &context.namespaces,
        &context.cache,
        &result,
        &Default::default(),
    );
    let proof = Manifest::load(root.path()).unwrap();
    assert!(!proof.lib_files.contains(&late));
    assert!(super::super::try_build(&request, None).is_none());
    let rebuilt = crate::run_build(request);
    assert_eq!(rebuilt.diagnostics, []);
    assert!(
        rebuilt
            .build_artifacts
            .iter()
            .any(|artifact| artifact.source_path == late)
    );
    assert!(root.path().join("lib/late.g.dart").exists());
}
