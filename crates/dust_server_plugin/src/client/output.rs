use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::DartFileIr;
use dust_plugin_api::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};

use crate::{ServerClientDestination, plan::ServerPlan};

/// Tests whether a route is included in one client destination.
pub(super) fn accepts_tags(destination: &ServerClientDestination, tags: &[String]) -> bool {
    destination.tags.is_empty() || tags.iter().any(|tag| destination.tags.contains(tag))
}

/// Indexes which source libraries generate each path before any output is written.
pub(crate) fn collect_output_claims(
    file: &DartFileIr,
    plan: &ServerPlan<'_>,
    destinations: &[ServerClientDestination],
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    for destination in destinations {
        let Ok(barrel) = main_output_path(file, destination) else {
            continue;
        };
        let groups = std::iter::once((None, plan.functions.as_slice())).chain(
            plan.controllers.iter().map(|controller| {
                (
                    Some(controller.class.name.as_str()),
                    controller.handlers.as_slice(),
                )
            }),
        );
        let mut has_groups = false;
        for (name, handlers) in groups {
            if handlers
                .iter()
                .any(|handler| accepts_tags(destination, &handler.metadata.tags))
                && let Ok((_, suffix)) = group_identity(name, &file.source_path)
            {
                let path = group_output_path(&barrel, &suffix);
                analysis.add_string_set_value(claim_key(&path), file.source_path.clone());
                has_groups = true;
            }
        }
        if has_groups {
            analysis.add_string_set_value(claim_key(&barrel), file.source_path.clone());
        }
    }
}

/// Plans and validates the generated output paths for one client destination.
pub(super) fn plan_group_outputs<'a>(
    file: &DartFileIr,
    plan: &ServerPlan<'_>,
    barrel: &Path,
    groups: impl Iterator<Item = Option<&'a str>>,
    analysis: &WorkspaceAnalysis,
) -> Result<Vec<(String, PathBuf)>, Vec<Diagnostic>> {
    let mut outputs = Vec::new();
    let mut diagnostics = Vec::new();
    let mut claimed = BTreeMap::new();
    for name in groups {
        let (class_name, suffix) = group_identity(name, &file.source_path)?;
        let path = group_output_path(barrel, &suffix);
        let label = group_label(file, plan, name);
        if let Some(previous) = claimed.get(&path) {
            diagnostics.push(
                collision(&path)
                    .with_label(label)
                    .with_label(SourceLabel::clone(previous)),
            );
        } else {
            check_other_sources(file, analysis, &path, &label, &mut diagnostics);
            claimed.insert(path.clone(), label);
        }
        outputs.push((class_name, path));
    }
    if let Some(label) = claimed.values().next() {
        check_other_sources(file, analysis, barrel, label, &mut diagnostics);
    }
    if diagnostics.is_empty() {
        Ok(outputs)
    } else {
        Err(diagnostics)
    }
}

/// Labels the annotation or declaration that defines one generated group.
fn group_label(file: &DartFileIr, plan: &ServerPlan<'_>, name: Option<&str>) -> SourceLabel {
    let (span, message) = if let Some(name) = name {
        let span = plan
            .controllers
            .iter()
            .find(|controller| controller.class.name == name)
            .map_or(file.span, |controller| controller.class.span);
        (
            span,
            format!("controller `{name}` contributes to this path"),
        )
    } else {
        let annotation = file.library_annotations.iter().find(|annotation| {
            annotation
                .resolved_symbol
                .as_ref()
                .is_some_and(|symbol| symbol.0 == "dust_server::Routes")
        });
        let span = annotation
            .map(|annotation| annotation.span)
            .or_else(|| {
                plan.functions
                    .first()
                    .map(|handler| handler.callable.span())
            })
            .unwrap_or(file.span);
        (span, "top-level routes contribute to this path".to_owned())
    };
    SourceLabel::new(span.file_id, span.range, message)
}

/// Diagnoses collision with a barrel or route group from another source library.
fn check_other_sources(
    file: &DartFileIr,
    analysis: &WorkspaceAnalysis,
    path: &Path,
    label: &SourceLabel,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for other in analysis.string_set(&claim_key(path)).unwrap_or_default() {
        if other != &file.source_path {
            diagnostics.push(
                collision(path)
                    .with_label(label.clone())
                    .with_note(format!("also generated from `{other}`")),
            );
        }
    }
}

/// Returns the stable analysis bucket for one output path.
fn claim_key(path: &Path) -> String {
    format!("dust_server.client_output.v1/{}", path.display())
}

/// Returns whether a current source library claims this generated client path.
pub fn is_client_output_claimed(analysis: &WorkspaceAnalysis, path: &Path) -> bool {
    analysis.string_set(&claim_key(path)).is_some()
}

/// Describes a duplicate generated output path.
fn collision(path: &Path) -> Diagnostic {
    Diagnostic::error(format!(
        "multiple server clients would generate `{}`",
        path.display()
    ))
}

/// Joins the generated barrel stem with a route group suffix.
fn group_output_path(barrel: &Path, suffix: &str) -> PathBuf {
    let stem = barrel
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .strip_suffix(".g")
        .unwrap();
    barrel.with_file_name(format!("{stem}.{suffix}.g.dart"))
}

/// Converts one route file stem into a Dart class-name prefix.
pub(super) fn file_class_name(source: &str) -> Result<String, Vec<Diagnostic>> {
    let stem = std::path::Path::new(source)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| vec![Diagnostic::error("server source has no UTF-8 file name")])?;
    let name = stem
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(|value| {
            let mut chars = value.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<String>();
    Ok(name)
}

/// Selects the class name and file suffix for one controller or route module.
pub(super) fn group_identity(
    controller: Option<&str>,
    source: &str,
) -> Result<(String, String), Vec<Diagnostic>> {
    match controller {
        Some(name) => Ok((format!("{name}Client"), snake_case(name))),
        None => {
            let name = file_class_name(source)?;
            Ok((format!("{name}RoutesClient"), "routes".to_owned()))
        }
    }
}

/// Converts a Dart class name into a deterministic file suffix.
pub(super) fn snake_case(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            output.push('_');
        }
        output.push(ch.to_ascii_lowercase());
    }
    output
}

/// Returns the generated barrel path inside the configured client package.
pub(super) fn main_output_path(
    file: &DartFileIr,
    destination: &ServerClientDestination,
) -> Result<PathBuf, Vec<Diagnostic>> {
    let source = std::path::Path::new(&file.source_path);
    let library_root = std::path::Path::new(&file.package_root).join("lib");
    let relative = source
        .strip_prefix(&library_root)
        .or_else(|_| source.strip_prefix("lib"))
        .map_err(|_| {
            vec![Diagnostic::error(
                "generated server clients require the route source to be under `lib/`",
            )]
        })?;
    let stem = relative
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| vec![Diagnostic::error("server source has no UTF-8 file name")])?;
    let mut output = destination
        .package_root
        .join("lib/src/generated")
        .join(relative);
    output.set_file_name(format!("{stem}_client.g.dart"));
    Ok(output)
}
