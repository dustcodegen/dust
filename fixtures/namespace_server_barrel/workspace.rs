//! Actual Dust runtime sources paired with the existing hermetic namespace fixture.

use std::{fs, path::Path};

/// Replace only owned dependencies; external packages/SDK stay source-only controls.
pub(crate) fn prepare(root: &Path) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["dust_dart", "dust_server"] {
        copy_sources(
            &repo.join(format!("packages/{name}/lib")),
            &root.join(format!("deps/{name}/lib")),
        );
        fs::copy(
            repo.join(format!("packages/{name}/pubspec.yaml")),
            root.join(format!("deps/{name}/pubspec.yaml")),
        )
        .unwrap();
    }
    let config = root.join(".dart_tool/package_config.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    value["packages"].as_array_mut().unwrap().push(serde_json::json!({
        "name":"dust_server", "rootUri":"../deps/dust_server", "packageUri":"lib/", "languageVersion":"3.6"
    }));
    fs::write(config, value.to_string()).unwrap();
}

/// Never resolve source files through the host pub cache or follow directory links.
fn copy_sources(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_sources(&entry.path(), &target);
        } else if entry.file_type().unwrap().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|suffix| suffix == "dart")
        {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
