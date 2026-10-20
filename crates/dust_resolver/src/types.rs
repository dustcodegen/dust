use dust_ir::{BuiltinType, LoweringOutcome, NameIr, RedirectTargetIr, SpanIr, TypeIr};
use dust_parser_dart::{ParsedRedirectTargetSurface, ParsedTypeKind, ParsedTypeSurface};
use dust_text::{FileId, TextRange};

/// Lowers a parser-owned redirect without treating its application as a declaration name.
pub(crate) fn lower_redirect_target_ir(
    file_id: FileId,
    target: &ParsedRedirectTargetSurface,
    type_parameters: &[&str],
) -> RedirectTargetIr {
    RedirectTargetIr {
        ty: type_from_parsed_surface(&target.ty, type_parameters),
        constructor: target.constructor.as_ref().map(|constructor| NameIr {
            source: constructor.name.clone(),
            short: constructor.name.clone(),
            prefix: None,
            span: SpanIr::new(file_id, constructor.span),
        }),
        span: SpanIr::new(file_id, target.span),
    }
}

/// Lowers parser-owned Dart type facts, or a raw fallback source, into `TypeIr`.
pub fn lower_type_ir(
    parsed: Option<&ParsedTypeSurface>,
    source: Option<&str>,
) -> LoweringOutcome<TypeIr> {
    lower_type_ir_in_scope(parsed, source, &[])
}

/// Lowers one type in the lexical scope of class type-parameter names.
pub fn lower_type_ir_in_scope(
    parsed: Option<&ParsedTypeSurface>,
    source: Option<&str>,
    type_parameters: &[&str],
) -> LoweringOutcome<TypeIr> {
    if let Some(parsed) = parsed {
        return LoweringOutcome::new(type_from_parsed_surface(parsed, type_parameters));
    }

    let Some(source) = source.map(str::trim).filter(|source| !source.is_empty()) else {
        return LoweringOutcome::new(TypeIr::unknown());
    };

    let span = TextRange::new(0_u32, source.len() as u32);
    let parsed = ParsedTypeSurface::parse(source, span);
    LoweringOutcome::new(
        parsed
            .as_ref()
            .map(|parsed| type_from_parsed_surface(parsed, type_parameters))
            .unwrap_or_else(TypeIr::unknown),
    )
}

/// Converts structured parser type output into type IR.
fn type_from_parsed_surface(parsed: &ParsedTypeSurface, type_parameters: &[&str]) -> TypeIr {
    let ty = match &parsed.kind {
        ParsedTypeKind::Builtin(name) if type_parameters.contains(&name.as_str()) => {
            TypeIr::named(name.as_str())
        }
        ParsedTypeKind::Builtin(name) => parse_builtin(name)
            .map(TypeIr::builtin)
            .unwrap_or_else(|| TypeIr::named(name.as_str())),
        ParsedTypeKind::Named { name, args } => {
            let args = args
                .iter()
                .map(|arg| type_from_parsed_surface(arg, type_parameters))
                .collect::<Vec<_>>();
            if args.is_empty() {
                TypeIr::named(name.as_str())
            } else {
                TypeIr::generic(name.as_str(), args)
            }
        }
        ParsedTypeKind::Function => TypeIr::function(non_nullable_source(parsed)),
        ParsedTypeKind::Record => TypeIr::record(non_nullable_source(parsed)),
        ParsedTypeKind::Dynamic => TypeIr::dynamic(),
        ParsedTypeKind::Unknown => return TypeIr::unknown(),
    };

    if parsed.nullable { ty.nullable() } else { ty }
}

/// Returns the source form without a trailing nullable marker.
fn non_nullable_source(parsed: &ParsedTypeSurface) -> &str {
    parsed.source.strip_suffix('?').unwrap_or(&parsed.source)
}

/// Parses supported builtin Dart type names.
fn parse_builtin(source: &str) -> Option<BuiltinType> {
    match source {
        "String" => Some(BuiltinType::String),
        "int" => Some(BuiltinType::Int),
        "bool" => Some(BuiltinType::Bool),
        "double" => Some(BuiltinType::Double),
        "num" => Some(BuiltinType::Num),
        "Object" => Some(BuiltinType::Object),
        _ => None,
    }
}
