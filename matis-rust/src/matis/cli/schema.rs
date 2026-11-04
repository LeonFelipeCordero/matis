//! Clap owns command routing, option syntax, positional arguments, and help.
use clap::{Arg, ArgAction, Command};

fn command(name: &'static str, about: &'static str) -> Command {
    Command::new(name).about(about).args_override_self(true)
}
fn value(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name).long(name).help(help).allow_hyphen_values(true)
}
fn flag(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name).long(name).help(help).action(ArgAction::SetTrue)
}
fn folder() -> Arg {
    value("folder", "Folder to analyze").short('f').visible_alias("f").default_value(".").value_name("PATH")
}
fn key(name: &'static str) -> Arg {
    Arg::new(name).required(true)
}
fn format(choices: &'static [&'static str], default: &'static str) -> Arg {
    value("format", "Output format")
        .value_parser(clap::builder::PossibleValuesParser::new(choices))
        .default_value(default)
}
fn traversal(name: &'static str, depth: &'static str, default: &'static str) -> Command {
    command(name, "Read bounded graph traversal")
        .arg(key(if name == "path" { "FROM" } else { "KEY" }))
        .arg(value(depth, "Maximum traversal depth (0..=20)").default_value(default))
        .arg(value("direction", "Traversal direction").value_parser(["out", "in", "both"]).default_value("out"))
        .arg(
            value("relations", "Comma-separated CONTAINS, CALLS, REFERENCES, IMPORTS, INHERITS")
                .default_value("CALLS,REFERENCES"),
        )
        .arg(value("max-nodes", "Maximum nodes (1..=300)").default_value("30"))
        .arg(value("max-edges", "Maximum edges (1..=1000)").default_value("100"))
}
/// Build the command schema with routing, defaults, help, and syntax validation.
///
/// This does not parse arguments or execute commands. Numeric ranges and graph
/// relation selections receive additional validation after parsing.
pub fn cli() -> Command {
    command("matis", "Analyze repositories and store their graphs in SQLite")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(command("index", "Analyze a folder and save its graph in SQLite").arg(folder()))
        .subcommand(command("projects", "List registered projects as JSON"))
        .subcommands(["report", "check"].map(|name| {
            command(name, if name == "check" { "Check sources against matis.json without saving by default" } else { "Show quantitative metrics without saving by default" })
                .arg(folder())
                .arg(flag("save", "Save the analyzed graph in SQLite"))
                .arg(format(&["markdown", "json"], "markdown"))
        }))
        .subcommand(command("mutations", "Derive JSON byte-edit manifests without changing sources or saving results")
            .long_about("Derive independent JSON byte-edit manifests, never change source or save results. Opt in via matis.json: mutations.rules is a list of rule IDs. Default/empty lists generate nothing. --rules lists exact language coverage. Conditions, unary operators and unlisted aliases are excluded. Syntactic support is not type validity.")
            .arg(folder())
            .arg(flag("saved", "Read persisted candidates; reject stale sources"))
            .arg(flag("rules", "Show the versioned rule catalog and limitations"))
            .subcommand(command("run", "Apply independent mutations and run trusted configured test commands")
                .long_about("Apply independent mutations and run trusted configured test commands in this workspace. No baseline or isolation is provided. Nonzero command exits count as killed. Results are incremental JSONL with separate command logs. Each run overwrites previous results. Progress is printed to stderr; stdout remains JSON.")
                .arg(value("folder", "Override the parent project folder").short('f').value_name("PATH"))
                .arg(value("output", "Results file relative to the project root; overwritten on each run").default_value("mutation-results.jsonl").value_name("PATH"))
                .arg(value("target", "Comma-separated target names, or 'all'; prompt when omitted on a terminal"))))
        .subcommand(graph())
}
fn graph() -> Command {
    command("graph", "Read an indexed graph as JSON")
        .long_about("Read an indexed graph as JSON; unresolved targets are not classified as external. Stable keys retain their indexed spelling.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(value(
            "project",
            "Registered project name or ID; defaults to the indexed folder containing the current directory",
        ))
        .arg(value("revision", "Expected graphRevision; reject a stale graph"))
        .subcommand(command("overview", "Print graph counts and bounded root nodes"))
        .subcommand(
            command("find", "Find nodes by name, path, or stable key")
                .arg(key("QUERY"))
                .arg(value("limit", "Maximum matches (1..=100)").default_value("10")),
        )
        .subcommand(command("show", "Print node details and bounded ancestors").arg(key("KEY")))
        .subcommand(
            command("children", "Print a page of direct children")
                .arg(key("KEY"))
                .arg(value("limit", "Maximum children (1..=100)").default_value("30"))
                .arg(value("offset", "Non-negative page offset").default_value("0")),
        )
        .subcommand(traversal("context", "depth", "2"))
        .subcommand(traversal("path", "max-depth", "6").arg(key("TO")))
        .subcommand(
            command("source", "Fetch verified excerpts for one or more keys")
                .arg(key("KEYS").num_args(1..))
                .arg(value("max-lines", "Maximum lines per excerpt (1..=500)").default_value("80"))
                .arg(value("start-line", "Absolute 1-based line; clamped to each symbol start"))
                .arg(format(&["json", "text"], "json")),
        )
}
