use super::{
    database::{create_engine, migrate_database, run},
    migrations::REVISIONS,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::path::Path;

fn execute(db: &DatabaseConnection, sql: &str) {
    run(db.execute_unprepared(sql)).unwrap();
}
fn scalar(db: &DatabaseConnection, sql: &str) -> String {
    run(db.query_one(Statement::from_string(DbBackend::Sqlite, sql))).unwrap().unwrap().try_get("", "value").unwrap()
}
fn legacy(path: &Path, count: usize) -> DatabaseConnection {
    let db = create_engine(Some(path)).unwrap();
    execute(&db, include_str!("../../../src/matis/persistence/migrations/alembic_version.sql"));
    for &(_, sql) in &REVISIONS[..count] {
        execute(&db, sql);
    }
    execute(
        &db,
        &format!(
            "INSERT INTO alembic_version VALUES('{}'); INSERT INTO repositories VALUES('id','Coffee','/coffee',123)",
            REVISIONS[count - 1].0
        ),
    );
    let artifact = if count < 4 { ",artifact" } else { "" };
    let value = if count < 4 { ",'legacy payload'" } else { "" };
    execute(
        &db,
        &format!("INSERT INTO repository_graphs(repository_id,imported_at{artifact}) VALUES('id',456{value})"),
    );
    if count >= 2 {
        execute(&db, "UPDATE repository_graphs SET graph_revision='revision',schema_version=2,indexed_at='date',root_name='Coffee',root_path='/coffee';
            INSERT INTO graph_nodes VALUES('id','file',NULL,0,'file','file','file','coffee.py','python',NULL,'coffee.py');
            INSERT INTO graph_files VALUES('id','file',12,'hash');
            INSERT INTO graph_symbols VALUES('id','file',1,2,3,4,'signature','public');");
    }
    if count >= 3 {
        execute(
            &db,
            "INSERT INTO graph_node_metrics VALUES('id','file',1,2,3,4,5,6,7);
            INSERT INTO graph_node_decorators VALUES('id','file',0,'decorator');
            INSERT INTO graph_edges VALUES('id',0,'file','file','CALLS','SYNTACTIC','coffee.py','evidence',1,0);
            INSERT INTO graph_diagnostics VALUES('id',0,'warning','coffee.py','python','message');",
        );
    }
    if count >= 5 {
        execute(
            &db,
            "INSERT INTO graph_mutation_candidates VALUES('id','candidate','file',0,'coffee.py','python','hash','binary',NULL,'{}')",
        );
    }
    db
}
fn snapshot(db: &DatabaseConnection) -> Vec<(String, String)> {
    let tables = run(db.query_all(Statement::from_string(DbBackend::Sqlite,
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('seaql_migrations','alembic_version') ORDER BY name"))).unwrap();
    tables
        .into_iter()
        .map(|row| {
            let table: String = row.try_get("", "name").unwrap();
            let columns =
                run(db.query_all(Statement::from_string(DbBackend::Sqlite, format!("PRAGMA table_info({table})"))))
                    .unwrap()
                    .into_iter()
                    .map(|r| r.try_get::<String>("", "name").unwrap())
                    .filter(|n| n != "artifact")
                    .collect::<Vec<_>>();
            let columns = if table == "repository_graphs" {
                [
                    "repository_id",
                    "imported_at",
                    "graph_revision",
                    "schema_version",
                    "indexed_at",
                    "root_name",
                    "root_path",
                ]
                .into_iter()
                .map(|n| if columns.iter().any(|c| c == n) { format!("\"{n}\"") } else { "NULL".into() })
                .collect::<Vec<_>>()
                .join(",")
            } else {
                columns.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(",")
            };
            let data = scalar(db, &format!("SELECT json_group_array(json_array({columns})) AS value FROM {table}"));
            (table, data)
        })
        .collect()
}
#[test]
fn adopts_every_alembic_revision_without_replaying_or_changing_rows() {
    for count in 1..=REVISIONS.len() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("legacy.sqlite");
        let db = legacy(&path, count);
        let before = snapshot(&db);
        migrate_database(Some(&path)).unwrap();
        let after = snapshot(&db);
        assert!(before.iter().all(|row| after.contains(row)), "revision {count}: {before:?}");
        assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), REVISIONS.last().unwrap().0);
        assert_eq!(scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM seaql_migrations"), "5");
        let tracking =
            scalar(&db, "SELECT json_group_array(json_array(version,applied_at)) AS value FROM seaql_migrations");
        migrate_database(Some(&path)).unwrap();
        assert_eq!(snapshot(&db), after);
        assert_eq!(
            scalar(&db, "SELECT json_group_array(json_array(version,applied_at)) AS value FROM seaql_migrations"),
            tracking
        );
        assert_eq!(scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM pragma_foreign_key_check"), "0");
    }
}
#[test]
fn migration_waits_for_an_existing_writer() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("locked.sqlite");
    let db = legacy(&path, 1);
    execute(&db, "PRAGMA journal_mode=WAL");
    let tx = run(db.get_sqlite_connection_pool().begin_with("BEGIN IMMEDIATE")).unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || send.send(migrate_database(Some(&path))).unwrap());
    let early = receive.recv_timeout(std::time::Duration::from_millis(250));
    run(tx.commit()).unwrap();
    worker.join().unwrap();
    assert!(matches!(early, Err(std::sync::mpsc::RecvTimeoutError::Timeout)), "{early:?}");
    receive.recv().unwrap().unwrap();
    assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), REVISIONS.last().unwrap().0);
    assert_eq!(scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM seaql_migrations"), "5");
}
#[test]
fn migration_lock_timeout_rolls_back_and_leaves_connection_reusable() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("timeout.sqlite");
    let writer = legacy(&path, 1);
    execute(&writer, "PRAGMA journal_mode=WAL");
    let db = create_engine(Some(&path)).unwrap();
    let before = snapshot(&db);
    execute(&db, "PRAGMA busy_timeout=10");
    let tx = run(writer.get_sqlite_connection_pool().begin_with("BEGIN IMMEDIATE")).unwrap();
    let error = run(super::migrations::migrate_with_timeout(&db, std::time::Duration::from_millis(100))).unwrap_err();
    run(tx.commit()).unwrap();
    assert!(error.to_string().contains("after 0.1 seconds"), "{error:#}");
    assert_eq!(snapshot(&db), before);
    assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), "0001_initial");
    assert_eq!(
        scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM sqlite_master WHERE name='seaql_migrations'"),
        "0"
    );
    run(super::migrations::migrate(&db)).unwrap();
    assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), REVISIONS.last().unwrap().0);
}
#[test]
fn fresh_migration_tracks_all_revisions_and_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fresh.sqlite");
    migrate_database(Some(&path)).unwrap();
    let db = create_engine(Some(&path)).unwrap();
    let before = snapshot(&db);
    migrate_database(Some(&path)).unwrap();
    assert_eq!(before, snapshot(&db));
    assert_eq!(
        scalar(
            &db,
            "SELECT group_concat(version,',') AS value FROM (SELECT version FROM seaql_migrations ORDER BY version)"
        ),
        REVISIONS.iter().map(|r| r.0).collect::<Vec<_>>().join(",")
    );
    assert_eq!(scalar(&db, "SELECT CAST(foreign_keys AS TEXT) AS value FROM pragma_foreign_keys"), "1");
}
#[test]
fn unknown_and_multiple_alembic_versions_leave_database_unchanged() {
    for versions in ["('unknown')", "('0001_initial'),('0002_graph_nodes')"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.sqlite");
        let db = create_engine(Some(&path)).unwrap();
        execute(&db, include_str!("../../../src/matis/persistence/migrations/alembic_version.sql"));
        execute(&db, &format!("INSERT INTO alembic_version VALUES {versions}"));
        let before = scalar(&db, "SELECT group_concat(version_num) AS value FROM alembic_version");
        let error = migrate_database(Some(&path)).unwrap_err().to_string();
        assert!(error.contains(if versions.contains("unknown") { "Unknown Alembic" } else { "Multiple Alembic" }));
        assert_eq!(scalar(&db, "SELECT group_concat(version_num) AS value FROM alembic_version"), before);
        assert_eq!(
            scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM sqlite_master WHERE name='seaql_migrations'"),
            "0"
        );
    }
}
#[test]
fn migration_failure_rolls_back_ddl_adoption_and_both_version_tables() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("failure.sqlite");
    let db = legacy(&path, 1);
    execute(&db, "CREATE TABLE graph_edges(blocker TEXT)");
    let before = snapshot(&db);
    assert!(migrate_database(Some(&path)).is_err());
    assert_eq!(snapshot(&db), before);
    assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), "0001_initial");
    assert_eq!(
        scalar(&db, "SELECT CAST(count(*) AS TEXT) AS value FROM sqlite_master WHERE name='seaql_migrations'"),
        "0"
    );
    execute(&db, "DROP TABLE graph_edges");
    migrate_database(Some(&path)).unwrap();
}
#[test]
fn rejects_unknown_or_ahead_seaorm_history() {
    for version in ["unknown", "0005_mutation_candidates"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.sqlite");
        let db = legacy(&path, 1);
        run(<super::migrations::Migrator as sea_orm_migration::MigratorTrait>::install(&db)).unwrap();
        execute(&db, &format!("INSERT INTO seaql_migrations VALUES('{version}',1)"));
        assert!(migrate_database(Some(&path)).is_err());
        assert_eq!(scalar(&db, "SELECT version_num AS value FROM alembic_version"), "0001_initial");
        assert_eq!(scalar(&db, "SELECT version AS value FROM seaql_migrations"), version);
    }
}
