use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};
use dust_plugin_api::GENERATED_HEADER;

use super::support::{make_pub_workspace_member, sdk_source_root, write_file};

#[test]
fn builds_and_checks_standalone_clients_in_a_sibling_package() {
    let (workspace, server) = make_pub_workspace_member();
    let client = workspace.path().join("packages/todo_client");
    write_file(
        &workspace.path().join("pubspec.yaml"),
        "name: dust_workspace\nworkspace:\n  - examples/product_showcase\n  - packages/todo_client\n",
    );
    write_file(
        &client.join("pubspec.yaml"),
        "name: todo_client\nresolution: workspace\ndependencies:\n  dio: any\n",
    );
    fs::create_dir_all(client.join("lib")).unwrap();
    let config_path = workspace.path().join(".dart_tool/package_config.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    config["packages"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "dust_server",
            "rootUri": repository.join("packages/dust_server").to_string_lossy(),
            "packageUri": "lib/",
            "languageVersion": "3.6"
        }));
    config["packages"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "todo_client",
            "rootUri": "../packages/todo_client",
            "packageUri": "lib/",
            "languageVersion": "3.6"
        }));
    write_file(&config_path, &config.to_string());
    write_file(
        &workspace.path().join("deps/meta/lib/meta_meta.dart"),
        "final class Target { const Target(Set<TargetKind> kinds); }\n\
         enum TargetKind { classType, library, method, function, parameter }\n",
    );
    write_file(
        &server.join("dust.yaml"),
        "server:\n  client_package: todo_client\n",
    );
    write_file(
        &server.join("lib/todos.dart"),
        r#"import 'package:dust_server/annotations.dart';
part 'todos.g.dart';

@Controller('/todos')
class Todos with _$Todos {
  @GET('/{id}')
  Future<String> read(@Path() String id, {@Query('q') String? q}) async => id;

  @POST('/')
  Future<String> create(@Body() Map<String, Object?> body) async => 'ok';

  @DELETE('/{id}')
  Future<String> delete(@Path() String id) async => id;
}

"#,
    );
    let request = BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };

    let built = run_build(request.clone());

    assert!(!built.has_errors(), "{:?}", built.diagnostics);
    let barrel = client.join("lib/src/generated/todos_client.g.dart");
    let output = client.join("lib/src/generated/todos_client.todos.g.dart");
    assert_eq!(
        fs::read_to_string(&barrel).unwrap(),
        format!("{GENERATED_HEADER}\nexport 'todos_client.todos.g.dart';\n")
    );
    let source = fs::read_to_string(&output).unwrap();
    assert!(source.starts_with(GENERATED_HEADER));
    assert!(source.contains("final class TodosClient"));
    assert!(source.contains("Future<String> read("));
    assert!(source.contains("method: 'POST'"));
    assert!(source.contains("method: 'DELETE'"));
    let checked = run_check(CheckRequest {
        sdk_source_root: request.sdk_source_root.clone(),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);

    let server_source = server.join("lib/todos.dart");
    let renamed = fs::read_to_string(&server_source)
        .unwrap()
        .replace("class Todos", "class Tasks")
        .replace("_$Todos", "_$Tasks");
    write_file(&server_source, &renamed);
    let rebuilt = run_build(request.clone());
    assert!(!rebuilt.has_errors(), "{:?}", rebuilt.diagnostics);
    assert!(!output.exists());
    let output = client.join("lib/src/generated/todos_client.tasks.g.dart");
    assert!(output.exists());

    write_file(&output, "stale\n");
    let stale = run_check(CheckRequest {
        sdk_source_root: request.sdk_source_root.clone(),
        cwd: server,
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert!(!stale.has_errors(), "{:?}", stale.diagnostics);
    assert_eq!(stale.checked_libraries.len(), 1);
    assert!(stale.checked_libraries[0].stale);
    let mut actual = stale.checked_libraries[0].auxiliary_output_paths.clone();
    actual.sort();
    let mut expected = vec![barrel, output];
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn shares_serde_query_models_across_server_and_client_packages() {
    let (workspace, server) = make_pub_workspace_member();
    let client = workspace.path().join("packages/todo_client");
    write_file(
        &workspace.path().join("pubspec.yaml"),
        "name: dust_workspace\nworkspace:\n  - examples/product_showcase\n  - packages/todo_client\n",
    );
    write_file(
        &client.join("pubspec.yaml"),
        "name: todo_client\nresolution: workspace\ndependencies:\n  dio: any\n  dust_dart: any\n",
    );
    let contracts = client.join("lib/contracts.dart");
    write_file(
        &contracts,
        r#"import 'package:dust_dart/serde.dart';
part 'contracts.g.dart';

@Derive([Serialize(), Deserialize()])
final class TodoFilter with _$TodoFilter {
  const TodoFilter({required this.owner, this.page = 1, this.tags = const []});
  final String owner;
  @SerDe(defaultValue: 1)
  final int page;
  @SerDe(defaultValue: <String>[])
  final List<String> tags;
}
"#,
    );
    let config_path = workspace.path().join(".dart_tool/package_config.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for (name, root) in [
        ("dust_server", repository.join("packages/dust_server")),
        ("todo_client", client.clone()),
    ] {
        config["packages"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": name,
                "rootUri": root.to_string_lossy(),
                "packageUri": "lib/",
                "languageVersion": "3.6"
            }));
    }
    write_file(&config_path, &config.to_string());
    write_file(
        &workspace.path().join("deps/meta/lib/meta_meta.dart"),
        "final class Target { const Target(Set<TargetKind> kinds); }\n\
         enum TargetKind { classType, library, method, function, parameter }\n",
    );
    write_file(
        &server.join("dust.yaml"),
        "server:\n  client_package: todo_client\n",
    );
    write_file(
        &server.join("lib/todos.dart"),
        r#"import 'package:dust_server/annotations.dart';
import 'package:todo_client/contracts.dart' as contracts;
part 'todos.g.dart';

@Controller('/todos')
class Todos with _$Todos {
  @GET('/search')
  Future<String> search(@Query() contracts.TodoFilter filter) async => filter.owner;
}
"#,
    );

    let built = run_build(BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });

    assert!(!built.has_errors(), "{:?}", built.diagnostics);
    let server_source = fs::read_to_string(server.join("lib/todos.g.dart")).unwrap();
    assert!(server_source.contains("query.coerceOne<int>(values, 'page');"));
    assert!(server_source.contains("query.copyAll(values, 'tags');"));
    assert!(server_source.contains("contracts.$TodoFilterDeserializer"));
    let client_source =
        fs::read_to_string(client.join("lib/src/generated/todos_client.todos.g.dart")).unwrap();
    assert!(client_source.contains("Future<String> search(contracts.TodoFilter filter)"));
    assert!(client_source.contains("filter.serialize()..removeWhere"));

    let deserialize_only = fs::read_to_string(&contracts)
        .unwrap()
        .replace("Serialize(), ", "");
    write_file(&contracts, &deserialize_only);
    let rebuilt = run_build(BuildRequest {
        sdk_source_root: Some(sdk_source_root(&server)),
        cwd: server,
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert!(
        rebuilt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains(
                "generated clients require query model `contracts.TodoFilter` to derive Serialize()"
            ))
    );
}
