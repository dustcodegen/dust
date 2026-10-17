//! Phase-0 ownership evidence, not server registration or handler generation.

use super::*;
use dust_ir::{SpanIr, SymbolId};
/// Controller configurations and parameter metadata retain ownership and spans.
mod controller;
/// Library and top-level handler sites use the same scoped annotation binding.
mod top_level;

/// Declaring-library directory, distinct from the public re-export identity.
const ROOT: &str = "package:dust_server/src/annotations";
/// Actual annotation-only barrel used to isolate namespace ownership.
const BARREL: &str = "package:dust_server/src/annotations/annotations.dart";
/// Retain static origin strings without allocating/leaking test registrations.
macro_rules! origins {
    ($owner:literal, $uri:expr; $($name:ident),* $(,)?) => {
        &[$(AnnotationOrigin::new(
            concat!($owner, "::", stringify!($name)), $uri, stringify!($name),
        )),*]
    };
}

/// Exact runtime declarations; only the first four groups contain annotations.
const GROUPS: &[(&str, &[AnnotationOrigin])] = &[
    (
        "controller.dart",
        origins!("dust_server", "package:dust_server/src/annotations/controller.dart"; Controller),
    ),
    (
        "routes.dart",
        origins!("dust_server", "package:dust_server/src/annotations/routes.dart"; Routes),
    ),
    (
        "http_methods.dart",
        origins!("dust_server", SERVER; GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS),
    ),
    (
        "parameters.dart",
        origins!("dust_server", "package:dust_server/src/annotations/parameters.dart";
            Extract, Path, Query, Queries, RawQuery, Header, HeaderMap, Body,
            Field, Part, RawBody, State, Peer, FormUrlEncoded, MultiPart),
    ),
    (
        "verb.dart",
        origins!("dust_server", "package:dust_server/src/annotations/verb.dart"; Verb),
    ),
    ("annotations.dart", &[]),
];

/// Client claims match the existing HTTP plugin, excluding its abstract base.
const CLIENT_ORIGINS: &[AnnotationOrigin] = origins!("dust_dart", CLIENT;
    HttpClient, GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, Path, Query, Queries,
    Header, Headers, HeaderMap, Body, Field, Part, Extra, FormUrlEncoded, MultiPart,
    HttpParse);

/// Load actual declarations and their actual sibling re-exports, not mock classes.
fn surface() -> NamespaceGraph {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packages/dust_server/lib/src/annotations");
    let mut graph = NamespaceGraph::new();
    let client = root.join("../../../../dust_dart/lib/src/http/http_client_annotations.dart");
    let client = fs::read_to_string(client).unwrap();
    assert_eq!(
        parse(1, &client)
            .classes
            .iter()
            .filter(|class| class.name != "HttpMethod")
            .map(|class| class.name.as_str())
            .collect::<Vec<_>>(),
        CLIENT_ORIGINS
            .iter()
            .map(|origin| origin.declaration)
            .collect::<Vec<_>>()
    );
    add(&mut graph, 1, CLIENT, &client);
    for (index, (file, names)) in GROUPS.iter().enumerate() {
        let id = 10 + index as u32;
        let parsed = parse(id, &fs::read_to_string(root.join(file)).unwrap());
        assert_eq!(
            parsed
                .classes
                .iter()
                .map(|class| class.name.as_str())
                .collect::<Vec<_>>(),
            names
                .iter()
                .map(|origin| origin.declaration)
                .collect::<Vec<_>>(),
            "runtime declaration drift in {file}"
        );
        graph
            .insert_library(
                format!("{ROOT}/{file}"),
                FileId::new(id),
                &parsed,
                |owner, uri| {
                    Ok(if uri.starts_with("package:") {
                        uri.to_owned()
                    } else {
                        format!("{}/{uri}", owner.rsplit_once('/').unwrap().0)
                    })
                },
            )
            .unwrap();
    }
    graph
}

/// A test catalog only: production server registration remains gated by #84.
fn owners(reverse: bool) -> SymbolCatalog {
    let mut catalog = SymbolCatalog::new();
    for server_first in [reverse, !reverse] {
        if server_first {
            for (_, origins) in &GROUPS[..4] {
                for origin in *origins {
                    catalog.register_config(origin.declaration, origin.symbol);
                    catalog.register_annotation_origin(*origin).unwrap();
                }
            }
        } else {
            for origin in CLIENT_ORIGINS {
                catalog.register_config(origin.declaration, origin.symbol);
                catalog.register_annotation_origin(*origin).unwrap();
            }
        }
    }
    catalog
}

#[test]
fn all_server_annotations_bind_through_the_real_barrel_in_both_orders() {
    let mut graph = surface();
    add(
        &mut graph,
        30,
        APP,
        &format!("import '{BARREL}' as server; import '{CLIENT}' as client;"),
    );
    for reverse in [false, true] {
        let catalog = owners(reverse);
        for (_, names) in &GROUPS[..4] {
            for origin in *names {
                let name = origin.declaration;
                assert_eq!(
                    graph.scope(APP).bind(&format!("server.{name}"), &catalog),
                    NamespaceBinding::Owned(SymbolId::new(format!("dust_server::{name}")))
                );
                if catalog
                    .resolve_qualified_config(&format!("dust_dart::{name}"))
                    .is_some()
                {
                    assert_eq!(
                        graph.scope(APP).bind(&format!("client.{name}"), &catalog),
                        NamespaceBinding::Owned(SymbolId::new(format!("dust_dart::{name}")))
                    );
                }
            }
        }
    }
}

#[test]
fn unprefixed_client_server_collisions_fail_in_both_orders() {
    let mut graph = surface();
    add(
        &mut graph,
        30,
        APP,
        &format!("import '{BARREL}'; import '{CLIENT}';"),
    );
    for reverse in [false, true] {
        let catalog = owners(reverse);
        for (_, names) in &GROUPS[..4] {
            for origin in *names {
                let name = origin.declaration;
                if catalog
                    .resolve_qualified_config(&format!("dust_dart::{name}"))
                    .is_some()
                {
                    let diagnostic =
                        rejection(graph.scope(APP).bind(name, &catalog), Failure::Ambiguous);
                    assert_eq!(
                        diagnostic.message,
                        format!("annotation `{name}` is ambiguous between distinct declarations")
                    );
                    assert_eq!(diagnostic.labels.len(), 2);
                }
            }
        }
    }
}
