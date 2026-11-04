use super::*;

#[test]
fn saved_candidates_match_live_and_reject_stale_sources() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("brew.py");
    let bytes = b"def brew(a):\n    return a + 2\n";
    std::fs::write(&path, bytes)?;
    let source = crate::domain::models::SourceFile {
        path: "brew.py".into(),
        absolute_path: path.clone(),
        language: "python".into(),
        source: bytes.to_vec(),
    };
    let found = crate::parsing::mutation_candidates::extract_candidates(&source)?;
    let artifact = json!({"schemaVersion": 2, "repository": {"name": "coffee", "path": root.path()}, "edges": [], "diagnostics": [],
        "nodes": [{"stableKey": "file:brew.py", "type": "file", "kind": "file", "level": "file", "name": "brew.py", "path": "brew.py", "language": "python", "properties": {}}],
        "mutationCandidates": found});
    let db = root.path().join("graph.sqlite");
    crate::bootstrap::migrate_database(Some(&db))?;
    let mut service = crate::bootstrap::repository_service(Some(&db))?;
    service.save_folder(root.path(), &artifact)?;
    let project = service.list()?.remove(0);
    let saved = service.report(&project.name)?;
    let config = config::MutationConfig { rules: vec!["arithmetic.add-to-subtract".into()], ..Default::default() };
    let expected = build_mutation_report(root.path(), &artifact, &config)?;
    assert_eq!(expected["mutations"].as_array().unwrap().len(), 1);
    assert_eq!(build_mutation_report(root.path(), &saved, &config)?, expected);
    assert_eq!(std::fs::read(&path)?, bytes);
    std::fs::write(&path, b"# changed\n")?;
    let error = build_mutation_report(root.path(), &saved, &config::MutationConfig::default()).unwrap_err();
    assert!(error.to_string().starts_with("Stale or invalid mutation candidate"));
    Ok(())
}
