//! Run trusted mutation targets in the current workspace; stdout is JSON only.
use super::options::Options;
use crate::mutations::{config, report::build_mutation_report, runner};
use anyhow::{Result, bail};
use serde_json::json;
use std::path::Path;

fn progress(message: &str) {
    eprintln!("[mutations] {message}");
}

fn ascii_json(value: &serde_json::Value) -> Result<String> {
    let text = serde_json::to_string_pretty(value)?;
    Ok(text
        .chars()
        .map(|character| {
            if character.is_ascii() {
                return character.to_string();
            }
            character.encode_utf16(&mut [0; 2]).iter().map(|unit| format!("\\u{unit:04x}")).collect()
        })
        .collect())
}

/// Select targets, generate verified manifests, and execute a mutation campaign.
///
/// Inherit the parent's folder and saved-candidate mode unless the child overrides
/// the folder. Validate all configuration before target selection and lock the
/// project before analysis. Temporarily edit sources, execute trusted commands,
/// and replace JSONL results and reused logs. Print progress to stderr and a JSON
/// summary to stdout. No baseline or isolated workspace is created.
///
/// Return 0 on completion, even for surviving mutants or recorded target errors;
/// return 1 on selection cancellation or interruption. Propagate configuration,
/// source-validation, locking, output, and restoration failures.
pub fn run(parent: &Options, matches: &clap::ArgMatches) -> Result<i32> {
    let options = Options::from_matches("mutations run", matches);
    let root = crate::paths::absolute_path(Path::new(options.get("--folder", parent.get("--folder", "."))))?;
    progress(&format!("Loading configuration: {}", root.join("matis.json").display()));
    let mut project = crate::metrics::config::load_project_config(&root)?;
    // Validate the full configuration before selecting a subset of targets.
    let _ = config::from_project(&project)?;
    if project["mutations"]["targets"].as_object().is_none_or(|targets| targets.is_empty()) {
        bail!("Configure mutations.targets in matis.json before running mutations");
    }
    if !super::mutation_targets::select(&mut project, &options)? {
        return Ok(1);
    }
    let config = config::from_project(&project)?;
    let names = project["mutations"]["targets"].as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    progress(&format!("Selected targets: {}", names.join(", ")));
    let runner = runner::MutationRunner::new(&root, &config)?;
    let saved = parent.has("--saved");
    progress(&format!(
        "{} {}",
        if saved { "Reading saved candidates for" } else { "Analyzing source files in" },
        root.display()
    ));
    let mut report = if saved {
        crate::services::repositories::saved_folder_report(&root)?
    } else {
        crate::metrics::report::build_artifact_report(&crate::indexer::analyze_repository(&root)?)?
    };
    let candidates = report["mutationCandidates"].as_array_mut().unwrap();
    candidates.retain(|item| !runner::matching_targets(item["path"].as_str().unwrap_or(""), &config).is_empty());
    progress(&format!(
        "Found {} candidate sites in selected targets; generating configured replacements",
        candidates.len()
    ));
    let planned = build_mutation_report(&root, &report, &config)?;
    let destination = root.join(options.get("--output", "mutation-results.jsonl"));
    let execution = runner.run_with_progress(planned["mutations"].as_array().unwrap(), &destination, &mut progress);
    let counts = match execution {
        Err(error) if error.to_string() == "Mutation run interrupted" => {
            eprintln!("\nAborted!");
            return Ok(1);
        }
        result => result?,
    };
    println!("{}", ascii_json(&json!({"output": crate::paths::absolute_path(&destination)?, "counts": counts}))?);
    Ok(0)
}
