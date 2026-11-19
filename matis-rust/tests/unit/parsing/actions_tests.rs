use super::*;
use crate::domain::models::SourceFile;
use std::path::PathBuf;

fn assert_java_qualification(source: &str, expected: &[(usize, &str, Option<&str>)]) {
    let source = SourceFile {
        path: "Example.java".into(),
        absolute_path: PathBuf::from("/repo/Example.java"),
        language: "java".into(),
        source: source.as_bytes().to_vec(),
    };
    let tree = super::super::syntax::parse("java", &source.source).unwrap();
    assert!(!tree.root_node().has_error());
    let mut actions = Vec::new();
    visit(&ParsedSyntax { source: &source, root: tree.root_node() }, |action| actions.push(action));
    let actual: Vec<_> =
        actions.iter().map(|action| (action.line, action.name.as_str(), action.qualified.as_deref())).collect();
    assert_eq!(actual, expected);
}

#[test]
fn java_receiver_shadowing_positive_pattern_stays_in_if_body() {
    assert_java_qualification(
        concat!(
            "import sample.Cup;\n",
            "class Example {\n",
            "  void run(Object value) {\n",
            "    if (value instanceof Taster Cup) {\n",
            "      Cup.inspect();\n",
            "      new Cup();\n",
            "    } else {\n",
            "      Cup.inspect();\n",
            "    }\n",
            "    Cup.inspect();\n",
            "  }\n",
            "}\n",
        ),
        &[
            (5, "Cup.inspect", None),
            (6, "Cup", Some("sample.Cup")),
            (8, "Cup.inspect", Some("sample.Cup.inspect")),
            (10, "Cup.inspect", Some("sample.Cup.inspect")),
        ],
    );
}

#[test]
fn java_receiver_shadowing_enum_constant_stays_in_enum_body() {
    assert_java_qualification(
        concat!(
            "import sample.Cup;\n",
            "enum Example {\n",
            "  Cup;\n",
            "  void inspect() {}\n",
            "  void run() { Cup.inspect(); new Cup(); }\n",
            "}\n",
            "class Peer { void run() { Cup.inspect(); } }\n",
        ),
        &[(5, "Cup.inspect", None), (5, "Cup", Some("sample.Cup")), (7, "Cup.inspect", Some("sample.Cup.inspect"))],
    );
}
