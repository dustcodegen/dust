//! Regression spike for the namespace-safe ownership prerequisite of server codegen.

use dust_ir::SymbolId;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText};

#[test]
fn canonical_owners_survive_client_first_registration() {
    assert_canonical_owners(["dust_dart", "dust_server"]);
}

#[test]
fn canonical_owners_survive_server_first_registration() {
    assert_canonical_owners(["dust_server", "dust_dart"]);
}

fn assert_canonical_owners(owners: [&str; 2]) {
    let mut catalog = SymbolCatalog::new();
    for owner in owners {
        catalog.register_config("GET", format!("{owner}::GET"));
    }
    for owner in owners {
        let name = format!("{owner}::GET");
        assert_eq!(
            catalog
                .resolve_qualified_config(&name)
                .map(|entry| &entry.symbol),
            Some(&SymbolId::new(&name)),
            "registration order: {owners:?}",
        );
    }
}

#[test]
fn prefixed_owners_resolve_with_client_first_registration() {
    assert_prefixed_owners(["dust_dart", "dust_server"]);
}

#[test]
fn prefixed_owners_resolve_with_server_first_registration() {
    assert_prefixed_owners(["dust_server", "dust_dart"]);
}

fn assert_prefixed_owners(owners: [&str; 2]) {
    let source = SourceText::new(
        FileId::new(84),
        r#"
import 'package:dust_dart/http.dart' as client;
import 'package:dust_server/server.dart' as server;
part 'api.g.dart';

@client.HttpClient()
abstract class ClientApi {
  @client.GET('/notes')
  Future<String> notes();
}

@server.Controller('/notes')
class NotesController {
  @server.GET('/')
  String notes() => 'notes';
}
"#,
    );
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("HttpClient", "dust_dart::HttpClient");
    catalog.register_config("Controller", "dust_server::Controller");
    for owner in owners {
        catalog.register_config("GET", format!("{owner}::GET"));
    }
    let resolved = resolve_library(
        FileId::new(84),
        "lib/api.dart",
        "lib/api.g.dart",
        &parsed.library,
        &catalog,
    );
    assert_eq!(resolved.diagnostics, []);
    let names = resolved
        .library
        .classes
        .iter()
        .map(|class| class.methods[0].configs[0].symbol.0.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        ["dust_dart::GET", "dust_server::GET"],
        "registration order: {owners:?}"
    );
}
