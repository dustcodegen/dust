use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use dust_parser_dart::{ParseOptions, ParseResult, parse_file_with_backend};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::{FileId, SourceText};

use super::Source;

/// The source text and generated-file classification used by one parse.
pub(super) struct SourceInput {
    /// Dart text, empty for generated placeholders.
    pub(super) text: Arc<str>,
    /// Declaring root for a generated primary part.
    pub(super) owner: Option<String>,
    /// Whether a generated library was replaced by a placeholder.
    pub(super) generated_library: bool,
}

impl SourceInput {
    /// Reads handwritten text or prepares an empty generated placeholder.
    pub(super) fn read(
        source: &Source,
        owner: Option<String>,
        potential: &BTreeSet<PathBuf>,
    ) -> Result<Self, String> {
        let mut generated_library = false;
        let text = if owner.is_some() {
            String::new()
        } else {
            let text = match fs::read_to_string(&source.path) {
                Ok(text) => text,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && potential.contains(&source.path) =>
                {
                    generated_library = true;
                    String::new()
                }
                Err(error) => {
                    return Err(format!(
                        "cannot read namespace source `{}`: {error}",
                        source.path.display()
                    ));
                }
            };
            generated_library |= crate::generated_source::is_dust_generated_source(&text);
            if generated_library {
                String::new()
            } else {
                text
            }
        };
        Ok(Self {
            text: text.into(),
            owner,
            generated_library,
        })
    }

    /// Extracts parser facts independently of directive resolution and the graph.
    pub(super) fn parse(&self, source: &Source, id: FileId) -> ParseResult {
        parse_file_with_backend(
            &TreeSitterDartBackend::new(),
            &SourceText::new(id, Arc::clone(&self.text)),
            ParseOptions {
                language_version: source.language_version,
                ..Default::default()
            },
        )
    }
}
