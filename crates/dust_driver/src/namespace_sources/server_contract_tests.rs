use super::*;
use dust_ir::{AnnotationOrigin, SpanIr, SymbolId};
use std::fs;

/// Shared source-only dependency and SDK fixtures; no installed SDK is selected.
#[path = "../../../../fixtures/namespace_sources/workspace.rs"]
mod namespace_fixture;
/// Actual runtime library trees are the same in unit and command-level checks.
#[path = "../../../../fixtures/namespace_server_barrel/workspace.rs"]
mod public_fixture;

#[test]
fn production_loader_preserves_server_owners_before_and_after_explicit_catalog_claims() {
    for server_claimed in [false, true] {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        fs::write(root.join("pubspec.yaml"), "name: dust_test\n").unwrap();
        namespace_fixture::prepare(root, root, "dust_test");
        namespace_fixture::set_language_version(root, "dust_test", "3.6");
        public_fixture::prepare(root);
        fs::create_dir_all(root.join("lib")).unwrap();
        let model = root.join("lib/model.dart");
        fs::write(
            &model,
            include_str!("../../../../fixtures/namespace_server_barrel/model.dart.snapshot"),
        )
        .unwrap();
        let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
        let mut catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
        if server_claimed {
            for origin in [
                AnnotationOrigin::new(
                    "dust_server::GET",
                    "package:dust_server/src/annotations/http_methods.dart",
                    "GET",
                ),
                AnnotationOrigin::new(
                    "dust_server::Path",
                    "package:dust_server/src/annotations/parameters.dart",
                    "Path",
                ),
                AnnotationOrigin::new(
                    "dust_server::Query",
                    "package:dust_server/src/annotations/parameters.dart",
                    "Query",
                ),
            ] {
                catalog.register_config(origin.declaration, origin.symbol);
                catalog.register_annotation_origin(origin).unwrap();
            }
        }
        let mut plan = dust_workspace::discover_workspace(
            root,
            &dust_workspace::SupportedAnnotations::new(registry.all_supported_annotations()),
        )
        .unwrap();
        let sources = NamespaceSources::load(
            root,
            &mut plan,
            Some(&namespace_fixture::sdk_source_root(root)),
            &catalog,
            &registry,
            false,
        )
        .unwrap();
        assert_eq!(plan.libraries.len(), 1);
        assert_eq!(sources.binding_errors(&model), []);
        let (parsed, diagnostics) = sources.parsed(&model).unwrap();
        assert_eq!(diagnostics, []);
        let method = &parsed.classes[0].methods[0];
        let annotations = method
            .annotations
            .iter()
            .chain(method.params.iter().flat_map(|param| &param.annotations))
            .collect::<Vec<_>>();
        assert_eq!(
            annotations
                .iter()
                .map(|annotation| annotation.name.as_str())
                .collect::<Vec<_>>(),
            ["GET", "Path", "Query"]
        );
        let file_id = sources.file_id(&model).unwrap();
        let scope = sources.scope(&model).unwrap();
        for annotation in annotations {
            assert_eq!(
                scope.bind_at(
                    &annotation.qualified_name,
                    &catalog,
                    SpanIr::new(file_id, annotation.span)
                ),
                if server_claimed {
                    NamespaceBinding::Owned(SymbolId::new(format!(
                        "dust_server::{}",
                        annotation.name
                    )))
                } else {
                    NamespaceBinding::Foreign
                }
            );
        }
        let resolved = dust_resolver::resolve_library_in_namespace(
            file_id,
            "model.dart",
            "model.g.dart",
            parsed,
            &catalog,
            scope,
            &[],
        );
        assert_eq!(resolved.diagnostics, []);
        let method = &resolved.library.classes[0].methods[0];
        assert_eq!(method.params.len(), 2);
        for (configs, name) in std::iter::once((&method.configs, "GET")).chain(
            method
                .params
                .iter()
                .zip(["Path", "Query"])
                .map(|(param, name)| (&param.configs, name)),
        ) {
            assert_eq!(
                configs
                    .iter()
                    .map(|config| config.symbol.0.clone())
                    .collect::<Vec<_>>(),
                if server_claimed {
                    vec![format!("dust_server::{name}")]
                } else {
                    vec![]
                }
            );
            assert!(configs.iter().all(|config| config.normalized.is_none()));
        }
    }
}
