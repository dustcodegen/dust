use dust_plugin_api::AnnotationOrigin;

/// Both classes and legacy typedefs are declared here, not in the DB barrel.
const LIBRARY: &str = "package:dust_dart/src/db/annotations.dart";

/// The first two records are exactly the row-only profile's claims.
pub(super) const ORIGINS: &[AnnotationOrigin] = &[
    AnnotationOrigin::new("dust_dart::FromRow", LIBRARY, "FromRow"),
    AnnotationOrigin::new("dust_dart::Sqlx", LIBRARY, "Sqlx"),
    AnnotationOrigin::new("dust_dart::Database", LIBRARY, "Database"),
    AnnotationOrigin::new("dust_dart::SqlxDatabase", LIBRARY, "SqlxDatabase"),
    AnnotationOrigin::new("dust_dart::Dao", LIBRARY, "Dao"),
    AnnotationOrigin::new("dust_dart::SqlxDao", LIBRARY, "SqlxDao"),
    AnnotationOrigin::new("dust_dart::Query", LIBRARY, "Query"),
];
