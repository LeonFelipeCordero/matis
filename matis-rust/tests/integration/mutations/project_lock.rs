use super::*;

#[test]
fn dropping_runner_releases_locks_even_with_inherited_descriptors() -> Result<()> {
    let project = tempfile::tempdir()?;
    let config = MutationConfig::default();
    let runner = MutationRunner::new(project.path(), &config)?;
    // A descriptor clone retains the same lock, as a concurrent fork does until exec.
    let inherited = runner._project_locks.iter().map(|lock| lock.0.try_clone()).collect::<std::io::Result<Vec<_>>>()?;
    assert!(MutationRunner::new(project.path(), &config).is_err());
    drop(runner);
    let replacement = MutationRunner::new(project.path(), &config)?;
    drop(inherited);
    assert!(MutationRunner::new(project.path(), &config).is_err());
    drop(replacement);
    assert!(MutationRunner::new(project.path(), &config).is_ok());
    Ok(())
}
