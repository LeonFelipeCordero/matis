//! Mutation manifests are derived only. Never save a graph or change source.
use super::options::Options;
use crate::mutations::{candidates::collect_candidates, config, report::build_candidate_report, rules::REGISTRY};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

fn output(options: &Options) -> Result<Value> {
    if options.has("--rules") {
        return Ok(json!({"rules": &*REGISTRY}));
    }
    let root = crate::paths::absolute_path(Path::new(options.get("--folder", ".")))?;
    let project = crate::metrics::config::load_project_config(&root)?;
    let config = config::from_project(&project)?;
    let candidates = if options.has("--saved") {
        crate::bootstrap::repository_service(None)?.saved_folder_candidates(&root)?
    } else {
        collect_candidates(&root)?
    };
    build_candidate_report(&root, &candidates, &config)
}

/// Print the rule catalog or verified manifests, or dispatch a mutation run.
///
/// Manifest generation reads fresh or saved candidates without saving a graph
/// or editing sources. The `run` subcommand executes trusted commands and writes
/// results while temporarily editing sources. Return its status, or 0 after
/// printing JSON. Reject `--rules` with a subcommand; propagate configuration,
/// database, filesystem, and candidate-validation errors to the CLI boundary.
pub fn run(matches: &clap::ArgMatches) -> Result<i32> {
    let options = Options::from_matches("mutations", matches);
    if let Some(("run", child)) = matches.subcommand() {
        if options.has("--rules") {
            return Err(super::options::usage("mutations", "--rules cannot be combined with a subcommand"));
        }
        return super::mutation_run::run(&options, child);
    }
    println!("{}", serde_json::to_string_pretty(&output(&options)?)?);
    Ok(0)
}

#[cfg(test)]
#[path = "../../../tests/integration/cli/mutations.rs"]
mod integration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn options(args: &[String]) -> Result<Options> {
        let matches = super::super::schema::cli()
            .try_get_matches_from(["matis", "mutations"].into_iter().chain(args.iter().map(String::as_str)))?;
        Ok(Options::from_matches("mutations", matches.subcommand_matches("mutations").unwrap()))
    }
    #[test]
    fn help_tokens_are_values_only_when_consumed_by_options() {
        let value = options(&["--folder".into(), "--help".into()]).unwrap();
        assert_eq!(value.get("--folder", "."), "--help");
        assert!(options(&["--".into(), "--help".into()]).is_err());
        assert!(options(&["--help".into()]).is_err());
    }
    #[test]
    fn rules_bypass_every_boundary() -> Result<()> {
        let args = ["--folder", "/missing-folder", "--rules", "--saved"].map(String::from);
        assert_eq!(output(&options(&args)?)?, json!({"rules": &*REGISTRY}));
        Ok(())
    }
    #[test]
    fn folder_aliases_and_help() -> Result<()> {
        for flag in ["-f", "--f", "--folder"] {
            let args = [flag, "~/sub/..", "--saved"].map(String::from);
            let options = options(&args)?;
            assert_eq!(options.get("--folder", "."), "~/sub/..");
            assert!(options.has("--saved"));
        }
        let mut cli = super::super::schema::cli();
        let help = cli.find_subcommand_mut("mutations").unwrap().render_long_help().to_string();
        assert!(help.contains("mutations.rules is a list of rule IDs"));
        Ok(())
    }
}
