use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{
    ClassKindIr, ConfigApplicationIr, DbConfigIr, NormalizedConfigIr, SpanIr, TraitApplicationIr,
};
use dust_parser_dart::{
    ParsedAnnotation, ParsedClassKind, ParsedClassSurface, ParsedDartFileSurface, ParsedDirective,
};
use dust_text::FileId;

use crate::{
    NamespaceScope, ResolveResult, ResolvedClass, ResolvedEnum, ResolvedEnumVariant,
    ResolvedLibrary, SymbolCatalog,
    binding::BindingContext,
    db::normalize_db,
    http::normalize_http,
    query_calls::normalize_query_calls,
    resolve_support::{
        first_part_uri, resolve_constructor, resolve_declaration_annotations, resolve_field,
        resolve_method,
    },
    route::normalize_route,
    serde::{normalize_class_serde, normalize_enum_variant_serde, normalize_sealed_serde_variants},
    state::normalize_state,
};

/// Deciding whether a declaration needs a generated part, and checking the part URI.
mod parts;
use self::parts::*;

/// Checking that annotations reached through an import prefix are spelled with it.
mod prefixes;
use self::prefixes::*;

/// Resolving one class body into the resolver's own representation.
mod classes;
use self::classes::*;

/// Resolves one parsed library against a symbol catalog.
pub fn resolve_library(
    file_id: FileId,
    source_path: &str,
    output_path: &str,
    library: &ParsedDartFileSurface,
    catalog: &SymbolCatalog,
) -> ResolveResult {
    resolve_library_with_partless_configs(file_id, source_path, output_path, library, catalog, &[])
}

/// Resolves one parsed library while allowing selected config symbols to emit standalone outputs.
pub fn resolve_library_with_partless_configs(
    file_id: FileId,
    source_path: &str,
    output_path: &str,
    library: &ParsedDartFileSurface,
    catalog: &SymbolCatalog,
    partless_config_symbols: &[&str],
) -> ResolveResult {
    resolve_library_with_bindings(
        file_id,
        source_path,
        output_path,
        library,
        &BindingContext {
            catalog,
            namespace: None,
        },
        partless_config_symbols,
    )
}

/// Resolves actual imported declarations before any feature normalization.
///
/// The caller supplies a fully loaded namespace graph; this does not load files
/// or change the context-free compatibility entrypoints.
pub fn resolve_library_in_namespace(
    file_id: FileId,
    source_path: &str,
    output_path: &str,
    library: &ParsedDartFileSurface,
    catalog: &SymbolCatalog,
    namespace: NamespaceScope<'_>,
    partless_config_symbols: &[&str],
) -> ResolveResult {
    resolve_library_with_bindings(
        file_id,
        source_path,
        output_path,
        library,
        &BindingContext {
            catalog,
            namespace: Some(namespace),
        },
        partless_config_symbols,
    )
}

/// Compatibility and production-context paths share feature resolution.
pub fn resolve_library_with_bindings(
    file_id: FileId,
    source_path: &str,
    output_path: &str,
    library: &ParsedDartFileSurface,
    context: &BindingContext<'_>,
    partless_config_symbols: &[&str],
) -> ResolveResult {
    let mut diagnostics = Vec::new();
    if context.namespace.is_none() {
        validate_annotation_prefixes(file_id, library, &mut diagnostics);
    }
    let part_uri = first_part_uri(&library.directives);
    let mut enums: Vec<ResolvedEnum> = Vec::new();
    let mut classes = Vec::new();
    let mut saw_dust_symbol = false;

    for class in &library.classes {
        let resolved = resolve_class(file_id, class, context, &mut diagnostics);
        if class_has_dust_symbol(&resolved) {
            saw_dust_symbol = true;
        }
        classes.push(resolved);
    }
    for enum_surface in &library.enums {
        let resolved: ResolvedEnum = resolve_enum(file_id, enum_surface, context, &mut diagnostics);
        if !resolved.traits.is_empty() || !resolved.configs.is_empty() {
            saw_dust_symbol = true;
        }
        enums.push(resolved);
    }
    normalize_sealed_serde_variants(&mut classes, &mut diagnostics);
    mark_required_lowering_diagnostics(&mut classes);

    let needs_part = saw_dust_symbol
        && classes
            .iter()
            .any(|class| class_needs_part(class, partless_config_symbols))
        || enums
            .iter()
            .any(|enum_ir| enum_needs_part(enum_ir, partless_config_symbols));

    if needs_part {
        match part_uri.as_deref() {
            Some(uri) => {
                if let Err(diagnostic) = validate_generated_part_uri(output_path, uri) {
                    diagnostics.push(diagnostic);
                }
            }
            None => diagnostics.push(
                Diagnostic::error("missing generated `part` directive for Dust-enabled library")
                    .with_label(SourceLabel::new(
                        file_id,
                        library.span,
                        format!(
                            "expected a matching `part '{}';` directive",
                            expected_part_uri(output_path)
                        ),
                    )),
            ),
        }
    }

    ResolveResult {
        library: ResolvedLibrary {
            source_path: source_path.to_owned(),
            output_path: output_path.to_owned(),
            span: SpanIr::new(file_id, library.span),
            directives: library.directives.clone(),
            part_uri,
            classes,
            enums,
            mixins: library.mixins.clone(),
            extensions: library.extensions.clone(),
            extension_types: library.extension_types.clone(),
            functions: library.functions.clone(),
            variables: library.variables.clone(),
            typedefs: library.typedefs.clone(),
            query_calls: normalize_query_calls(file_id, &library.query_calls, &mut diagnostics),
        },
        diagnostics,
    }
}

/// Resolves one parsed enum into semantic data.
fn resolve_enum(
    file_id: FileId,
    enum_surface: &dust_parser_dart::ParsedEnumSurface,
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedEnum {
    let mut traits: Vec<TraitApplicationIr> = Vec::new();
    let mut configs: Vec<ConfigApplicationIr> = Vec::new();
    resolve_declaration_annotations(
        file_id,
        &enum_surface.annotations,
        context,
        diagnostics,
        &mut traits,
        &mut configs,
    );

    let variants = enum_surface
        .variants
        .iter()
        .map(|variant| {
            let mut variant_traits = Vec::new();
            let mut variant_configs = Vec::new();
            resolve_declaration_annotations(
                file_id,
                &variant.annotations,
                context,
                diagnostics,
                &mut variant_traits,
                &mut variant_configs,
            );
            ResolvedEnumVariant {
                name: variant.name.clone(),
                span: SpanIr::new(file_id, variant.span),
                serde: normalize_enum_variant_serde(&variant.name, &variant_configs, diagnostics),
                configs: variant_configs,
            }
        })
        .collect();

    ResolvedEnum {
        name: enum_surface.name.clone(),
        span: SpanIr::new(file_id, enum_surface.span),
        variants,
        traits,
        serde: normalize_class_serde(&enum_surface.name, &configs, diagnostics),
        configs,
    }
}

/// Validates that a generated part URI matches the source file name.
pub fn validate_generated_part_uri(output_path: &str, part_uri: &str) -> Result<(), Diagnostic> {
    let expected = expected_part_uri(output_path);
    let Some(file_name) = Path::new(part_uri)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return Err(Diagnostic::error(
            "generated part path must contain a valid file name",
        ));
    };

    if file_name != expected {
        return Err(Diagnostic::error(format!(
            "generated part path `{file_name}` does not match expected `{expected}`"
        )));
    }

    Ok(())
}

/// Returns the expected part URI from a generated output path.
fn expected_part_uri(output_path: &str) -> String {
    Path::new(output_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file.g.dart")
        .to_owned()
}
