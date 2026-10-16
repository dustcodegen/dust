use std::collections::HashMap;

use dust_plugin_api::WorkspaceAnalysis;

use super::{DeclaredRow, ROW_COLUMNS_KEY, parse_row};

/// Returns parsed row declarations and their source paths in one package.
pub(in crate::plugin) fn package_row_declarations(
    analysis: &WorkspaceAnalysis,
    package: &str,
) -> HashMap<String, DeclaredRow> {
    analysis
        .string_set(ROW_COLUMNS_KEY)
        .unwrap_or_default()
        .iter()
        .filter_map(|value| parse_row(value, package))
        .collect()
}
