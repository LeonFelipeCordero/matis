use super::options::{Options, UsageError, json_dump};
use crate::{paths::absolute_path, services::repositories::analyze_folder};
use anyhow::Result;
use serde_json::json;
use std::path::Path;

/// Execute the CLI with process arguments, excluding the executable name.
///
/// Return the exit status from [`run_args`] without terminating the process.
/// Commands can print output, read sources, update SQLite, or run mutations.
/// Diagnostic output failures propagate as errors.
pub fn run() -> Result<i32> {
    run_args(&std::env::args().skip(1).collect::<Vec<_>>())
}
/// Parse and execute arguments that exclude the executable name.
///
/// Commands print results and diagnostics. Indexing and `--save` persist graphs
/// in SQLite; mutation runs edit and restore sources and execute trusted commands.
/// Return 0 on success/help, 2 for usage errors, or 1 for command failures or
/// check findings. Diagnostic output failures propagate instead of an exit status.
pub fn run_args(args: &[String]) -> Result<i32> {
    match dispatch(args) {
        Ok(code) => Ok(code),
        Err(error) => {
            if let Some(error) = error.downcast_ref::<UsageError>() {
                let mut schema = super::schema::cli();
                let mut command = &mut schema;
                for name in error.command.split_whitespace() {
                    command = command.find_subcommand_mut(name).expect("known command");
                }
                command
                    .clone()
                    .bin_name(format!("matis {}", error.command))
                    .error(clap::error::ErrorKind::ValueValidation, &error.message)
                    .print()?;
                return Ok(2);
            }
            eprintln!("Error: {error}");
            Ok(1)
        }
    }
}
fn dispatch(args: &[String]) -> Result<i32> {
    let matches = match super::schema::cli()
        .try_get_matches_from(std::iter::once("matis").chain(args.iter().map(String::as_str)))
    {
        Ok(matches) => matches,
        Err(error) => {
            error.print()?;
            return Ok(error.exit_code());
        }
    };
    let (command, matches) = matches.subcommand().expect("clap requires a command");
    match command {
        "graph" => super::graph::run(matches),
        "mutations" => super::mutations::run(matches),
        _ => standard_command(command, matches),
    }
}
fn standard_command(command: &str, matches: &clap::ArgMatches) -> Result<i32> {
    let options = Options::from_matches(command, matches);
    let format = options.get("--format", "markdown");
    if command == "projects" {
        println!("{}", serde_json::to_string_pretty(&crate::bootstrap::repository_service(None)?.list()?)?);
        return Ok(0);
    }
    let root = absolute_path(Path::new(options.get("--folder", ".")))?;
    let config = if command == "check" { Some(crate::metrics::config::load_config(&root)?) } else { None };
    let artifact = analyze_folder(&root)?;
    if command == "index" {
        let graph = crate::bootstrap::repository_service(None)?.save_folder(&root, &artifact)?;
        println!(
            "{}",
            json_dump(
                &json!({"repositoryId": graph.repository_id, "schemaVersion": graph.schema_version, "stats": graph.stats})
            )
        );
        return Ok(0);
    }
    let report = crate::metrics::report::build_artifact_report(&artifact)?;
    if options.has("--save") {
        crate::bootstrap::repository_service(None)?.save_folder(&root, &artifact)?;
    }
    if command == "report" {
        return print_report(&report, format);
    }
    print_check(&report, config.as_ref().unwrap(), format)
}
fn print_report(report: &serde_json::Value, format: &str) -> Result<i32> {
    println!(
        "{}",
        if format == "json" {
            serde_json::to_string_pretty(report)?
        } else {
            crate::metrics::report::render_markdown(report)
        }
    );
    Ok(0)
}
fn print_check(report: &serde_json::Value, config: &serde_json::Value, format: &str) -> Result<i32> {
    let check = crate::metrics::check::build_check(report, config)?;
    println!(
        "{}",
        if format == "json" {
            serde_json::to_string_pretty(&check)?
        } else {
            crate::metrics::check::render_check(&check)
        }
    );
    Ok(if check["findings"].as_array().is_some_and(|f| !f.is_empty()) { 1 } else { 0 })
}
