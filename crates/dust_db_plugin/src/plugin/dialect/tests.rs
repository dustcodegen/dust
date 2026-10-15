use super::*;

/// A copied dialect that kept another driver's type names would emit code
/// naming the wrong package, and nothing else would catch it.
#[test]
fn every_driver_names_a_distinct_runtime() {
    for (index, left) in DIALECTS.iter().enumerate() {
        for right in DIALECTS.iter().skip(index + 1) {
            assert_ne!(left.name, right.name);
            assert_ne!(left.runtime_type, right.runtime_type);
            assert_ne!(left.unsafe_type, right.unsafe_type);
            assert_ne!(left.migrate_expr, right.migrate_expr);
            assert_ne!(left.options_type, right.options_type);
            assert_ne!(left.runtime_package, right.runtime_package);
        }
    }
}

/// One `DUST_DATABASE_URL` and two drivers in a workspace: each project
/// has to recognise the URL that is not for it, rather than trying to open
/// it and reporting the other driver's parse error.
#[test]
fn a_dialect_only_accepts_its_own_urls() {
    let sqlite = DbDriver::Sqlite3.dialect();
    let postgres = DbDriver::Postgres.dialect();

    assert!(sqlite.accepts_url("sqlite://app.db"));
    assert!(sqlite.accepts_url("app.db"));
    assert!(!sqlite.accepts_url("postgres://user@localhost/app?sslmode=disable"));

    assert!(postgres.accepts_url("postgres://user@localhost/app"));
    assert!(postgres.accepts_url("POSTGRESQL://user@localhost/app"));
    assert!(!postgres.accepts_url("sqlite://app.db"));
    assert!(!postgres.accepts_url("app.db"));
}

/// Two dialects answering to one spelling would make `@Database(driver:)`
/// mean whichever came first in the registry.
#[test]
fn no_two_dialects_answer_to_the_same_annotation_spelling() {
    let mut seen = Vec::new();
    for dialect in DIALECTS {
        for spelling in std::iter::once(&dialect.name).chain(dialect.annotation_aliases) {
            assert!(
                !seen.contains(spelling),
                "`{spelling}` names more than one dialect"
            );
            seen.push(spelling);
        }
        assert!(!dialect.runtime_package.is_empty());
    }
}

#[test]
fn an_annotation_resolves_through_either_enum_spelling() {
    for source in ["Driver.sqlite3", "SqlxDatabaseType.sqlite", "sqlite3"] {
        assert_eq!(
            Dialect::from_annotation(source).map(|dialect| dialect.driver),
            Some(DbDriver::Sqlite3),
            "{source}"
        );
    }
    for source in ["Driver.postgres", "SqlxDatabaseType.postgres", "postgresql"] {
        assert_eq!(
            Dialect::from_annotation(source).map(|dialect| dialect.driver),
            Some(DbDriver::Postgres),
            "{source}"
        );
    }
    assert!(Dialect::from_annotation("Driver.mysql").is_none());
    assert!(Dialect::from_name("mysql").is_none());
}

#[test]
fn driver_names_round_trip_through_analysis() {
    assert_eq!(DbDriver::Sqlite3.as_str(), "sqlite3");
    assert_eq!(DbDriver::Postgres.as_str(), "postgres");
}
