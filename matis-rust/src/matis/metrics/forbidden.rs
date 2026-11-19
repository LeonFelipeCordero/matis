//! Evaluate user rules for source-level calls and explicit constructors.
use crate::{
    parsing::{
        actions::{self, ActionKind},
        syntax::ParsedSyntax,
    },
    paths::globs,
};
use serde_json::{Value, json};

const CALLS: (&str, &str) = ("forbiddenCalls", "calls");
const CONSTRUCTORS: (&str, &str) = ("forbiddenConstructors", "types");
pub(super) const RULES: [(&str, &str); 2] = [CALLS, CONSTRUCTORS];

pub(crate) fn enabled(config: &Value) -> bool {
    RULES.iter().any(|(rule, _)| !config[*rule].as_array().unwrap().is_empty())
}

pub(crate) fn findings(config: &Value, syntax: ParsedSyntax<'_>, findings: &mut Vec<Value>) {
    let path = &syntax.source.path;
    let select = |(rule, _): (&str, &str)| {
        config[rule].as_array().unwrap().iter().filter(|entry| in_scope(entry, path)).collect::<Vec<_>>()
    };
    let calls = select(CALLS);
    let constructors = select(CONSTRUCTORS);
    if calls.is_empty() && constructors.is_empty() {
        return;
    }
    actions::visit(&syntax, |action| {
        let ((rule, target), entries) = match action.kind {
            ActionKind::Call => (CALLS, &calls),
            ActionKind::Constructor => (CONSTRUCTORS, &constructors),
        };
        for entry in entries {
            if let Some(name) = matching_name(entry, target, &action) {
                findings.push(json!({"rule": rule, "metric": target, "subject": name, "actual": 1, "limit": 0,
                    "files": [{"path": path, "line": action.line}],
                    "details": [entry["message"].as_str().unwrap_or("This action is forbidden in this file.")]}));
            }
        }
    });
}

fn in_scope(entry: &Value, path: &str) -> bool {
    entry["files"].as_array().unwrap().iter().any(|pattern| globs::matches(path, pattern.as_str().unwrap()))
}

fn matching_name<'a>(entry: &'a Value, target: &str, action: &actions::Action) -> Option<&'a str> {
    entry[target]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .find(|name| *name == action.name || Some(*name) == action.qualified.as_deref())
}
