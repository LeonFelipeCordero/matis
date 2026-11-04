use super::options::Options;
use crate::{
    domain::navigation::{PathRequest, SourceOptions, TraversalRequest},
    services::navigation::NavigationService,
};
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;

const RELATIONS: &[&str] = &["CONTAINS", "CALLS", "REFERENCES", "IMPORTS", "INHERITS"];
pub struct GraphOptions {
    pub operation: Operation,
    pub text: bool,
}
pub enum Operation {
    Overview,
    Find(String, usize),
    Show(String),
    Children(String, usize, serde_json::Number),
    Context(TraversalRequest),
    Path(PathRequest),
    Source(Vec<String>, SourceOptions),
}
impl GraphOptions {
    /// Convert matches for a known graph subcommand into an operation and format.
    ///
    /// Reject invalid numeric ranges and empty or unknown relation selections.
    /// `matches` must come from the schema for `command`.
    pub fn from_matches(command: &str, matches: &clap::ArgMatches) -> Result<Self> {
        let options = Options::from_matches(&format!("graph {command}"), matches);
        let text = options.get("--format", "json") == "text";
        Ok(Self { operation: Operation::parse(command, options)?, text })
    }
}
impl Operation {
    fn parse(command: &str, options: Options) -> Result<Self> {
        let key = options.positional.first().cloned().unwrap_or_default();
        Ok(match command {
            "overview" => Self::Overview,
            "find" => Self::Find(key, options.number("--limit", 10, (1, 100))?),
            "show" => Self::Show(key),
            "children" => {
                Self::Children(key, options.number("--limit", 30, (1, 100))?, options.integer("--offset", 0, 0)?)
            }
            "context" => Self::Context(traversal_request(command, &options, &key)?),
            "path" => Self::Path(PathRequest {
                target: options.positional[1].clone(),
                traversal: traversal_request(command, &options, &key)?,
            }),
            "source" => Self::Source(options.positional.clone(), source_options(&options)?),
            _ => unreachable!(),
        })
    }
    /// Execute this read operation and return the service's JSON payload.
    ///
    /// Use the service's graph snapshot; source operations also read local files.
    /// Propagate lookup, traversal, database, and source-verification errors.
    pub fn execute(self, service: &mut NavigationService) -> Result<Value> {
        match self {
            Self::Overview => service.overview(),
            Self::Find(query, limit) => service.find(&query, limit),
            Self::Show(key) => service.show(&key),
            Self::Children(key, limit, offset) => service.children(&key, limit, offset),
            Self::Context(request) => service.context(&request),
            Self::Path(request) => service.path(&request),
            Self::Source(keys, options) => source(service, &keys, options),
        }
    }
}
fn source(service: &mut NavigationService, keys: &[String], options: SourceOptions) -> Result<Value> {
    if keys.len() == 1 {
        service.source(&keys[0], options.max_lines, options.start_line)
    } else {
        service.sources(keys, options.max_lines, options.start_line)
    }
}
fn source_options(options: &Options) -> Result<SourceOptions> {
    Ok(SourceOptions {
        max_lines: options.number("--max-lines", 80, (1, 500))?,
        start_line: if options.has("--start-line") { Some(options.integer("--start-line", 1, 1)?) } else { None },
    })
}
fn traversal_request(command: &str, options: &Options, key: &str) -> Result<TraversalRequest> {
    let direction = options.get("--direction", "out");
    let relations: BTreeSet<_> = options
        .get("--relations", "CALLS,REFERENCES")
        .split(',')
        .map(|p| p.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)).to_uppercase())
        .filter(|p| !p.is_empty())
        .collect();
    if relations.is_empty() || relations.iter().any(|r| !RELATIONS.contains(&r.as_str())) {
        return Err(
            options.invalid("--relations", format!("Choose comma-separated relations from {}", RELATIONS.join(", ")))
        );
    }
    let (depth_flag, default_depth) = depth_option(command);
    Ok(TraversalRequest {
        key: key.into(),
        direction: direction.into(),
        relations: relations.into_iter().collect(),
        depth: options.number(depth_flag, default_depth, (0, 20))?,
        max_nodes: options.number("--max-nodes", 30, (1, 300))?,
        max_edges: options.number("--max-edges", 100, (1, 1000))?,
    })
}
/// Return the depth flag and default: `--max-depth`/6 for `path`, otherwise
/// `--depth`/2. This does not validate the command name or a depth value.
pub fn depth_option(command: &str) -> (&'static str, usize) {
    if command == "path" { ("--max-depth", 6) } else { ("--depth", 2) }
}
