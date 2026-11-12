use super::*;
use crate::mutations::{digest, report::build_mutation_report};

#[test]
fn saved_candidates_match_live_and_reject_stale_sources() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("brew.py");
    let bytes = "# café\r\ndef brew(a):\r\n    return (a + 2) * 3\r\n".as_bytes();
    std::fs::write(&path, bytes)?;
    std::fs::create_dir(root.path().join("nested"))?;
    for (name, content) in [
        ("a.js", "function brew(a) { return a + 2; }\n"),
        ("nested/z.rs", "fn brew(a: i32) -> i32 { a + 2 }\n"),
        (".gitignore", "git_ignored.py\n"),
        (".matisignore", "matis_ignored.py\n"),
        ("nested/.gitignore", "skip.py\n"),
        ("git_ignored.py", "x = 1 + 2\n"),
        ("matis_ignored.py", "x = 1 + 2\n"),
        ("nested/skip.py", "x = 1 + 2\n"),
        ("unsupported.txt", "x = 1 + 2\n"),
    ] {
        std::fs::write(root.path().join(name), content)?;
    }
    let config = config::MutationConfig { rules: vec!["arithmetic.add-to-subtract".into()], ..Default::default() };
    std::fs::write(root.path().join("matis.json"), serde_json::to_vec(&json!({"mutations": config}))?)?;
    let artifact = crate::indexer::analyze_repository(root.path())?;
    let full_report = crate::metrics::report::build_artifact_report(&artifact)?;
    let live = collect_candidates(root.path())?;
    assert_eq!(json!(live), artifact["mutationCandidates"]);
    assert_eq!(json!(live), full_report["mutationCandidates"]);
    assert_eq!(
        live.iter().map(|item| item["path"].as_str().unwrap()).collect::<Vec<_>>(),
        ["a.js", "brew.py", "brew.py", "nested/z.rs"]
    );
    assert_eq!(live[1]["sourceHash"], digest(bytes));
    let expected = build_mutation_report(root.path(), &full_report, &config)?;
    assert_eq!(expected["mutations"].as_array().unwrap().len(), 3);
    let options = super::tests::options(&["--folder".into(), root.path().to_string_lossy().into_owned()])?;
    assert_eq!(output(&options)?, expected);
    let db = root.path().join("graph.sqlite");
    assert!(!db.exists());
    crate::bootstrap::migrate_database(Some(&db))?;
    let mut service = crate::bootstrap::repository_service(Some(&db))?;
    service.save_folder(root.path(), &artifact)?;
    let saved = service.saved_folder_candidates(root.path())?;
    assert_eq!(saved, live);
    assert_eq!(build_candidate_report(root.path(), &saved, &config)?, expected);
    let project = service.list()?.remove(0);
    assert_eq!(build_mutation_report(root.path(), &service.report(&project.name)?, &config)?, expected);
    assert_eq!(std::fs::read(&path)?, bytes);
    std::fs::write(&path, b"# changed\n")?;
    let error = build_candidate_report(root.path(), &saved, &config::MutationConfig::default()).unwrap_err();
    assert!(error.to_string().starts_with("Stale or invalid mutation candidate"));
    assert_eq!(output(&options)?["mutations"].as_array().unwrap().len(), 2);
    Ok(())
}

#[test]
fn candidate_discovery_validates_roots_and_configuration_before_boundaries() -> Result<()> {
    let root = tempfile::tempdir()?;
    assert!(collect_candidates(root.path())?.is_empty());
    let missing = root.path().join("missing");
    assert!(collect_candidates(&missing).unwrap_err().to_string().starts_with("Repository does not exist:"));
    std::fs::write(root.path().join(".gitignore"), b"\xff")?;
    assert!(collect_candidates(root.path()).is_err());
    std::fs::write(root.path().join("matis.json"), r#"{"mutations":{"rules":["unknown"]}}"#)?;
    for saved in [false, true] {
        let mut args = vec!["--folder".into(), root.path().to_string_lossy().into_owned()];
        if saved {
            args.push("--saved".into());
        }
        assert!(output(&super::tests::options(&args)?).unwrap_err().to_string().contains("Unknown mutation rule"));
    }
    Ok(())
}
