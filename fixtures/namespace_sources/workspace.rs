//! Source-only namespace fixtures; real consumer compilation uses real SDKs/packages.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const DECLARATIONS: &[(&str, &str)] = &[
    (
        "dust_dart/src/derive/base.dart",
        include_str!("../../packages/dust_dart/lib/src/derive/base.dart"),
    ),
    (
        "dust_dart/src/derive/traits.dart",
        include_str!("../../packages/dust_dart/lib/src/derive/traits.dart"),
    ),
    (
        "dust_dart/src/derive/validation.dart",
        include_str!("../../packages/dust_dart/lib/src/derive/validation.dart"),
    ),
    (
        "dust_dart/src/serde/serde.dart",
        include_str!("../../packages/dust_dart/lib/src/serde/serde.dart"),
    ),
    (
        "dust_dart/src/http/http_client_annotations.dart",
        include_str!("../../packages/dust_dart/lib/src/http/http_client_annotations.dart"),
    ),
    (
        "dust_dart/src/db/annotations.dart",
        include_str!("../../packages/dust_dart/lib/src/db/annotations.dart"),
    ),
    (
        "dust_flutter/src/router/route_annotations.dart",
        include_str!("../../packages/dust_flutter/lib/src/router/route_annotations.dart"),
    ),
    (
        "dust_flutter/src/state/annotations.dart",
        include_str!("../../packages/dust_flutter/lib/src/state/annotations.dart"),
    ),
];

const BARRELS: &[(&str, &str)] = &[
    (
        "dust_dart/derive.dart",
        "export 'src/derive/base.dart'; export 'src/derive/traits.dart'; export 'src/derive/validation.dart';",
    ),
    (
        "dust_dart/serde.dart",
        "export 'derive.dart'; export 'src/serde/serde.dart';",
    ),
    (
        "dust_dart/http.dart",
        "export 'serde.dart'; export 'src/http/http_client_annotations.dart';",
    ),
    (
        "dust_dart/db.dart",
        "export 'derive.dart'; export 'src/db/annotations.dart';",
    ),
    (
        "dust_flutter/route.dart",
        "export 'src/router/route_annotations.dart';",
    ),
    (
        "dust_flutter/state.dart",
        "export 'src/state/annotations.dart';",
    ),
    ("dust_flutter/i18n.dart", "library dust_i18n;"),
    (
        "dust_db_sqlite3/dust_db_sqlite3.dart",
        "export 'package:dust_dart/db.dart';",
    ),
    (
        "dust_db_postgres/dust_db_postgres.dart",
        "export 'package:dust_dart/db.dart';",
    ),
    ("flutter/material.dart", "export 'widgets.dart';"),
    (
        "flutter/widgets.dart",
        "class Widget {} class BuildContext {} const immutable=Object();",
    ),
    (
        "flutter_localizations/flutter_localizations.dart",
        "library flutter_localizations;",
    ),
    ("collection/collection.dart", "library collection;"),
    (
        "meta/meta.dart",
        "const immutable=Object(); const protected=Object();",
    ),
    ("dio/dio.dart", "library dio;"),
];

pub(crate) fn prepare(root: &Path, package_root: &Path, package_name: &str) {
    let mut packages = BTreeMap::new();
    for (uri, text) in DECLARATIONS.iter().chain(BARRELS) {
        let (package, path) = uri.split_once('/').unwrap();
        write(&root.join(format!("deps/{package}/lib/{path}")), text);
        packages.insert(
            package,
            serde_json::json!({"name":package,
            "rootUri":format!("../deps/{package}"),"packageUri":"lib/","languageVersion":"3.6"}),
        );
    }
    for name in [
        "dust_dart",
        "dust_flutter",
        "dust_db_sqlite3",
        "dust_db_postgres",
    ] {
        write(
            &root.join(format!("deps/{name}/pubspec.yaml")),
            &format!("name: {name}\nversion: 0.2.1\n"),
        );
    }
    let relative = package_root
        .strip_prefix(root)
        .unwrap()
        .to_str()
        .unwrap()
        .replace('\\', "/");
    let mut entries = packages.into_values().collect::<Vec<_>>();
    entries.push(serde_json::json!({"name":package_name,"rootUri":format!("../{relative}"),"packageUri":"lib/"}));
    write(
        &root.join(".dart_tool/package_config.json"),
        &serde_json::json!({"configVersion":2,"packages":entries}).to_string(),
    );
    let sdk = root.join(".dust-test-sdk");
    write(&sdk.join("version"), "3.13.5\n");
    write(
        &sdk.join("lib/libraries.json"),
        r#"{"vm":{"libraries":{
      "core":{"uri":"core/core.dart"},"async":{"uri":"async/async.dart"},
      "isolate":{"uri":"isolate/isolate.dart"}}}}"#,
    );
    write(
        &sdk.join("lib/core/core.dart"),
        "library dart.core; class Object {} class String {} class Deprecated { const Deprecated(String message); } const override=Object(); const deprecated=Object();",
    );
    write(
        &sdk.join("lib/async/async.dart"),
        "library dart.async; class Future<T> {} class Stream<T> {} void unawaited(Object? value) {}",
    );
    write(
        &sdk.join("lib/isolate/isolate.dart"),
        "library dart.isolate; class Isolate {}",
    );
    let command = sdk.join(if cfg!(windows) {
        "bin/dart.exe"
    } else {
        "bin/dart"
    });
    write(
        &command,
        "SDK discovery fixture must never execute this file\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(command, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

pub(crate) fn sdk_source_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .map(|root| root.join(".dust-test-sdk"))
        .find(|root| root.join("version").is_file())
        .unwrap_or_else(|| cwd.join(".dust-test-sdk"))
}

pub(crate) fn set_language_version(root: &Path, name: &str, version: &str) {
    let path = root
        .ancestors()
        .map(|root| root.join(".dart_tool/package_config.json"))
        .find(|path| path.is_file())
        .unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let entry = value["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|package| package["name"] == name)
        .unwrap();
    entry["languageVersion"] = version.into();
    write(&path, &value.to_string());
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
