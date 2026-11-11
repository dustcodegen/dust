use std::{
    fs,
    path::{Path, PathBuf},
};

use dust_parser_dart::{DartLanguageVersion, ParseBackend, ParseOptions, ParseResult};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::{FileId, SourceText};
use serde_json::Value;

use super::*;
use crate::namespace_sources::{
    NamespaceSources,
    index::{Source, SourceIndex},
    test_workspace,
};

/// Exact input and parser-policy identity are required before facts may be reused.
mod identity;
/// Disposable snapshots reject malformed, incomplete and internally invalid bytes.
mod payload;
/// Current resolution, generation and diagnostic identities remain outside the cache.
mod pipeline;
/// File kinds and atomic publication cannot redirect writes into source files.
mod storage;

const MODEL: &str =
    "class Model<T> { Model(this.value); final T value; T method(T next) => next; }";

struct Fixture {
    root: tempfile::TempDir,
    source: Source,
    options: ParseOptions,
    text: String,
    annotations: Vec<&'static str>,
}

/// Owned copies make controlled mutations while production framing stays shared.
#[derive(Clone)]
struct StoredRecord {
    uri: String,
    options: format::Options,
    full: bool,
    text: Vec<u8>,
    library: Vec<u8>,
}

impl Fixture {
    fn new(text: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("pubspec.yaml"),
            "name: parser_cache_test\n",
        )
        .unwrap();
        test_workspace::prepare(root.path(), root.path(), "parser_cache_test");
        test_workspace::set_language_version(root.path(), "parser_cache_test", "3.13");
        fs::create_dir_all(root.path().join("lib")).unwrap();
        let path = root.path().join("lib/source.dart");
        fs::write(&path, text).unwrap();
        let source = index(root.path()).file(&path).unwrap();
        let options = ParseOptions {
            language_version: source.language_version,
            ..Default::default()
        };
        let annotations = registry().all_supported_annotations();
        let fixture = Self {
            root,
            source,
            options,
            text: text.into(),
            annotations,
        };
        fixture.save();
        fixture
    }

    fn path(&self) -> PathBuf {
        path(self.root.path())
    }

    fn cache(&self) -> Cache {
        Cache::load(self.root.path(), &self.annotations)
    }

    fn lookup(&self) -> Option<ParseResult> {
        self.cache()
            .lookup(&self.source, &self.text, self.options, true)
    }

    fn save(&self) {
        save(
            &namespaces(self.root.path()),
            self.root.path(),
            &self.annotations,
        )
        .unwrap();
    }

    fn snapshot(&self) -> (format::Header, Vec<StoredRecord>) {
        let bytes = fs::read(self.path()).unwrap();
        let frame = frame::index(&bytes).expect("saved snapshot has checked framing");
        let header = format::decode(&bytes[frame.header]).unwrap();
        let records = frame
            .records
            .into_iter()
            .map(|(uri, record)| StoredRecord {
                uri,
                options: record.options,
                full: record.full,
                text: bytes[record.text].to_vec(),
                library: bytes[record.library].to_vec(),
            })
            .collect();
        (header, records)
    }

    fn uris(&self) -> Vec<String> {
        self.snapshot()
            .1
            .into_iter()
            .map(|record| record.uri)
            .collect()
    }

    fn rewrite(&self, edit: impl FnOnce(&mut format::Header, &mut Vec<StoredRecord>)) {
        let (mut header, mut records) = self.snapshot();
        edit(&mut header, &mut records);
        fs::write(self.path(), encode(&header, &records)).unwrap();
    }

    fn fresh(&self, id: FileId) -> ParseResult {
        TreeSitterDartBackend::new()
            .parse_file(&SourceText::new(id, self.text.clone()), self.options)
    }
}

fn registry() -> dust_plugin_api::PluginRegistry {
    crate::build::registry_for_selection(crate::build::RegistrySelection::All)
}

fn index(root: &Path) -> SourceIndex {
    SourceIndex::load(
        root,
        &root.join(".dart_tool/package_config.json"),
        Some(&root.join(".dust-test-sdk")),
        false,
    )
    .unwrap()
}

fn namespaces(root: &Path) -> NamespaceSources {
    let mut workspace = dust_workspace::load_workspace(root).unwrap();
    let registry = registry();
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    NamespaceSources::load(
        root,
        &mut workspace,
        Some(&root.join(".dust-test-sdk")),
        &catalog,
        &registry,
        false,
        Some(1),
    )
    .unwrap()
}

fn root_record<'a>(records: &'a mut [StoredRecord], uri: &str) -> &'a mut StoredRecord {
    records.iter_mut().find(|record| record.uri == uri).unwrap()
}

/// Use the same checked field widths and checksum as the production writer.
fn encode(header: &format::Header, records: &[StoredRecord]) -> Vec<u8> {
    let mut bytes = format::begin(header, records.len()).unwrap();
    for record in records {
        format::append_record(
            &mut bytes,
            &record.uri,
            record.options,
            record.full,
            &record.text,
            &record.library,
        );
    }
    format::seal(&mut bytes);
    bytes
}

/// Resealing makes structural mutations test more than the checksum guard.
fn reseal(bytes: &mut Vec<u8>) {
    bytes.truncate(bytes.len().checked_sub(8).unwrap());
    format::seal(bytes);
}
