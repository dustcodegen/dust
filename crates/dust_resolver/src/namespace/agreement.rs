use super::*;

/// Preserve a concrete failure before comparing target-independent ownership.
pub(super) fn agree(
    name: &str,
    bindings: &[NamespaceBinding],
    labels: impl FnOnce() -> Vec<SourceLabel>,
) -> Option<NamespaceBinding> {
    if let Some(failure) = bindings.iter().find(|binding| {
        matches!(binding, NamespaceBinding::Rejected { kind, .. }
            if *kind != NamespaceFailureKind::Hidden)
    }) {
        return Some(failure.clone());
    }
    let first = bindings.first()?;
    Some(if bindings.iter().all(|binding| binding == first) {
        first.clone()
    } else {
        rejected(
            NamespaceFailureKind::Conditional,
            format!("annotation `{name}` has target-dependent conditional ownership"),
            labels(),
        )
    })
}

/// Empty partial graphs must not manufacture a conditional ownership error.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_alternatives_have_no_binding_and_do_not_build_diagnostics() {
        assert_eq!(
            agree("marker", &[], || panic!("no conditional failure")),
            None
        );
    }
}
