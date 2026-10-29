use std::{fs, path::Path};

use dust_workspace::{
    DustConfig, OutputConfig, PackageConfig, PackageConfigKind, ServerClientConfig, ServerConfig,
    WorkspacePlan,
};
use tempfile::TempDir;

use super::resolve_destinations;

fn write(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, source).unwrap();
}

fn workspace(config: &str) -> (TempDir, WorkspacePlan) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let server = root.join("server");
    let client = root.join("client");
    write(&server.join("pubspec.yaml"), "name: server\n");
    write(&client.join("pubspec.yaml"), "name: client\n");
    fs::create_dir_all(client.join("lib")).unwrap();
    let package_config = root.join(".dart_tool/package_config.json");
    write(&package_config, config);
    let plan = WorkspacePlan {
        package_root: server.clone(),
        cache_root: server,
        package_name: "server".to_owned(),
        is_flutter_package: false,
        is_pub_workspace_root: false,
        dart_sdk_lower_bound: None,
        package_config: PackageConfig {
            path: package_config,
            kind: PackageConfigKind::WorkspaceShared {
                package_graph_path: root.join("server/.dart_tool/package_graph.json"),
            },
        },
        dust_config: DustConfig {
            path: Some(root.join("server/dust.yaml")),
            outputs: OutputConfig::default(),
            i18n: None,
            server: Some(ServerConfig {
                clients: vec![ServerClientConfig {
                    package: "client".to_owned(),
                    tags: Vec::new(),
                }],
            }),
        },
        libraries: Vec::new(),
        dust_packages: Vec::new(),
    };
    (temp, plan)
}

#[test]
fn resolves_separate_package_inside_shared_workspace() {
    let (temp, plan) = workspace(
        r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client","packageUri":"lib/"}]}"#,
    );

    let destination = resolve_destinations(&plan).unwrap().remove(0);

    assert_eq!(destination.package_name, "client");
    assert_eq!(destination.package_root, temp.path().join("client"));
}

#[test]
fn resolves_tagged_destinations_in_config_order() {
    let (temp, mut plan) = workspace(
        r#"{"configVersion":2,"packages":[{"name":"admin_client","rootUri":"../admin_client"},{"name":"client","rootUri":"../client"}]}"#,
    );
    let admin = temp.path().join("admin_client");
    write(&admin.join("pubspec.yaml"), "name: admin_client\n");
    fs::create_dir_all(admin.join("lib")).unwrap();
    plan.dust_config.server = Some(ServerConfig {
        clients: vec![
            ServerClientConfig {
                package: "admin_client".to_owned(),
                tags: vec!["admin".to_owned()],
            },
            ServerClientConfig {
                package: "client".to_owned(),
                tags: vec!["public".to_owned()],
            },
        ],
    });

    let destinations = resolve_destinations(&plan).unwrap();

    assert_eq!(destinations[0].package_name, "admin_client");
    assert_eq!(destinations[0].tags, ["admin"]);
    assert_eq!(destinations[1].package_name, "client");
    assert_eq!(destinations[1].tags, ["public"]);
}

#[test]
fn disabled_config_has_no_destination() {
    let (_temp, mut plan) = workspace(r#"{"configVersion":2,"packages":[]}"#);
    plan.dust_config.server = None;

    assert!(resolve_destinations(&plan).unwrap().is_empty());
}

#[test]
fn rejects_missing_and_ambiguous_packages() {
    for (config, expected) in [
        (r#"{"configVersion":2,"packages":[]}"#, "is not resolved"),
        (
            r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client"},{"name":"client","rootUri":"../client"}]}"#,
            "is ambiguous",
        ),
    ] {
        let (_temp, plan) = workspace(config);
        let error = resolve_destinations(&plan).unwrap_err();
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn rejects_non_workspace_and_external_destinations() {
    let (_temp, mut direct) =
        workspace(r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client"}]}"#);
    direct.package_config.kind = PackageConfigKind::Direct;
    assert!(
        resolve_destinations(&direct)
            .unwrap_err()
            .message
            .contains("requires a local Dart pub workspace")
    );

    let (_temp, external) = workspace(
        r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../../external"}]}"#,
    );
    assert!(
        resolve_destinations(&external)
            .unwrap_err()
            .message
            .contains("outside the active Dart workspace")
    );
}

#[cfg(unix)]
#[test]
fn rejects_workspace_symlink_to_external_destination() {
    use std::os::unix::fs::symlink;

    let (temp, plan) =
        workspace(r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client"}]}"#);
    let external = tempfile::tempdir().unwrap();
    write(&external.path().join("pubspec.yaml"), "name: client\n");
    fs::create_dir_all(external.path().join("lib")).unwrap();
    fs::remove_dir_all(temp.path().join("client")).unwrap();
    symlink(external.path(), temp.path().join("client")).unwrap();

    let error = resolve_destinations(&plan).unwrap_err();

    assert!(
        error
            .message
            .contains("resolves outside the active Dart workspace"),
        "{}",
        error.message
    );
}

#[test]
fn rejects_same_missing_lib_and_wrong_pubspec_destinations() {
    let (_temp, mut same) =
        workspace(r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../server"}]}"#);
    same.package_name = "client".to_owned();
    assert!(
        resolve_destinations(&same)
            .unwrap_err()
            .message
            .contains("separate client package")
    );

    let (temp, no_lib) =
        workspace(r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client"}]}"#);
    fs::remove_dir(temp.path().join("client/lib")).unwrap();
    assert!(
        resolve_destinations(&no_lib)
            .unwrap_err()
            .message
            .contains("has no `lib` directory")
    );

    let (temp, wrong_name) =
        workspace(r#"{"configVersion":2,"packages":[{"name":"client","rootUri":"../client"}]}"#);
    write(&temp.path().join("client/pubspec.yaml"), "name: other\n");
    assert!(
        resolve_destinations(&wrong_name)
            .unwrap_err()
            .message
            .contains("resolves to package `other`")
    );
}

#[test]
fn rejects_malformed_package_entries() {
    for (config, expected) in [
        ("{}", "packages list"),
        (r#"{"packages":[{"name":"client"}]}"#, "missing rootUri"),
        (
            r#"{"packages":[{"name":"client","rootUri":"https://example.invalid/client"}]}"#,
            "invalid client package rootUri",
        ),
    ] {
        let (_temp, plan) = workspace(config);
        let error = resolve_destinations(&plan).unwrap_err();
        assert!(error.message.contains(expected), "{}", error.message);
    }
}
