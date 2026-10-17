use super::*;

/// Explicit class inputs, keeping parser-owned header facts in lexical scope.
pub(super) struct ClassLoweringInput<'a> {
    /// Declaration kind.
    pub(super) kind: ClassKindIr,
    /// Class name.
    pub(super) name: &'a str,
    /// Ordered class type parameters and written bounds.
    pub(super) type_parameters: &'a [ParsedTypeParamSurface],
    /// Written mixin types and their source spans.
    pub(super) mixins: &'a [ParsedTypeSurface],
    /// Whether the class is abstract.
    pub(super) is_abstract: bool,
    /// Whether the class is an interface class.
    pub(super) is_interface: bool,
    /// Immediate superclass name.
    pub(super) superclass_name: Option<&'a str>,
    /// Written superclass application in the owning class scope.
    pub(super) superclass: Option<&'a ParsedTypeSurface>,
    /// Class source span.
    pub(super) span: SpanIr,
    /// Resolved fields.
    pub(super) fields: &'a mut [ResolvedField],
    /// Resolved constructors.
    pub(super) constructors: &'a [ResolvedConstructor],
    /// Resolved methods.
    pub(super) methods: &'a [ResolvedMethod],
    /// Resolved trait applications.
    pub(super) traits: &'a [TraitApplicationIr],
    /// Resolved configuration applications.
    pub(super) configs: &'a [ConfigApplicationIr],
    /// Resolver-normalized SerDe configuration.
    pub(super) serde_value: &'a mut Option<SerdeClassConfigIr>,
}

/// Lowers class headers and members with all owning type-parameter names available.
pub(super) fn lower_class_from_parts(input: ClassLoweringInput<'_>) -> LoweringOutcome<ClassIr> {
    let names = input
        .type_parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let type_parameters = input
        .type_parameters
        .iter()
        .map(|parameter| {
            let bound = parameter.bound.as_ref().map(|bound| {
                let outcome = lower_type_in_scope(Some(bound), None, &names);
                diagnostics.extend(outcome.diagnostics);
                outcome.value
            });
            TypeParamIr {
                name: lower_name_ir(input.span.file_id, &parameter.name, parameter.name_span),
                bound,
                span: SpanIr::new(input.span.file_id, parameter.span),
            }
        })
        .collect();
    let mixins = input
        .mixins
        .iter()
        .map(|mixin| {
            let outcome = lower_type_in_scope(Some(mixin), None, &names);
            diagnostics.extend(outcome.diagnostics);
            MixinApplicationIr {
                ty: outcome.value,
                span: SpanIr::new(input.span.file_id, mixin.span),
            }
        })
        .collect();
    let superclass = input.superclass.map(|superclass| {
        let outcome = lower_type_in_scope(Some(superclass), None, &names);
        diagnostics.extend(outcome.diagnostics);
        dust_ir::SuperclassIr {
            ty: outcome.value,
            span: SpanIr::new(input.span.file_id, superclass.span),
        }
    });
    let fields = lower_resolved_fields(input.fields, &names, &mut diagnostics);
    let methods = lower_resolved_methods(input.methods, &names, &mut diagnostics);
    let constructors = lower_resolved_constructors(
        input.span.file_id,
        input.constructors,
        &fields,
        &names,
        &mut diagnostics,
    );
    LoweringOutcome {
        value: ClassIr {
            kind: input.kind,
            name: input.name.to_owned(),
            type_parameters,
            mixins,
            is_abstract: input.is_abstract,
            is_interface: input.is_interface,
            superclass_name: input.superclass_name.map(str::to_owned),
            superclass,
            span: input.span,
            fields,
            constructors,
            methods,
            traits: input.traits.to_vec(),
            configs: input.configs.to_vec(),
            serde: input.serde_value.take(),
        },
        diagnostics,
    }
}
