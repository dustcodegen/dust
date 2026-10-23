use dust_dart_emit::{DYNAMIC_TYPES, dart_string_literal};
use dust_ir::TypeIr;

use crate::plan::{
    BodyFormat, Extraction, ExtractionWrapper, ParamPlan, QueryConversionKind, QueryConversionPlan,
    QueryModelPlan, RawBodyFormat,
};

#[cfg(test)]
mod tests;

/// Stateful parameter emitter so form and multipart bodies are read once.
#[derive(Default)]
pub(super) struct ParamEmitter {
    /// Body already decoded for the current handler.
    shared_body: Option<SharedBody>,
}

impl ParamEmitter {
    /// Emits one parameter in declaration order.
    pub(super) fn push(&mut self, lines: &mut Vec<String>, index: usize, plan: &ParamPlan<'_>) {
        match &plan.extraction {
            Extraction::Request => push_passthrough(lines, index, "request"),
            Extraction::RequestParts => push_passthrough(lines, index, "RequestParts.of(request)"),
            Extraction::Field(name) => {
                self.ensure_shared(lines, SharedBody::Form);
                let ty = DYNAMIC_TYPES.render(plan.value_type);
                let expression = format!("_dustBody.field<{ty}>({})", dart_string_literal(name));
                push_evaluated(lines, index, plan, &expression);
            }
            Extraction::Part { name, file } => {
                self.ensure_shared(lines, SharedBody::Multipart);
                let expression = if *file {
                    format!("_dustBody.file({})", dart_string_literal(name))
                } else {
                    let ty = DYNAMIC_TYPES.render(plan.value_type);
                    format!("_dustBody.field<{ty}>({})", dart_string_literal(name))
                };
                push_evaluated(lines, index, plan, &expression);
            }
            _ => push_extractor(lines, index, plan),
        }
    }

    /// Emits the first shared body read and reuses it for later fields or parts.
    fn ensure_shared(&mut self, lines: &mut Vec<String>, body: SharedBody) {
        if self.shared_body.is_some() {
            return;
        }
        self.shared_body = Some(body);
        let (ty, extractor) = match body {
            SharedBody::Form => ("FormMap", "FormExtractable"),
            SharedBody::Multipart => ("MultipartForm", "MultipartExtractable"),
        };
        lines.push(format!("  final Result<{ty}, Rejection> _dustBodyResult ="));
        lines.push(format!("      await const {extractor}().extract(request);"));
        lines.push(
            "  if (_dustBodyResult case Err(:final error)) return error.intoResponse();".to_owned(),
        );
        lines.push(format!(
            "  final _dustBody = (_dustBodyResult as Ok<{ty}, Rejection>).value;"
        ));
        lines.push(String::new());
    }
}

#[derive(Debug, Clone, Copy)]
/// One decoded body shared by a field or part parameter group.
enum SharedBody {
    /// URL-encoded form fields.
    Form,
    /// Multipart fields and uploaded files.
    Multipart,
}

/// Emits a parameter that directly reuses request state.
fn push_passthrough(lines: &mut Vec<String>, index: usize, expression: &str) {
    lines.push(format!("  final _dustValue{index} = {expression};"));
    lines.push(String::new());
}

/// Emits one runtime extractor and its rejection short circuit.
fn push_extractor(lines: &mut Vec<String>, index: usize, plan: &ParamPlan<'_>) {
    let ty = DYNAMIC_TYPES.render(&plan.param.ty);
    let extractor = render_extractor(plan);
    lines.push(format!(
        "  final Result<{ty}, Rejection> _dustResult{index} ="
    ));
    let extraction_call = format!("      await {extractor}.extract(request);");
    if !extractor.contains('\n') && extraction_call.len() + 2 > 80 {
        lines.push(format!("      await {extractor}"));
        lines.push("          .extract(request);".to_owned());
    } else {
        lines.push(extraction_call);
    }
    push_unwrap(lines, index, &ty);
}

/// Emits extraction from an already decoded body value.
fn push_evaluated(lines: &mut Vec<String>, index: usize, plan: &ParamPlan<'_>, expression: &str) {
    let value_type = DYNAMIC_TYPES.render(plan.value_type);
    let ty = DYNAMIC_TYPES.render(&plan.param.ty);
    let expression = match plan.wrapper {
        ExtractionWrapper::Required => expression.to_owned(),
        ExtractionWrapper::Optional => {
            format!("optionalExtractionResult<{value_type}>({expression})")
        }
        ExtractionWrapper::Fallible => {
            format!("fallibleExtractionResult<{value_type}>({expression})")
        }
    };
    lines.push(format!(
        "  final Result<{ty}, Rejection> _dustResult{index} ="
    ));
    lines.push(format!("      {expression};"));
    push_unwrap(lines, index, &ty);
}

/// Emits the successful value read after extraction.
fn push_unwrap(lines: &mut Vec<String>, index: usize, ty: &str) {
    lines.push(format!(
        "  if (_dustResult{index} case Err(:final error)) return error.intoResponse();"
    ));
    let unwrap =
        format!("  final _dustValue{index} = (_dustResult{index} as Ok<{ty}, Rejection>).value;");
    if unwrap.len() + 2 > 80 {
        lines.push(format!("  final _dustValue{index} ="));
        lines.push(format!(
            "      (_dustResult{index} as Ok<{ty}, Rejection>).value;"
        ));
    } else {
        lines.push(unwrap);
    }
    lines.push(String::new());
}

/// Renders the runtime extractor selected for a parameter.
pub(super) fn render_extractor(plan: &ParamPlan<'_>) -> String {
    let (mut inner, is_const) = render_base_extractor(plan);
    let value_type = DYNAMIC_TYPES.render(plan.value_type);
    let decoder = render_decoder(plan, &value_type);
    if plan.validated {
        inner = match plan.extraction {
            Extraction::Body(BodyFormat::Json) => format!(
                "ValidatedExtractable<{value_type}>(\n    JsonExtractable<{value_type}>({decoder}),\n  )"
            ),
            Extraction::Body(BodyFormat::Form) => format!(
                "ValidatedExtractable<{value_type}>(\n    FormModelExtractable<{value_type}>({decoder}),\n  )"
            ),
            Extraction::QueryModel(_) => format!(
                "ValidatedExtractable<{value_type}>(\n    {},\n  )",
                inner.replace('\n', "\n    ")
            ),
            _ => inner,
        };
    }
    match plan.wrapper {
        ExtractionWrapper::Required if is_const => format!("const {inner}"),
        ExtractionWrapper::Required => inner,
        ExtractionWrapper::Optional if is_const => {
            format!("const OptionalExtractable<{value_type}>({inner})")
        }
        ExtractionWrapper::Optional => format!("OptionalExtractable<{value_type}>({inner})"),
        ExtractionWrapper::Fallible if is_const => {
            format!("const FallibleExtractable<{value_type}>({inner})")
        }
        ExtractionWrapper::Fallible => format!("FallibleExtractable<{value_type}>({inner})"),
    }
}

/// Renders the unwrapped extractor constructor.
fn render_base_extractor(plan: &ParamPlan<'_>) -> (String, bool) {
    let ty = DYNAMIC_TYPES.render(plan.value_type);
    let decoder = render_decoder(plan, &ty);
    match &plan.extraction {
        Extraction::Path(key) => (
            format!("PathExtractable<{ty}>({})", dart_string_literal(key)),
            true,
        ),
        Extraction::Query(key) => (
            format!("QueryExtractable<{ty}>({})", dart_string_literal(key)),
            true,
        ),
        Extraction::QueryList(key) => {
            let item = DYNAMIC_TYPES.render(&plan.value_type.args()[0]);
            (
                format!("QueryListExtractable<{item}>({})", dart_string_literal(key)),
                true,
            )
        }
        Extraction::QueryModel(query) => (render_query_model(plan, query, &ty), false),
        Extraction::Queries if is_string_map(plan.value_type) => (
            "QueriesExtractable<Map<String, String>>((queries) => queries)".to_owned(),
            false,
        ),
        Extraction::Queries => (
            format!("QueriesExtractable<{ty}>(\n    {decoder},\n  )"),
            true,
        ),
        Extraction::Header(name) => (
            format!("HeaderExtractable<{ty}>({})", dart_string_literal(name)),
            true,
        ),
        Extraction::HeaderMap => ("HeaderMapExtractable()".to_owned(), true),
        Extraction::Body(BodyFormat::Json) if is_json_map(plan.value_type) => {
            (format!("JsonExtractable<{ty}>((json) => json)"), false)
        }
        Extraction::Body(BodyFormat::Json) => {
            (format!("JsonExtractable<{ty}>(\n    {decoder},\n  )"), true)
        }
        Extraction::Body(BodyFormat::Form) if is_json_map(plan.value_type) => (
            format!("FormModelExtractable<{ty}>((fields) => fields)"),
            false,
        ),
        Extraction::Body(BodyFormat::Form) => (
            format!("FormModelExtractable<{ty}>(\n    {decoder},\n  )"),
            true,
        ),
        Extraction::Custom(name) => (format!("{name}()"), true),
        Extraction::RawQuery => ("RawQueryExtractable()".to_owned(), true),
        Extraction::RawBody(RawBodyFormat::Bytes) => ("RawBodyExtractable()".to_owned(), true),
        Extraction::RawBody(RawBodyFormat::Text) => ("TextBodyExtractable()".to_owned(), true),
        Extraction::RawBody(RawBodyFormat::Stream) => ("StreamBodyExtractable()".to_owned(), true),
        Extraction::State => (format!("StateExtractable<{ty}>()"), true),
        Extraction::Peer => ("PeerExtractable()".to_owned(), true),
        Extraction::Field(_)
        | Extraction::Part { .. }
        | Extraction::Request
        | Extraction::RequestParts => unreachable!("handled before extractor rendering"),
    }
}

/// Renders the adapter from URI pairs to the generated SerDe decoder.
fn render_query_model(plan: &ParamPlan<'_>, query: &QueryModelPlan, ty: &str) -> String {
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
    match &conversion.kind {
        QueryConversionKind::Scalar(value) => format!(
            "  query.coerceOne<{}>(values, {key});",
            DYNAMIC_TYPES.render(value)
        ),
        QueryConversionKind::StringList => format!("  query.copyAll(values, {key});"),
        QueryConversionKind::ScalarList(value) => format!(
            "  query.coerceAll<{}>(values, {key});",
            DYNAMIC_TYPES.render(value)
        ),
    }
}

/// Renders the generated Serde decoder or the model's compatibility fallback.
fn render_decoder(plan: &ParamPlan<'_>, ty: &str) -> String {
    if plan.serde_deserialized {
        let name = plan
            .value_type
            .name()
            .expect("Serde-derived request models are named types");
        format!("_${name}Deserialize")
    } else {
        format!("{ty}.deserialize")
    }
}

/// Returns whether a type is a string-to-string map.
pub(super) fn is_string_map(ty: &TypeIr) -> bool {
    ty.is_named("Map")
        && ty.args().len() == 2
        && ty.args()[0].is_named("String")
        && ty.args()[1].is_named("String")
}

/// Returns whether a type is a JSON-object map.
pub(super) fn is_json_map(ty: &TypeIr) -> bool {
    ty.is_named("Map") && ty.args().len() == 2 && ty.args()[0].is_named("String")
}
