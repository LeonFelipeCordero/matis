//! Explicit target selection and terminal-only prompting, matching the Python CLI.
use super::options::{Options, usage};
use anyhow::Result;
use serde_json::{Map, Value};
use std::io::{self, IsTerminal, Write};

fn parse(value: &str, targets: &Map<String, Value>) -> Result<Map<String, Value>> {
    let names: Vec<_> = value.split(',').map(str::trim).collect();
    let error = if names.iter().any(|name| name.is_empty()) {
        Some("Provide comma-separated target names or 'all'; empty names are not allowed".to_owned())
    } else if names == ["all"] {
        return Ok(targets.clone());
    } else if names.contains(&"all") {
        Some("Use 'all' alone, not with other target names".to_owned())
    } else {
        let unknown: std::collections::BTreeSet<_> =
            names.iter().copied().filter(|name| !targets.contains_key(*name)).collect();
        (!unknown.is_empty()).then(|| {
            format!(
                "Unknown mutation targets: {}. Available targets: {}, all",
                unknown.into_iter().collect::<Vec<_>>().join(", "),
                targets.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        })
    };
    if let Some(message) = error {
        return Err(usage("mutations run", format!("Invalid value for --target: {message}")));
    }
    Ok(names.into_iter().map(|name| (name.to_owned(), targets[name].clone())).collect())
}

/// Replace the in-memory target map with the explicit or interactive selection.
///
/// `project` must already contain a validated `mutations.targets` object. Keep
/// requested order and remove duplicate names. Prompt on stderr only when stdin
/// is a terminal. Return `false` on prompt EOF without changing the project.
/// Reject empty or unknown names, mixed `all` selections, and missing nonterminal
/// selections; propagate terminal I/O errors. Do not write configuration to disk.
pub fn select(project: &mut Value, options: &Options) -> Result<bool> {
    let targets = project["mutations"]["targets"].as_object().expect("validated targets");
    let selected = if let Some(value) = options.values.get("--target") {
        parse(value, targets)?
    } else {
        let Some(selected) = prompt(targets)? else { return Ok(false) };
        selected
    };
    project["mutations"]["targets"] = Value::Object(selected);
    Ok(true)
}

fn prompt(targets: &Map<String, Value>) -> Result<Option<Map<String, Value>>> {
    if !io::stdin().is_terminal() {
        return Err(usage(
            "mutations run",
            "Pass --target NAME[,NAME...] or --target all when stdin is not a terminal",
        ));
    }
    eprintln!("Available mutation targets:\n  all (all configured targets)");
    for name in targets.keys() {
        eprintln!("  {name}");
    }
    loop {
        eprint!("Targets (comma-separated names or all): ");
        io::stderr().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            eprintln!("Aborted!");
            return Ok(None);
        }
        let value = line.trim_end_matches(['\r', '\n']);
        if value.is_empty() {
            continue;
        }
        match parse(value, targets) {
            Ok(selected) => return Ok(Some(selected)),
            Err(error) => eprintln!("Error: {}", error.to_string().trim_start_matches("Invalid value for --target: ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn selection_preserves_request_order_and_rejects_empty_or_unknown_names() {
        let targets = json!({"z": {}, "a": {}}).as_object().unwrap().clone();
        assert_eq!(parse("a,z,a", &targets).unwrap().keys().collect::<Vec<_>>(), ["a", "z"]);
        assert_eq!(parse("all", &targets).unwrap(), targets);
        for value in ["", "a,", "all,a", "missing"] {
            assert!(parse(value, &targets).is_err(), "{value}");
        }
    }
}
