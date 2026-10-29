use dust_workspace::{ServerClientConfig, ServerConfig, load_dust_config};
use tempfile::tempdir;

use crate::support::write_file;

#[test]
fn reads_one_server_client_package() {
    let root = tempdir().unwrap();
    write_file(&root.path().join("pubspec.yaml"), "name: dust_test\n");
    write_file(
        &root.path().join("dust.yaml"),
        "server:\n  client_package: todo_client\n",
    );

    let config = load_dust_config(root.path()).unwrap();

    assert_eq!(
        config.server,
        Some(ServerConfig {
            clients: vec![ServerClientConfig {
                package: "todo_client".to_owned(),
                tags: Vec::new(),
            }],
        })
    );
}

#[test]
fn reads_tag_split_client_packages_in_name_order() {
    let root = tempdir().unwrap();
    write_file(
        &root.path().join("dust.yaml"),
        "server:\n  client_packages:\n    public_client: [public, mobile]\n    admin_client: [admin]\n",
    );

    let config = load_dust_config(root.path()).unwrap();

    assert_eq!(
        config.server.unwrap().clients,
        vec![
            ServerClientConfig {
                package: "admin_client".to_owned(),
                tags: vec!["admin".to_owned()],
            },
            ServerClientConfig {
                package: "public_client".to_owned(),
                tags: vec!["public".to_owned(), "mobile".to_owned()],
            },
        ]
    );
}

#[test]
fn rejects_invalid_server_client_package_names() {
    let root = tempdir().unwrap();
    write_file(&root.path().join("pubspec.yaml"), "name: dust_test\n");

    for value in ["", "TodoClient", "../todo_client", "todo-client"] {
        write_file(
            &root.path().join("dust.yaml"),
            &format!("server:\n  client_package: '{value}'\n"),
        );
        let error = load_dust_config(root.path()).unwrap_err();
        assert!(error.message.contains("server.client_package"), "{value}");
    }
}

#[test]
fn rejects_missing_ambiguous_and_empty_server_clients() {
    let root = tempdir().unwrap();
    for source in [
        "server: {}\n",
        "server:\n  client_package: app_client\n  client_packages:\n    admin_client: [admin]\n",
        "server:\n  client_packages: {}\n",
        "server:\n  client_packages:\n    admin_client: []\n",
        "server:\n  client_packages:\n    admin_client: ['']\n",
    ] {
        write_file(&root.path().join("dust.yaml"), source);
        assert!(load_dust_config(root.path()).is_err(), "{source}");
    }
}
