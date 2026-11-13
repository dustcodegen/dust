use super::*;

/// Plans independent roots and merges errors/conflicts in original source order.
pub(in crate::namespace_sources) fn plan(
    roots: &[Source],
    registry: &PluginRegistry,
) -> Result<BTreeMap<PathBuf, Claim>, Diagnostic> {
    let groups = crate::build::round_robin_groups(
        roots.iter().enumerate(),
        crate::build::available_worker_count(roots.len(), None),
    );
    let mut planned = std::thread::scope(|scope| {
        let handles = groups
            .into_iter()
            .map(|group| {
                scope.spawn(move || {
                    group
                        .into_iter()
                        .map(|(order, root)| {
                            let outputs =
                                registry
                                    .potential_generated_libraries(&root.path)
                                    .map(|outputs| {
                                        outputs
                                            .into_iter()
                                            .map(|output| {
                                                (
                                                    crate::source_uri::normalize_path(&output.path),
                                                    Claim {
                                                        source: root.uri.clone(),
                                                        producer: output.producer,
                                                    },
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                    });
                            (order, outputs)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("output planning must not panic"))
            .collect::<Vec<_>>()
    });
    planned.sort_by_key(|(order, _)| *order);
    let mut claims = BTreeMap::new();
    for (_, outputs) in planned {
        for (path, claim) in outputs? {
            if let Some(previous) = claims.insert(path.clone(), claim)
                && claims[&path] != previous
            {
                return Err(Diagnostic::error(format!(
                    "generated library `{}` has conflicting producers",
                    path.display()
                )));
            }
        }
    }
    Ok(claims)
}
