use std::fs;

use dust_cache::WorkspaceCache;
use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::{
    server_client_outputs::client_workspace,
    support::{sdk_source_root, write_file},
};

const HEALTH_CLIENT: &str = r#"import 'package:dust_server/annotations.dart';
part 'api.g.dart';

@Controller('/index')
class HealthClient with _$HealthClient {
  @GET('/')
  Future<String> index() async => 'index';
}
"#;
const OTHER_ROUTES: &str = r#"@Routes('/api', name: 'otherRoutes')
library;
import 'package:dust_server/annotations.dart';
part 'api_client.health.g.dart';

@GET('/health')
Future<String> health() async => 'ok';
"#;

#[test]
fn rejects_client_path_collisions_without_replacing_outputs() {
    let (_workspace, server, client) = client_workspace();
    let source = server.join("lib/api.dart");
    let module = r#"@Routes('/health')
library;
import 'package:dust_server/annotations.dart';
part 'api.g.dart';

@GET('/')
Future<String> health() async => 'ok';
"#;
    let routes_controller = r#"
@Controller('/index')
class Routes with _$Routes {
  @GET('/')
  Future<String> index() async => 'index';
}
"#;
    let build = BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };
    let check = CheckRequest {
        sdk_source_root: build.sdk_source_root.clone(),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };
    let colliding = format!("{module}{routes_controller}");
    write_file(&source, &colliding);
    let cold = run_build(build.clone());
    let diagnostics = cold
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("api_client.routes.g.dart"))
        .expect("cold build reports the collision");
    assert_eq!(diagnostics.labels.len(), 2);
    assert!(!client.exists());
    assert!(!server.join("lib/api.g.dart").exists());

    write_file(&source, module);
    let valid = run_build(build.clone());
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics);
    let outputs = [
        client.join("api_client.g.dart"),
        client.join("api_client.routes.g.dart"),
        server.join("lib/api.g.dart"),
    ];
    let original = outputs.each_ref().map(|path| fs::read(path).unwrap());

    write_file(&source, &colliding);
    assert!(run_check(check).has_errors());
    assert!(run_build(build).has_errors());
    assert_eq!(outputs.map(|path| fs::read(path).unwrap()), original);
}

#[test]
fn rejects_colliding_outputs_from_distinct_server_libraries() {
    let (_workspace, server, client) = client_workspace();
    write_file(&server.join("lib/api.dart"), HEALTH_CLIENT);
    let build = BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };
    let valid = run_build(build.clone());
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics);
    let output = client.join("api_client.health_client.g.dart");
    let original = fs::read(&output).unwrap();

    write_file(&server.join("lib/api_client.health.dart"), OTHER_ROUTES);
    let invalid = run_build(build);
    let diagnostics = invalid
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .message
                .contains("api_client.health_client.g.dart")
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{:?}", invalid.diagnostics);
    assert_ne!(
        diagnostics[0].labels[0].file_id,
        diagnostics[1].labels[0].file_id
    );
    assert_eq!(fs::read(output).unwrap(), original);
    assert!(!server.join("lib/api_client.health.g.dart").exists());
}

#[test]
fn preserves_live_client_output_when_a_former_owner_is_rebuilt() {
    let (_workspace, server, client) = client_workspace();
    let former_owner = server.join("lib/api.dart");
    let current_owner = server.join("lib/api_client.health.dart");
    let shared_path = client.join("api_client.health_client.g.dart");
    let build = BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server.clone(),
        fail_fast: false,
        jobs: Some(1),
        db: Default::default(),
    };
    write_file(&former_owner, HEALTH_CLIENT);
    let original = run_build(build.clone());
    assert!(!original.has_errors(), "{:?}", original.diagnostics);
    let cache = WorkspaceCache::load(&server).unwrap();
    let original_entry = cache.get(&server, &former_owner).unwrap().clone();

    write_file(
        &former_owner,
        &HEALTH_CLIENT.replace("HealthClient", "IndexClient"),
    );
    write_file(&current_owner, OTHER_ROUTES);
    let transferred = run_build(build.clone());
    assert!(!transferred.has_errors(), "{:?}", transferred.diagnostics);
    let expected = fs::read(&shared_path).unwrap();

    // Keep the new owner's outputs current while the former owner's old
    // cache record still lists the path, as after a partially saved build.
    let restore_former_entry = || {
        let mut cache = WorkspaceCache::load(&server).unwrap();
        cache.insert(&server, &former_owner, original_entry.clone());
        cache.flush().unwrap();
    };
    restore_former_entry();
    let checked = run_check(CheckRequest {
        sdk_source_root: build.sdk_source_root.clone(),
        cwd: server.clone(),
        fail_fast: false,
        jobs: Some(1),
        db: Default::default(),
    });
    restore_former_entry();
    let rebuilt = run_build(build);
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);
    assert!(!rebuilt.has_errors(), "{:?}", rebuilt.diagnostics);
    assert!(
        rebuilt
            .build_artifacts
            .iter()
            .any(|artifact| artifact.source_path == current_owner && artifact.cached)
    );
    assert_eq!(fs::read(&shared_path).unwrap(), expected);
    assert!(
        checked
            .checked_libraries
            .iter()
            .all(|library| !library.stale)
    );
}
