use super::*;

#[test]
#[cfg(unix)]
fn new_snapshots_are_private_and_replacements_preserve_existing_mode() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new(MODEL);
    let control = tempfile::NamedTempFile::new_in(fixture.root.path()).unwrap();
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&fixture.path()), mode(control.path()));
    assert_eq!(mode(&fixture.path()) & 0o077, 0);

    fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o660)).unwrap();
    fixture.save();
    assert_eq!(mode(&fixture.path()), 0o660);
    assert!(fixture.lookup().is_some());
}

#[test]
fn a_nonregular_snapshot_is_a_miss_and_failed_replacement_preserves_it() {
    let fixture = Fixture::new(MODEL);
    fs::remove_file(fixture.path()).unwrap();
    fs::create_dir(fixture.path()).unwrap();
    assert!(fixture.lookup().is_none());
    let sources = namespaces(fixture.root.path());
    assert!(save(&sources, fixture.root.path(), &fixture.annotations).is_err());
    assert!(fixture.path().is_dir());
    assert_eq!(fs::read_to_string(&fixture.source.path).unwrap(), MODEL);
}

#[test]
#[cfg(unix)]
fn snapshot_symlinks_miss_and_atomic_save_replaces_only_the_cache_entry() {
    let fixture = Fixture::new(MODEL);
    let target = fixture.root.path().join("handwritten.dart");
    let handwritten = "class Handwritten {}\n";
    fs::write(&target, handwritten).unwrap();
    fs::remove_file(fixture.path()).unwrap();
    std::os::unix::fs::symlink(&target, fixture.path()).unwrap();
    assert!(fixture.lookup().is_none());
    fixture.save();
    assert_eq!(fs::read_to_string(&target).unwrap(), handwritten);
    assert!(
        !fs::symlink_metadata(fixture.path())
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(fixture.lookup().is_some());
}

#[test]
#[cfg(unix)]
fn a_failed_temporary_write_keeps_the_last_complete_snapshot_usable() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(MODEL);
    let before = fs::read(fixture.path()).unwrap();
    let sources = namespaces(fixture.root.path());
    let directory = fixture.path().parent().unwrap().to_path_buf();
    let permissions = fs::metadata(&directory).unwrap().permissions();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap();
    let result = save(&sources, fixture.root.path(), &fixture.annotations);
    fs::set_permissions(directory, permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(fixture.path()).unwrap(), before);
    assert!(fixture.lookup().is_some());
}

#[test]
fn clean_removes_parser_facts_with_the_existing_dust_cache_storage() {
    let fixture = Fixture::new(MODEL);
    assert!(fixture.lookup().is_some());
    let result = crate::run_clean(crate::CleanRequest {
        cwd: fixture.root.path().to_path_buf(),
    });
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(result.clean.unwrap().cache_cleared);
    assert!(!fixture.path().exists());
    assert!(fixture.lookup().is_none());
    assert_eq!(fs::read_to_string(&fixture.source.path).unwrap(), MODEL);
}
