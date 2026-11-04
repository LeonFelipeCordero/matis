//! Failed discovery must not replace a previously saved graph.
#![cfg(unix)]
use matis::{
    bootstrap::{migrate_database, repository_service},
    indexer::analyze_repository,
};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn unreadable_source_directory_preserves_saved_graph() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let root = workspace.path().join("repo");
    let src = root.join("src");
    fs::create_dir_all(&src)?;
    fs::write(src.join("main.py"), "def run():\n    return 42\n")?;
    let database = workspace.path().join("matis.sqlite");
    migrate_database(Some(&database))?;
    let mut service = repository_service(Some(&database))?;
    let saved = service.save_folder(&root, &analyze_repository(&root)?)?;
    let name = service.list()?.into_iter().find(|p| p.id == saved.repository_id).unwrap().name;
    let before = service.get_report_facts(&saved.repository_id)?;
    for mode in [0o000, 0o400] {
        fs::set_permissions(&src, fs::Permissions::from_mode(mode))?;
        let indexed = service.index(&name, &root);
        fs::set_permissions(&src, fs::Permissions::from_mode(0o700))?;
        assert!(indexed.is_err(), "permission failure must abort indexing: {mode:o}");
        assert!(format!("{:#}", indexed.unwrap_err()).contains("Cannot inspect path"));
        assert_eq!(service.get_architecture(&saved.repository_id)?.unwrap().graph_revision, saved.graph_revision);
        assert_eq!(service.get_report_facts(&saved.repository_id)?, before);
    }
    Ok(())
}
