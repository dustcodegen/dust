use dust_dart_emit::{DYNAMIC_TYPES, dart_string_literal};

use crate::plan::{ParamPlan, QueryConversionKind, QueryConversionPlan, QueryModelPlan};

use super::render_decoder;

/// Renders the adapter from URI pairs to the generated SerDe decoder.
pub(super) fn render_query_model(plan: &ParamPlan<'_>, query: &QueryModelPlan, ty: &str) -> String {
    let decoder = render_decoder(plan, ty);
    let mut lines = vec![
        format!("QueryModelExtractable<{ty}>((query) {{"),
        "  final values = query.toObjectMap();".to_owned(),
    ];
    for conversion in &query.conversions {
        lines.push(render_query_conversion(conversion));
    }
    lines.push(format!("  return {decoder}(values);"));
    lines.push("})".to_owned());
    lines.join("\n")
}

/// Renders one field conversion inside a query-model adapter.
fn render_query_conversion(conversion: &QueryConversionPlan) -> String {
    let key = dart_string_literal(&conversion.key);
    let aliases = if conversion.aliases.is_empty() {
        String::new()
    } else {
        format!(
            ", aliases: const [{}]",
            conversion
                .aliases
                .iter()
                .map(|alias| dart_string_literal(alias))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    match &conversion.kind {
        QueryConversionKind::Scalar(value) => format!(
            "  query.coerceOne<{}>(values, {key}{aliases});",
            DYNAMIC_TYPES.render(value)
        ),
        QueryConversionKind::StringList => {
            format!("  query.copyAll(values, {key}{aliases});")
        }
        QueryConversionKind::ScalarList(value) => format!(
            "  query.coerceAll<{}>(values, {key}{aliases});",
            DYNAMIC_TYPES.render(value)
        ),
    }
}
