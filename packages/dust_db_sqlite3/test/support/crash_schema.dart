import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

const crashMigrations = {
  '0001.sql': '''
CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, payload BLOB);
''',
  '0002.sql': '''
CREATE TABLE upgraded (id INTEGER PRIMARY KEY);
INSERT INTO upgraded VALUES (7);
INSERT INTO items VALUES (4, 'pending', zeroblob(262144));
UPDATE items SET name = 'changed' WHERE id = 1;
''',
};

SqliteConnectOptions crashOptions(String journal) => SqliteConnectOptions(
  journalMode: SqliteJournalMode.values.byName(journal),
  synchronous: SqliteSynchronousMode.full,
  busyTimeout: const Duration(seconds: 2),
  // Force uncommitted pages out of the cache; prevent automatic WAL checkpoint.
  pragmas: const {
    'cache_size': 1,
    'cache_spill': true,
    'wal_autocheckpoint': 0,
  },
);
