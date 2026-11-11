use super::*;

impl PreparedParse {
    /// Builds immutable facts with neutral labels for either preparation path.
    pub(in crate::namespace_sources::loader) fn new(
        index: &SourceIndex,
        source: &Source,
        input: SourceInput,
        policy: &ParsePolicy,
    ) -> Self {
        let parsed = input.parse(source, PREPARED_FILE_ID, policy);
        let targets = resolve_targets(index, source, &parsed.library);
        Self {
            input,
            parsed,
            targets,
        }
    }

    /// Validates the input and installs the canonical ID on both diagnostic views.
    pub(in crate::namespace_sources::loader) fn rebind(
        self,
        source: &Source,
        input: &SourceInput,
        id: FileId,
    ) -> Option<(ParseResult, ResolvedTargets)> {
        let Self {
            input: before,
            mut parsed,
            targets,
        } = self;
        if before != *input || parsed.options != SourceInput::options(source) {
            return None;
        }
        for diagnostic in parsed
            .diagnostics
            .iter_mut()
            .chain(&mut parsed.namespace_diagnostics)
        {
            for label in &mut diagnostic.labels {
                label.file_id = id;
            }
        }
        Some((parsed, targets))
    }
}
