use std::path::{Path, PathBuf};

/// The one path list shared by namespace planning and route emission.
pub(super) fn route_output_paths(directory: &Path) -> [PathBuf; 5] {
    ["routes", "paths", "metadata", "navigation", "runtime"]
        .map(|name| directory.join(format!("{name}.g.dart")))
}
