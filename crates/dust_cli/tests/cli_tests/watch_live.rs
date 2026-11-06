use std::{
    fs::{self, File},
    path::Path,
    process::{Child, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::helpers::{cli_command, make_workspace, run_cli, write_file};

/// Stops the unbounded watcher even when an assertion fails.
struct WatchChild(Child);

impl Drop for WatchChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Waits for a build or rebuild result to become observable.
fn wait_until(reason: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for {reason}");
        thread::sleep(Duration::from_millis(10));
    }
}

/// Publishes a complete source edit to the watcher.
fn replace_source(path: &Path, text: &str) {
    let staging = path.with_extension("next");
    write_file(&staging, text);
    fs::rename(staging, path).unwrap();
}

/// Returns a library whose generated output varies with the field name.
fn valid_source(field: &str) -> String {
    format!(
        "import 'package:dust_dart/serde.dart';\n\
         part 'user.g.dart';\n\
         @ToString()\n\
         class User {{\n\
           final String {field};\n\
           const User(this.{field});\n\
         }}\n"
    )
}

#[test]
fn reports_each_edit_live_and_keeps_cache_after_interruption() {
    let workspace = make_workspace();
    let root = workspace.path();
    let root_arg = root.to_str().unwrap();
    let source = root.join("lib/user.dart");
    let output = root.join("lib/user.g.dart");
    let cache = root.join(".dart_tool/dust/build_cache_v1.json");
    let stderr = root.join("watch.stderr");
    write_file(&source, &valid_source("id"));
    let mut watch = WatchChild(
        cli_command(["watch", "--root", root_arg, "--poll-ms", "20"])
            .stdout(Stdio::null())
            .stderr(File::create(&stderr).unwrap())
            .spawn()
            .unwrap(),
    );
    wait_until("the initial watch cache", || cache.is_file());
    let original_cache = fs::read(&cache).unwrap();

    for invalid in [
        "final FirstBrokenExpression = ;",
        "final SecondBrokenExpression = ;",
    ] {
        replace_source(&source, &format!("{}{invalid}\n", valid_source("id")));
        wait_until(invalid, || {
            fs::read_to_string(&stderr)
                .unwrap_or_default()
                .contains(invalid)
        });
        assert!(watch.0.try_wait().unwrap().is_none());
    }

    replace_source(&source, &valid_source("label"));
    wait_until("the rebuilt watch cache", || {
        fs::read(&cache).is_ok_and(|bytes| bytes != original_cache)
    });
    let generated = fs::read(&output).unwrap();
    drop(watch);

    let warm = run_cli(["build", "--root", root_arg]);
    assert_eq!(warm.exit_code, 0, "{}", warm.stderr);
    assert!(warm.stdout.contains("cached: 1"), "{}", warm.stdout);
    assert_eq!(fs::read(&output).unwrap(), generated);
}

#[test]
fn bounded_watch_prints_diagnostic_details_once_and_preserves_exit_status() {
    let workspace = make_workspace();
    let root = workspace.path();
    let invalid = "final FirstBrokenExpression = ;";
    write_file(
        &root.join("lib/user.dart"),
        &format!("{}{invalid}\n", valid_source("id")),
    );

    let run = run_cli([
        "watch",
        "--root",
        root.to_str().unwrap(),
        "--poll-ms",
        "10",
        "--max-cycles",
        "1",
    ]);

    assert_eq!(run.exit_code, 1);
    assert!(
        run.stderr.contains("diagnostics  errors: 1"),
        "{}",
        run.stderr
    );
    assert_eq!(run.stderr.matches(invalid).count(), 1, "{}", run.stderr);
}
