//! Reindexing replaces saved facts instead of accumulating stale files and connections.
use crate::support::project;
use matis::bootstrap::{navigation_service, repository_service};
use project::Project;
use serde_json::json;
use std::fs;

#[test]
fn changed_then_deleted_sources_replace_the_graph_without_registering_another_project() -> anyhow::Result<()> {
    let project = Project::new("def brew():\n    return 1 < 2\n\ndef serve():\n    return brew()\n");
    let mut service = repository_service(Some(&project.database))?;
    let registered = service.list()?.remove(0);
    let before = service.get_report_facts(&registered.id)?.unwrap();
    assert!(!before["edges"].as_array().unwrap().is_empty());
    assert!(!before["mutationCandidates"].as_array().unwrap().is_empty());

    // Given a saved graph, replacing the source must remove its old symbols, calls, and candidates.
    fs::write(project.root.join("brew.py"), "def roast():\n    pass\n")?;
    let changed = service.index(&registered.name, &project.root)?;
    assert_eq!(changed.repository_id, registered.id);
    assert_ne!(changed.graph_revision, project.saved.graph_revision);
    assert_eq!((changed.stats.files, changed.stats.callables), (1, 1));
    let mut navigation = navigation_service(Some(&registered.id), None, Some(&project.database))?;
    assert!(navigation.show(project.key("brew")).is_err());
    assert_eq!(navigation.find("roast", 10)?["nodes"].as_array().unwrap().len(), 1);
    assert!(navigation.show(project.key("serve")).is_err());
    drop(navigation);
    let facts = service.get_report_facts(&registered.id)?.unwrap();
    for field in ["edges", "mutationCandidates"] {
        assert_eq!(facts[field], json!([]), "stale {field} after source replacement");
    }

    // When the last source disappears, the stored graph must contain only the root hierarchy.
    fs::remove_file(project.root.join("brew.py"))?;
    let empty = service.index(&registered.name, &project.root)?;
    assert_ne!(empty.graph_revision, changed.graph_revision);
    assert_eq!((empty.stats.files, empty.stats.callables, empty.stats.nodes, empty.stats.edges), (0, 0, 2, 1));
    drop(service);
    let mut reopened = repository_service(Some(&project.database))?;
    let projects = reopened.list()?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, registered.id);
    let facts = reopened.get_report_facts(&registered.id)?.unwrap();
    assert_eq!(facts["graphRevision"], empty.graph_revision);
    for field in ["edges", "imports", "diagnostics", "mutationCandidates"] {
        assert_eq!(facts[field], json!([]), "stale {field}");
    }
    Ok(())
}
