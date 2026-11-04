use anyhow::Result;
use std::{collections::BTreeMap, fmt};

#[derive(Debug)]
pub struct UsageError {
    pub command: String,
    pub message: String,
}
impl fmt::Display for UsageError {
    /// Write the usage message without its command path; propagate formatter errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for UsageError {}
/// Create a CLI usage error with the command path used to select its diagnostic.
///
/// `command` names schema subcommands separated by spaces; `message` is shown
/// by the CLI boundary, which normally returns exit status 2.
pub fn usage(command: &str, message: impl Into<String>) -> anyhow::Error {
    UsageError { command: command.into(), message: message.into() }.into()
}
pub struct Options {
    pub command: String,
    pub values: BTreeMap<String, String>,
    pub positional: Vec<String>,
}
impl Options {
    /// Collect parsed values for a schema command path such as `mutations run`.
    ///
    /// Keep positional values in order and option names with `--` prefixes.
    /// Store enabled flags as `true` and the last value of repeated options.
    ///
    /// # Panics
    /// Panics if `command` is unknown or `matches` does not fit its schema.
    pub fn from_matches(command: &str, matches: &clap::ArgMatches) -> Self {
        let mut result = Self { command: command.into(), values: BTreeMap::new(), positional: vec![] };
        let cli = super::schema::cli();
        let mut schema = &cli;
        for name in command.split_whitespace() {
            schema = schema.find_subcommand(name).expect("known CLI command");
        }
        for argument in schema.get_arguments().filter(|arg| arg.get_id() != "help") {
            result.insert_match(argument, matches);
        }
        result
    }
    fn insert_match(&mut self, argument: &clap::Arg, matches: &clap::ArgMatches) {
        let id = argument.get_id().as_str();
        if matches!(argument.get_action(), clap::ArgAction::SetTrue) {
            if matches.get_flag(id) {
                self.values.insert(format!("--{id}"), "true".into());
            }
        } else if let Some(mut values) = matches.get_many::<String>(id) {
            if argument.is_positional() {
                self.positional.extend(values.cloned());
            } else {
                self.values.insert(format!("--{id}"), values.next_back().unwrap().clone());
            }
        }
    }
    /// Borrow the stored option value, or `default` if the name is absent.
    pub fn get<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.values.get(name).map(String::as_str).unwrap_or(default)
    }
    /// Return whether the option name is stored, including schema defaults.
    pub fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
    /// Parse a Python-style decimal option as `usize` within inclusive bounds.
    ///
    /// Use and range-check `default` when absent. Return a usage error for invalid
    /// syntax or range, and propagate numeric conversion errors.
    pub fn number(&self, name: &str, default: usize, range: (usize, usize)) -> Result<usize> {
        Ok(self.checked_integer(name, default, (range.0, Some(range.1)))?.to_string().parse()?)
    }
    /// Parse a decimal option as a JSON integer with no upper bound.
    ///
    /// Use `default` when absent; reject invalid Python-style decimal syntax or
    /// values below `minimum` with a usage error. Propagate JSON number errors.
    pub fn integer(&self, name: &str, default: usize, minimum: usize) -> Result<serde_json::Number> {
        self.checked_integer(name, default, (minimum, None))
    }
    fn checked_integer(&self, name: &str, default: usize, range: (usize, Option<usize>)) -> Result<serde_json::Number> {
        let number = match self.values.get(name) {
            None => default.to_string(),
            Some(value) => super::integers::normalize(value).ok_or_else(|| {
                self.invalid(name, format!("{} is not a valid integer range.", super::integers::repr(value)))
            })?,
        };
        if !super::integers::in_range(&number, range) {
            return Err(self.invalid(
                name,
                format!("{number} is not in the range {}.", super::integers::range_description(range)),
            ));
        }
        Ok(number.parse()?)
    }
    /// Create a usage error that identifies this command and the invalid option.
    pub fn invalid(&self, name: &str, message: impl fmt::Display) -> anyhow::Error {
        usage(&self.command, format!("Invalid value for '{name}': {message}"))
    }
}
/// Serialize JSON on one line with spaces after structural commas and colons.
///
/// Preserve Unicode and string contents; do not insert spacing inside strings.
pub fn json_dump(value: &serde_json::Value) -> String {
    let compact = serde_json::to_string(value).expect("JSON value serialization");
    let mut result = String::new();
    let (mut quoted, mut escaped) = (false, false);
    for character in compact.chars() {
        result.push(character);
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            quoted = !quoted;
        }
        if !quoted && matches!(character, ',' | ':') {
            result.push(' ');
        }
    }
    result
}
