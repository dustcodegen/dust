use dust_ir::{MethodIr, TypeIr};

use super::*;

fn background_class(is_static: bool) -> ClassIr {
    let mut class = class("AppDatabase");
    class.methods.push(MethodIr {
        name: "openBackground".to_owned(),
        is_static,
        is_external: false,
        return_type: TypeIr::generic("Future", vec![TypeIr::named("AppDatabase")]),
        has_body: true,
        body_source: None,
        params: Vec::new(),
        span: span(),
        traits: Vec::new(),
        configs: Vec::new(),
    });
    class
}

#[test]
fn emits_background_opener_only_for_opted_in_sqlite() {
    let class = background_class(true);
    let library = library(Path::new(""), vec![class.clone()]);
    let database = DatabaseClass {
        class: &class,
        driver: DbDriver::Sqlite3,
        migrations: "migrations".to_owned(),
    };
    assert_eq!(
        render_database_class(&library, &database),
        EXPECTED_BACKGROUND
    );
}

#[test]
fn static_name_does_not_change_postgres_output() {
    let class = background_class(true);
    let library = library(Path::new(""), vec![class.clone()]);
    let database = DatabaseClass {
        class: &class,
        driver: DbDriver::Postgres,
        migrations: "migrations".to_owned(),
    };
    assert_eq!(
        render_database_class(&library, &database),
        EXPECTED_POSTGRES_DATABASE
    );
}

#[test]
fn instance_method_does_not_opt_into_background_emission() {
    let class = background_class(false);
    let library = library(Path::new(""), vec![class.clone()]);
    let database = DatabaseClass {
        class: &class,
        driver: DbDriver::Sqlite3,
        migrations: "migrations".to_owned(),
    };
    let plain_class = super::class("AppDatabase");
    let plain = DatabaseClass {
        class: &plain_class,
        ..database.clone()
    };
    assert_eq!(
        render_database_class(&library, &database),
        render_database_class(&library, &plain)
    );
}

const EXPECTED_BACKGROUND: &str = r#"final class _$AppDatabase implements AppDatabase {
  _$AppDatabase._(this._driver, this._unsafe);

  factory _$AppDatabase.open(
    String path, {
    SqliteConnectOptions? options,
  }) {
    final driver = Sqlite3Driver.open(
      path,
      migrations: _$appDatabaseMigrations,
      options: options,
    );
    return _$AppDatabase._(driver, Sqlite3UnsafeSql(driver));
  }

  static Future<AppDatabase> openBackground(
    String path, {
    SqliteConnectOptions? options,
  }) async {
    final driver = await Sqlite3WorkerDriver.open(
      path,
      migrations: _$appDatabaseMigrations,
      options: options,
    );
    return _$AppDatabase._(driver, Sqlite3WorkerUnsafeSql(driver));
  }

  final Pool _driver;
  final UnsafeSql _unsafe;

  @override
  Connection get connection => _driver;

  @override
  Future<Result<Unit, SqlxError>> migrate() => Future<Result<Unit, SqlxError>>.value(const Ok(unit));

  @override
  UnsafeSql get unsafe => _unsafe;

  Pool get pool => _driver;
}

const Map<String, String> _$appDatabaseMigrations = <String, String>{};"#;
