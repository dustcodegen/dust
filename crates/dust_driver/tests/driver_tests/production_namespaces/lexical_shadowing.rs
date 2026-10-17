use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn class_constant_metadata_shadows_an_import_prefix_without_changing_generation() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!(
        "import 'package:dust_dart/derive.dart' as d;\n{}",
        MODEL.replace("@Derive([ToString()])", "@d.Derive([d.ToString()])")
    );
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    write_file(
        &model,
        &format!(
            "{source}\nclass Other {{
        const Other(this.value); static const d = Object(); @d final int value;
    }}"
        ),
    );
    let result = build(root);
    assert_eq!(result.diagnostics, []);
    assert_eq!(fs::read(output_path).unwrap(), output);
    let warm = build(root);
    assert_eq!(warm.diagnostics, []);
    assert_eq!(warm.cache.unwrap().hits, 1);
}
