use super::parser::{parse_file, parse_files};
use crate::domain::models::SourceFile;
use anyhow::Result;
use std::path::PathBuf;

fn source(language: &str, text: &str) -> SourceFile {
    SourceFile {
        path: format!("test.{language}"),
        absolute_path: PathBuf::from(format!("test.{language}")),
        language: language.to_owned(),
        source: text.as_bytes().to_vec(),
    }
}

#[test]
fn python_structure_calls_and_inheritance() -> Result<()> {
    let file = parse_file(source(
        "python",
        "@decorate\nclass Child(Base[T]):\n    def run(self):\n        helper()\n        self.work()\n",
    ))?;
    assert_eq!(file.symbols.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Child", "run"]);
    assert_eq!(file.symbols[0].decorators, ["@decorate"]);
    assert_eq!(file.inheritances[0].target_name, "Base");
    assert_eq!(file.calls.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["helper", "work"]);
    assert!(file.calls[0].bare_target);
    assert_eq!(file.calls[1].qualifier.as_deref(), Some("self"));
    Ok(())
}

#[test]
fn parse_files_preserves_input_order_duplicates_and_errors() -> Result<()> {
    let mut first = source("python", "");
    first.path = "z.py".into();
    let mut second = source("python", "");
    second.path = "a.py".into();
    let repository = parse_files(vec![first.clone(), second, first.clone()])?;
    assert_eq!(
        repository.files.iter().map(|file| file.source.path.as_str()).collect::<Vec<_>>(),
        ["z.py", "a.py", "z.py"]
    );
    assert!(parse_files(vec![first, source("not-a-grammar", "")]).is_err());
    Ok(())
}

#[test]
fn script_callbacks_and_jsx_references() -> Result<()> {
    let file = parse_file(source("tsx", "const run = ((x) => helper(x));\nconst view = <cards.CoffeeCard />;\n"))?;
    assert!(file.symbols.iter().any(|s| s.name == "run" && s.kind == "function"));
    let component = file.calls.iter().find(|call| call.name == "CoffeeCard").unwrap();
    assert_eq!(component.relationship, "REFERENCES");
    assert_eq!(component.qualifier.as_deref(), Some("cards"));
    Ok(())
}

#[test]
fn rust_macro_payload_offsets_and_shadowing() -> Result<()> {
    let text = "fn main() { println!(\"{}\", helper()); custom!(hidden()); }";
    let file = parse_file(source("rust", text))?;
    assert_eq!(file.calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["println!", "helper", "custom!"]);
    let helper = &file.calls[1];
    assert_eq!(&text[helper.start_byte..helper.end_byte], "helper()");
    assert!(helper.bare_target);
    let shadowed =
        parse_file(source("rust", "macro_rules! println { ($x:expr) => {} } fn main(){ println!(hidden()); }"))?;
    assert!(!shadowed.calls.iter().any(|call| call.name == "hidden"));
    Ok(())
}

#[test]
fn indexed_call_lines_cover_blank_lines_nested_calls_and_macro_offsets() -> Result<()> {
    let file = parse_file(source("python", "\nfirst()\n\nsecond(\n third()\n)\n"))?;
    let lines: Vec<_> = file.calls.iter().map(|call| (call.name.as_str(), call.line)).collect();
    assert_eq!(lines, [("first", 2), ("second", 4), ("third", 5)]);
    let file = parse_file(source("rust", "fn main() {\n println!(\n \"{} {}\",\n helper(),\n nested!());\n}\n"))?;
    let lines: Vec<_> = file.calls.iter().map(|call| (call.name.as_str(), call.line)).collect();
    assert_eq!(lines, [("println!", 2), ("helper", 4), ("nested!", 5)]);
    Ok(())
}

#[test]
fn syntax_errors_and_invalid_utf8_preserve_hash_source() -> Result<()> {
    let file = parse_file(source("python", "def broken(:\n pass"))?;
    assert!(file.parse_error.unwrap().starts_with("Syntax error at line "));
    let mut input = source("python", "run()");
    input.source = b"#\xff\nrun()".to_vec();
    let file = parse_file(input.clone())?;
    assert_eq!(file.source.source, input.source);
    assert_eq!(file.calls[0].start_byte, 5);
    use sha2::{Digest, Sha256};
    assert_eq!(file.content_hash, format!("{:x}", Sha256::digest(&input.source)));
    assert!(parse_files(vec![])?.files.is_empty());
    assert!(parse_file(source("c", "int main() {}")).is_err());
    Ok(())
}
