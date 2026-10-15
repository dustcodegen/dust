//! What the engine knows about one database, in one place.
//!
//! Emit, validation, and the escape hatch each used to `match` on the driver at
//! the point of use, so adding a database meant finding every arm and missing
//! one meant generated code naming a type from the wrong driver. A dialect is a
//! value instead, and [`DIALECTS`] is the registry every lookup reads.
//!
//! # Adding a database
//!
//! In the engine:
//!
//! 1. A `Dialect` const here, and an entry in [`DIALECTS`].
//! 2. A `DbDriver` variant in `model.rs`, and a `DbDriverIr` variant in
//!    `dust_ir`, so the resolver can carry it.
//! 3. A type table in `column_types.rs`, named by the dialect's
//!    `accepted_sql_types`.
//! 4. An arm in `validate/sqlx.rs`, which is the one place that has to name a
//!    concrete `sqlx` backend. Everything past the connection is generic.
//!
//! Nothing else in the engine matches on the driver: names, aliases, emitted
//! Dart types and URL schemes all come from the dialect.
//!
//! Outside the engine, a database also needs a Dart runtime package
//! implementing `Executor`, `Connection`, `Transaction`, `Pool` and `Row` from
//! `dust_dart`, a `Driver` enum member there, and the package added to
//! workspace discovery, the compatibility contract, and the per-package lists
//! in `scripts/dart/`, CI and Sonar.
//!
//! `driver_tests::compatibility` checks the first three of those agree with
//! this registry, because they silently did not once already.

use super::{column_types, model::DbDriver};

/// Native worker types supported by a dialect's opt-in async facade.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub(crate) struct BackgroundDialect {
    /// Driver owning opening, migrations, statements and close.
    pub(crate) runtime_type: &'static str,
    /// Unchecked asynchronous facade adapter, never a native handle.
    pub(crate) unsafe_type: &'static str,
}

/// One database, and the Dart runtime that talks to it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Dialect {
    /// The driver this describes.
    pub(crate) driver: DbDriver,
    /// Stable name used in analysis keys and query cache entries.
    ///
    /// This is the one spelling of the driver. Parsing an annotation, reading
    /// a packed analysis value and writing a cache entry all go through it, so
    /// a new dialect cannot be spelled one way in one place and another
    /// elsewhere.
    pub(crate) name: &'static str,
    /// Dart package providing this dialect's runtime.
    ///
    /// The engine does not emit the import — user source brings it in — but
    /// workspace discovery and the compatibility contract both have to know
    /// the package exists, and a test holds them to this list.
    pub(crate) runtime_package: &'static str,
    /// Extra spellings `@SqlxDatabase(type: ...)` accepts for this dialect.
    ///
    /// `Driver.sqlite3` and `SqlxDatabaseType.sqlite` name the same database.
    /// Aliases live with the dialect rather than in a parser-side match.
    pub(crate) annotation_aliases: &'static [&'static str],
    /// SQL types each Dart type may be read from, if the dialect has an opinion.
    ///
    /// A dialect brings its own table, so adding a database does not mean
    /// finding a `match` in the type checker.
    pub(crate) accepted_sql_types: fn(&str) -> Option<&'static [&'static str]>,
    /// Dart type the generated facade holds.
    pub(crate) runtime_type: &'static str,
    /// Constructor the runtime type offers.
    pub(crate) factory: &'static str,
    /// Parameter that constructor takes, as it appears in Dart source.
    ///
    /// SQLite opens a file and Postgres connects to a server, and the facade's
    /// signature follows the database rather than pretending they are the same.
    pub(crate) factory_parameter: &'static str,
    /// Dart type carrying per-connection settings.
    pub(crate) options_type: &'static str,
    /// Dart type implementing the unchecked SQL escape hatch.
    pub(crate) unsafe_type: &'static str,
    /// Optional background facade support; absent dialects stay unchanged.
    pub(crate) background: Option<BackgroundDialect>,
    /// Dart expression applying migrations from the generated facade.
    pub(crate) migrate_expr: &'static str,
    /// Whether the driver rewrites `$n` into another placeholder form.
    ///
    /// SQLite has no `$n`, so its driver rewrites to `?` at bind time and a
    /// repeated `$1` becomes two binds. PostgreSQL reads `$n` itself and binds
    /// a repeated one once. The engine has to describe the text the database
    /// will actually receive, and expect the bind count that dialect implies.
    pub(crate) rewrites_placeholders: bool,
    /// Whether the dialect's nullability inference is worth reporting.
    ///
    /// SQLite describes a `PRIMARY KEY` column as nullable, so checking its
    /// inference warns about correct code: `fixtures/server_app` produced five
    /// such warnings, every one of them wrong. PostgreSQL describes the same
    /// shapes accurately. The column-alias overrides are what a dialect with
    /// untrustworthy inference leaves callers, and they work either way.
    pub(crate) checks_nullability: bool,
    /// URL schemes `DUST_DATABASE_URL` may use for this dialect.
    ///
    /// One workspace can hold projects on both drivers, and one environment
    /// variable names one database. Without this, a SQLite project handed a
    /// PostgreSQL URL tries to open it as a file and reports whatever the
    /// other driver's URL parser disliked.
    pub(crate) url_schemes: &'static [&'static str],
    /// Whether SQL for this dialect is checked against the schema at build time.
    ///
    /// False means the runtime works but `describe` does not, so queries reach
    /// the database unchecked and the build warns rather than refusing.
    pub(crate) validates: bool,
}

/// SQLite through `package:sqlite3`.
const SQLITE3: Dialect = Dialect {
    driver: DbDriver::Sqlite3,
    name: "sqlite3",
    runtime_package: "dust_db_sqlite3",
    annotation_aliases: &["sqlite"],
    accepted_sql_types: column_types::sqlite_types,
    runtime_type: "Sqlite3Driver",
    factory: "open",
    factory_parameter: "String path",
    options_type: "SqliteConnectOptions",
    unsafe_type: "Sqlite3UnsafeSql",
    background: Some(BackgroundDialect {
        runtime_type: "Sqlite3WorkerDriver",
        unsafe_type: "Sqlite3WorkerUnsafeSql",
    }),
    // Applied while opening, so there is nothing left to do.
    migrate_expr: "Future<Result<Unit, SqlxError>>.value(const Ok(unit))",
    rewrites_placeholders: true,
    checks_nullability: false,
    url_schemes: &["sqlite"],
    validates: true,
};

/// PostgreSQL through `package:postgres`.
const POSTGRES: Dialect = Dialect {
    driver: DbDriver::Postgres,
    name: "postgres",
    runtime_package: "dust_db_postgres",
    annotation_aliases: &["postgresql"],
    accepted_sql_types: column_types::postgres_types,
    runtime_type: "PostgresDriver",
    factory: "connect",
    factory_parameter: "String url",
    options_type: "PgConnectOptions",
    unsafe_type: "PostgresUnsafeSql",
    background: None,
    migrate_expr: "_driver.migrate()",
    rewrites_placeholders: false,
    checks_nullability: true,
    url_schemes: &["postgres", "postgresql"],
    validates: true,
};

/// Every dialect the engine supports.
///
/// The registry, not a `match` at each point of use: lookups by name and by
/// annotation spelling read this, so a new dialect is one const and one entry.
pub(crate) const DIALECTS: &[&Dialect] = &[&SQLITE3, &POSTGRES];

/// Dart packages providing a database runtime this engine can target.
///
/// Workspace discovery and the CLI compatibility contract each keep their own
/// list of Dust runtime packages, and both have to include these. Exposing the
/// registry lets a test say so rather than leaving the lists to drift.
pub fn database_runtime_packages() -> Vec<&'static str> {
    DIALECTS
        .iter()
        .map(|dialect| dialect.runtime_package)
        .collect()
}

impl DbDriver {
    /// Returns everything the engine knows about this driver.
    pub(crate) const fn dialect(self) -> &'static Dialect {
        match self {
            Self::Sqlite3 => &SQLITE3,
            Self::Postgres => &POSTGRES,
        }
    }

    /// Returns the stable driver name used in analysis keys and cache entries.
    pub(crate) const fn as_str(self) -> &'static str {
        self.dialect().name
    }
}

impl Dialect {
    /// Looks a dialect up by its canonical name.
    pub(crate) fn from_name(name: &str) -> Option<&'static Dialect> {
        DIALECTS
            .iter()
            .copied()
            .find(|dialect| dialect.name == name)
    }

    /// Looks a dialect up by any spelling an annotation may use.
    ///
    /// Takes the last dotted segment, so `Driver.sqlite3`, `sqlite3` and
    /// `SqlxDatabaseType.sqlite` all arrive here as a bare word.
    pub(crate) fn from_annotation(source: &str) -> Option<&'static Dialect> {
        let member = source.trim().rsplit('.').next()?;
        DIALECTS
            .iter()
            .copied()
            .find(|dialect| dialect.name == member || dialect.annotation_aliases.contains(&member))
    }

    /// Whether `url` names a database this dialect can validate against.
    ///
    /// A URL with no scheme is a bare SQLite path, which only SQLite accepts.
    pub(crate) fn accepts_url(&self, url: &str) -> bool {
        match url.split_once("://") {
            Some((scheme, _)) => self
                .url_schemes
                .iter()
                .any(|known| scheme.eq_ignore_ascii_case(known)),
            None => self.driver == DbDriver::Sqlite3,
        }
    }
}

#[cfg(test)]
#[path = "dialect/tests.rs"]
mod tests;
