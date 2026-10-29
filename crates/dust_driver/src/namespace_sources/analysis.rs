use std::{collections::BTreeSet, path::Path};

use dust_diagnostics::Diagnostic;
use dust_parser_dart::ParsedDartFileSurface;
use dust_text::FileId;

use super::NamespaceSources;

/// One imported annotated library available for structured plugin analysis.
pub(crate) struct DependencyAnalysisSource<'a> {
    /// Canonical package URI retained by the source graph.
    pub(crate) uri: &'a str,
    /// Owning Dart package root.
    pub(crate) package_root: std::path::PathBuf,
    /// Owning Dart package name.
    pub(crate) package_name: &'a str,
    /// Handwritten source path.
    pub(crate) source_path: &'a Path,
    /// Primary output path used while lowering part ownership.
    pub(crate) output_path: std::path::PathBuf,
    /// Stable file identifier from the shared source graph.
    pub(crate) file_id: FileId,
    /// Parser facts retained by the shared source graph.
    pub(crate) parsed: &'a ParsedDartFileSurface,
    /// Parser diagnostics retained for this dependency.
    pub(crate) diagnostics: &'a [Diagnostic],
}

impl NamespaceSources {
    /// Returns imported package libraries that contain active Dust annotations.
    pub(crate) fn dependency_analysis_sources(
        &self,
        registry: &dust_plugin_api::PluginRegistry,
    ) -> Vec<DependencyAnalysisSource<'_>> {
        let supported = registry
            .all_supported_annotations()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let roots = self.roots.values().cloned().collect::<BTreeSet<_>>();
        self.units
            .iter()
            .filter_map(|(uri, unit)| {
                if roots.contains(uri) {
                    return None;
                }
                let unit = unit.as_ref().ok()?;
                if unit.generated_owner.is_some()
                    || unit.generated_library
                    || !super::metadata::annotations(&unit.parsed)
                        .iter()
                        .any(|annotation| supported.contains(annotation.name.as_str()))
                {
                    return None;
                }
                let package = uri.strip_prefix("package:")?;
                let (package_name, relative) = package.split_once('/')?;
                let mut library_root = unit.source.path.clone();
                for _ in Path::new(relative).components() {
                    library_root.pop();
                }
                Some(DependencyAnalysisSource {
                    uri,
                    package_root: library_root.parent()?.to_path_buf(),
                    package_name,
                    source_path: &unit.source.path,
                    output_path: dust_workspace::primary_output_path(
                        &unit.source.path,
                        &self.primary_suffix,
                    ),
                    file_id: unit.file_id,
                    parsed: &unit.parsed,
                    diagnostics: &unit.diagnostics,
                })
            })
            .collect()
    }
}
