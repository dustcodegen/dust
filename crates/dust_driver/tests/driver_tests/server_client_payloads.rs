use std::{fs, path::PathBuf};

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};
use tempfile::TempDir;

use super::support::{make_pub_workspace_member, sdk_source_root, write_file};

fn workspace() -> (TempDir, PathBuf, PathBuf) {
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
    (workspace, server, client)
}

fn request(server: &std::path::Path) -> BuildRequest {
    BuildRequest {
        sdk_source_root: Some(sdk_source_root(server)),
        cwd: server.to_owned(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    }
}

#[test]
fn generates_shared_serde_request_and_response_payloads() {
    let (_workspace, server, client) = workspace();
    write_file(
        &client.join("lib/contracts.dart"),
        r#"import 'package:dust_dart/serde.dart';
part 'contracts.g.dart';

@Derive([Serialize(), Deserialize(), Validate()])
final class CreateTodo with _$CreateTodo {
  const CreateTodo(this.title);
  @Validate(length: Length(min: 1))
  final String title;
}

@Derive([Serialize(), Deserialize()])
final class Todo with _$Todo {
  const Todo(this.title);
  final String title;
}
"#,
    );
    write_file(
        &server.join("lib/todos.dart"),
        r#"import 'package:dust_server/annotations.dart';
import 'package:todo_client/contracts.dart' as contracts;
part 'todos.g.dart';

@Controller('/todos')
class Todos with _$Todos {
  @POST('/')
  Future<contracts.Todo> create(@Body() contracts.CreateTodo input) async =>
      contracts.Todo(input.title);

  @GET('/')
  Future<List<contracts.Todo>> list() async => const [];

  @GET('/{id}')
  Future<contracts.Todo?> read(@Path() String id) async => null;
}
"#,
    );

    let request = request(&server);
    let built = run_build(request.clone());
    assert!(!built.has_errors(), "{:?}", built.diagnostics);
    let server_source = fs::read_to_string(server.join("lib/todos.g.dart")).unwrap();
    assert!(server_source.contains("contracts.$CreateTodoDeserializer"));
    assert!(server_source.contains("ValidatedExtractable<contracts.CreateTodo>"));
    assert!(!server_source.contains("const ValidatedExtractable<contracts.CreateTodo>"));

    let output = client.join("lib/src/generated/todos_client.todos.g.dart");
    let source = fs::read_to_string(&output).unwrap();
    assert!(source.contains("input.serialize()"));
    assert!(source.contains("const contracts.$TodoDeserializer().deserialize"));
    assert!(source.contains("Future<List<contracts.Todo>> list()"));
    assert!(source.contains("Future<contracts.Todo?> read(String id)"));

    let checked = run_check(CheckRequest {
        sdk_source_root: request.sdk_source_root.clone(),
        cwd: server.clone(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);

    let contracts = client.join("lib/contracts.dart");
    let deserialize_only = fs::read_to_string(&contracts)
        .unwrap()
        .replace("Serialize(), ", "");
    write_file(&contracts, &deserialize_only);
    let rebuilt = run_build(request);
    assert!(rebuilt.diagnostics.iter().any(|value| {
        value
            .message
            .contains("body model `contracts.CreateTodo` to derive Serialize()")
    }));
    assert_eq!(fs::read_to_string(output).unwrap(), source);
}

#[test]
fn rejects_known_payload_models_without_required_capabilities() {
    let (_workspace, server, client) = workspace();
    write_file(
        &client.join("lib/contracts.dart"),
        r#"import 'package:dust_dart/serde.dart';
part 'contracts.g.dart';

@Derive([Serialize()])
final class Sentinel with _$Sentinel {
  const Sentinel();
}

final class Payload {
  const Payload(this.value);
  factory Payload.fromJson(Map<String, Object?> json) =>
      Payload(json['value']! as String);

  static Payload deserialize(Map<String, Object?> json) =>
      Payload.fromJson(json);

  final String value;

  Map<String, Object?> toJson() => {'value': value};
}
"#,
    );
    write_file(
        &server.join("lib/todos.dart"),
        r#"import 'package:dust_server/annotations.dart';
import 'package:todo_client/contracts.dart' as contracts;
part 'todos.g.dart';

@Controller('/todos')
class Todos with _$Todos {
  @POST('/')
  Future<contracts.Payload> create(@Body() contracts.Payload input) async => input;
}
"#,
    );

    let built = run_build(request(&server));
    let messages = built
        .diagnostics
        .iter()
        .map(|value| value.message.as_str())
        .collect::<Vec<_>>();
    assert!(built.diagnostics.iter().all(|value| value.has_labels()));
    for expected in [
        "body model `contracts.Payload` must derive Deserialize()",
        "body model `contracts.Payload` to derive Serialize()",
        "response model `contracts.Payload` to derive Serialize()",
        "response model `contracts.Payload` to derive Deserialize()",
    ] {
        assert!(
            messages.iter().any(|value| value.contains(expected)),
            "{messages:?}"
        );
    }
}
