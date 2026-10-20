use std::{fs, path::Path};

use dust_driver::{BuildRequest, CheckRequest, DbRequestOptions, run_build, run_check};

use crate::support::{make_workspace, sdk_source_root, write_file};

const ROUTER: &str = r#"import 'package:dust_flutter/route.dart';
import 'route/routes.g.dart';
export 'route/routes.g.dart';
@AppRouter(initial: '/', notFound: '/404')
class Router extends $Router { const Router(); }
@AppRoute('/', name: 'home')
class HomePage { const HomePage(); }
@AppRoute('/404', name: 'notFound')
class MissingPage { const MissingPage(); }
"#;

fn build(root: &Path) -> dust_driver::CommandResult {
    run_build(BuildRequest {
        sdk_source_root: Some(sdk_source_root(root)),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}

fn check(root: &Path) -> dust_driver::CommandResult {
    run_check(CheckRequest {
        sdk_source_root: Some(sdk_source_root(root)),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}

fn outputs(root: &Path) -> Vec<Vec<u8>> {
    ["routes", "paths", "metadata", "navigation", "runtime"]
        .map(|name| fs::read(root.join(format!("lib/route/{name}.g.dart"))).unwrap())
        .to_vec()
}

#[test]
fn cold_alias_and_prefixed_producers_match_direct_outputs_and_warm_checks() {
    for alias in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        let route = root.join("lib/route.dart");
        write_file(&route, ROUTER);
        assert_eq!(build(root).diagnostics, []);
        let expected = outputs(root);
        for name in ["routes", "paths", "metadata", "navigation", "runtime"] {
            fs::remove_file(root.join(format!("lib/route/{name}.g.dart"))).unwrap();
        }
        write_file(
            &root.join("lib/annotations.dart"),
            if alias {
                "import 'package:dust_flutter/route.dart'; export 'package:dust_flutter/route.dart' show AppRoute; typedef RouterAnnotation = AppRouter;"
            } else {
                "export 'package:dust_flutter/route.dart';"
            },
        );
        let source = if alias {
            ROUTER
                .replace("package:dust_flutter/route.dart", "annotations.dart")
                .replace("@AppRouter", "@RouterAnnotation")
        } else {
            ROUTER
                .replace(
                    "import 'package:dust_flutter/route.dart';",
                    "import 'annotations.dart' as ann;",
                )
                .replace("@AppRouter", "@ann.AppRouter")
                .replace("@AppRoute", "@ann.AppRoute")
        };
        write_file(&route, &source);
        write_file(
            &root.join("dust.yaml"),
            "outputs:\n  primary_suffix: .custom.dart\n",
        );
        assert_eq!(build(root).diagnostics, []);
        assert_eq!(outputs(root), expected);
        assert_eq!(check(root).diagnostics, []);
        assert!(!root.join("lib/route.custom.dart").exists());
    }
}

#[test]
fn unmarked_output_collisions_preserve_existing_files_and_successful_cache() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(&root.join("lib/route.dart"), ROUTER);
    assert_eq!(build(root).diagnostics, []);
    let prior = outputs(root);
    let cache = fs::read(root.join(".dart_tool/dust/build_cache_v1.json")).unwrap();
    let path = root.join("lib/route/routes.g.dart");
    for source in ["class Foreign {}", "class {"] {
        write_file(&path, source);
        for result in [build(root), check(root)] {
            assert!(result.has_errors());
            assert!(
                result.diagnostics.iter().any(|error| error
                    .message
                    .contains("collides with an unmarked existing source")
                    || error.message.contains("cannot parse namespace source")),
                "{:?}",
                result.diagnostics
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
            assert_eq!(&outputs(root)[1..], &prior[1..]);
            assert_eq!(
                fs::read(root.join(".dart_tool/dust/build_cache_v1.json")).unwrap(),
                cache
            );
        }
    }
}

#[test]
fn foreign_or_ignored_site_producers_and_unrelated_missing_libraries_do_not_authorize_outputs() {
    for source in [
        ROUTER.replace("@AppRouter(initial: '/', notFound: '/404')", ""),
        ROUTER.replace(
            "import 'package:dust_flutter/route.dart';",
            "import 'package:dust_flutter/route.dart'; import 'foreign.dart';",
        ),
        ROUTER.replace(
            "import 'package:dust_flutter/route.dart';",
            "import 'package:dust_flutter/route.dart' if (dart.library.io) 'foreign.dart';",
        ),
        ROUTER
            .replace("@AppRouter(initial: '/', notFound: '/404')", "")
            .replace(
                "const Router();",
                "@AppRouter(initial: '/', notFound: '/404') const Router();",
            ),
        ROUTER
            .replace("@AppRouter(initial: '/', notFound: '/404')", "")
            .replace(
                "const Router();",
                "const Router(); @AppRouter(initial: '/') void ignored() {}",
            ),
        format!(
            "{}\nclass AppRouter {{ const AppRouter({{String? initial, String? notFound}}); }}",
            ROUTER
        ),
        ROUTER.replace(
            "import 'package:dust_flutter/route.dart';",
            "import 'package:dust_flutter/route.dart' hide AppRouter;",
        ),
        ROUTER
            .replace(
                "import 'package:dust_flutter/route.dart';",
                "import 'package:dust_flutter/route.dart' deferred as ann;",
            )
            .replace("@AppRouter", "@ann.AppRouter")
            .replace("@AppRoute", "@ann.AppRoute"),
        ROUTER.replace("route/routes.g.dart", "missing.g.dart"),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("lib/foreign.dart"),
            "class AppRouter { const AppRouter({String? initial, String? notFound}); }",
        );
        write_file(&root.join("lib/route.dart"), &source);
        for result in [build(root), check(root)] {
            assert!(result.has_errors(), "{source}: {:?}", result.diagnostics);
            assert!(!root.join("lib/route/routes.g.dart").exists());
            assert!(!root.join("lib/missing.g.dart").exists());
            assert!(!root.join(".dart_tool/dust/build_cache_v1.json").exists());
        }
    }
}

#[test]
fn marked_generated_annotations_cannot_supply_the_producer() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/route.dart"),
        &ROUTER.replace(
            "import 'package:dust_flutter/route.dart';",
            "import 'package:dust_flutter/route.dart' hide AppRouter;",
        ),
    );
    let generated = "// GENERATED CODE - DO NOT MODIFY BY HAND.\n// Generated by dust.\nclass AppRouter { const AppRouter({String? initial, String? notFound}); }";
    write_file(&root.join("lib/route/routes.g.dart"), generated);
    let result = build(root);
    assert!(result.has_errors());
    assert_eq!(
        fs::read_to_string(root.join("lib/route/routes.g.dart")).unwrap(),
        generated
    );
    assert!(!root.join("lib/route/runtime.g.dart").exists());
}

#[test]
fn focused_db_mode_can_load_route_namespace_metadata_without_emitting_routes() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(&root.join("lib/route.dart"), ROUTER);
    let result = run_build(BuildRequest {
        sdk_source_root: Some(sdk_source_root(root)),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: DbRequestOptions {
            only_db: true,
            ..Default::default()
        },
    });
    assert_eq!(result.diagnostics, []);
    assert!(!root.join("lib/route").exists());
}

#[test]
fn final_graph_rejects_a_producer_that_depended_on_an_unverified_placeholder() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/route.dart"),
        &format!("import 'other/routes.g.dart';\n{ROUTER}"),
    );
    write_file(
        &root.join("lib/other.dart"),
        "import 'other/routes.g.dart';\n@AppRouter() class Other {}\nclass AppRouter { const AppRouter(); }",
    );
    for result in [build(root), check(root)] {
        assert!(result.has_errors());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|error| error.message.contains("route/routes.g.dart")
                    && error
                        .message
                        .contains("no verified handwritten class producer")),
            "{:?}",
            result.diagnostics
        );
        assert!(!root.join("lib/route").exists());
        assert!(!root.join("lib/other").exists());
        assert!(!root.join(".dart_tool/dust/build_cache_v1.json").exists());
    }
}
