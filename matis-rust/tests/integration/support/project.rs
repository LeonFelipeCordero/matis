//! A real Python project indexed into an isolated SQLite database.
use matis::{
    bootstrap::{migrate_database, repository_service},
    domain::repositories::GraphDto,
    indexer::analyze_repository,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

pub struct Project {
    _workspace: tempfile::TempDir,
    #[allow(dead_code)] // Not every integration test needs to edit the source.
    pub root: PathBuf,
    pub database: PathBuf,
    pub graph: Value,
    pub saved: GraphDto,
}

impl Default for Project {
    fn default() -> Self {
        Self::new("def brew():\n    return 1\n")
    }
}

impl Project {
    pub fn new(source: &str) -> Self {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("brew.py"), source).unwrap();
        let database = workspace.path().join("matis.sqlite");
        migrate_database(Some(&database)).unwrap();
        let graph = analyze_repository(&root).unwrap();
        let saved = repository_service(Some(&database)).unwrap().save_folder(&root, &graph).unwrap();
        Self { _workspace: workspace, root, database, graph, saved }
    }

    pub fn key(&self, name: &str) -> &str {
        self.graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["name"] == name && node["kind"] == "function")
            .unwrap()["stableKey"]
            .as_str()
            .unwrap()
    }
}
