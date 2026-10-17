use std::fs;

use dust_driver::{BuildRequest, DbRequestOptions, run_build};

use super::super::helpers::write_db_workspace;
use crate::support::{make_workspace, write_file};

#[test]
fn switching_to_db_mode_preserves_router_outputs_without_an_empty_primary() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_db_workspace(root, false);
    write_file(
        &root.join("lib/route.dart"),
        r"import 'package:dust_flutter/route.dart';
@AppRoute('/', name: 'home')
final class HomePage { const HomePage(); }
@AppRoute('/404', name: 'notFound')
final class NotFoundPage { const NotFoundPage(); }
@AppRouter(initial: '/', notFound: '/404')
final class TestRouter extends $TestRouter { const TestRouter(); }
",
    );
    let build = |only_db| {
        let result = run_build(BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: true,
            jobs: None,
            db: DbRequestOptions {
                only_db,
                offline: false,
            },
        });
        assert_eq!(result.diagnostics, vec![]);
        result
    };
    let first = build(false);
    let paths = &first
        .build_artifacts
        .iter()
        .find(|artifact| artifact.source_path.ends_with("route.dart"))
        .unwrap()
        .auxiliary_output_paths;
    assert!(!paths.is_empty());
    let read_outputs = || {
        paths
            .iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect::<Vec<_>>()
    };
    let original = read_outputs();
    assert!(!root.join("lib/route.g.dart").exists());
    build(true);
    assert!(!root.join("lib/route.g.dart").exists());
    assert_eq!(read_outputs(), original);
    build(false);
    assert!(!root.join("lib/route.g.dart").exists());
    assert_eq!(read_outputs(), original);
}
