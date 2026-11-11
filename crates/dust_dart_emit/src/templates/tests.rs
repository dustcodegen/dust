use super::*;

#[test]
fn template_name_source_and_context_remain_independent() {
    let context = |name| BTreeMap::from([("name", name)]);
    assert_eq!(
        render_template("reused", "{{ name }}\n\n", context("first")),
        "first"
    );
    assert_eq!(
        render_template("reused", "{{ name }}\n\n", context("second")),
        "second"
    );
    assert_eq!(
        render_template("reused", "other {{ name }}", context("third")),
        "other third"
    );
    assert_eq!(
        render_template("another", "{{ name }}\n\n", context("🦀")),
        "🦀"
    );
}

#[test]
fn invalid_template_does_not_poison_the_worker_cache() {
    let invalid = std::panic::catch_unwind(|| render_template("invalid", "{% if true %}", ()));
    assert!(invalid.is_err());
    assert_eq!(render_template("invalid", "valid", ()), "valid");
}

/// A serializer can itself render without borrowing the outer template cache.
struct Nested;
impl Serialize for Nested {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = render_template("nested", "{{ name }}", BTreeMap::from([("name", "inside")]));
        serializer.serialize_str(&value)
    }
}

#[test]
fn serialization_can_render_another_template_and_workers_stay_independent() {
    std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    render_template("outer", "{{ value }}", BTreeMap::from([("value", Nested)]))
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), "inside");
        }
    });
}
