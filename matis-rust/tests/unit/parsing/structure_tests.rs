use super::*;
use crate::{parsing::syntax::parse, syntax::named_children};
use tree_sitter_language_pack::{ProcessConfig, process};

#[test]
fn kotlin_queries_keep_declaration_names_kinds_and_callback_carriers() -> Result<()> {
    let source = crate::domain::models::SourceFile {
        path: "sample.kt".into(),
        absolute_path: "sample.kt".into(),
        language: "kotlin".into(),
        source: concat!(
            "package coffee\ninterface Reader {\n fun read(): Int\n}\n",
            "class Child {\n companion object {}\n}\nclass Named {\n companion object Factory {}\n}\n",
            "object Singleton {}\ntypealias Label = String\nfun String.extension(): Int = 1\n",
            "fun <T> generic(value: T): T = value\nval callback = { 1 }\n",
            "val anonymous = fun(): Int { return 1 }\n"
        )
        .as_bytes()
        .to_vec(),
    };
    let tree = parse("kotlin", &source.source)?;
    let syntax = ParsedSyntax { source: &source, root: tree.root_node() };
    let items = extract_structure(&syntax, &[])?;
    let names: Vec<_> = items.iter().map(|item| (item.name.as_deref(), kind_name(&item.kind))).collect();
    assert_eq!(
        names,
        [
            (Some("coffee"), "module".into()),
            (Some("Reader"), "interface".into()),
            (Some("Child"), "class".into()),
            (Some("Named"), "class".into()),
            (Some("Singleton"), "class".into()),
            (Some("Label"), "type".into()),
            (Some("extension"), "function".into()),
            (Some("generic"), "function".into()),
            (None, "function".into()),
            (None, "function".into()),
        ]
    );
    for (index, name, kind) in [
        (1, "read", StructureKind::Function),
        (2, "Companion", StructureKind::Class),
        (3, "Factory", StructureKind::Class),
    ] {
        assert_eq!(items[index].children.len(), 1);
        assert_eq!(items[index].children[0].name.as_deref(), Some(name));
        assert_eq!(items[index].children[0].kind, kind);
    }
    assert_eq!(items[8].signature.as_deref(), Some("{ 1 }"));
    assert_eq!(items[9].signature.as_deref(), Some("fun(): Int { return 1 }"));
    Ok(())
}

#[test]
fn kotlin_query_names_reconcile_native_identity_without_losing_metadata() -> Result<()> {
    let source = crate::domain::models::SourceFile {
        path: "sample.kt".into(),
        absolute_path: "sample.kt".into(),
        language: "kotlin".into(),
        source: b"@Trace internal interface Reader {}\nprivate fun <T> String.brew(value: T): T = value\n".to_vec(),
    };
    let tree = parse("kotlin", &source.source)?;
    let nodes = named_children(tree.root_node());
    let syntax = ParsedSyntax { source: &source, root: tree.root_node() };
    for name in [None, Some(String::new()), Some("legacy".to_owned())] {
        let native = vec![
            StructureItem {
                name: Some("Reader".into()),
                kind: StructureKind::Class,
                span: span(nodes[0]),
                ..Default::default()
            },
            StructureItem {
                name,
                kind: StructureKind::Function,
                span: span(nodes[1]),
                signature: Some("native header".into()),
                visibility: Some("private".into()),
                decorators: vec!["@native".into()],
                doc_comment: Some("native documentation".into()),
                body_span: Some(span(nodes[1])),
                ..Default::default()
            },
        ];
        let original = native.clone();
        let items = extract_structure(&syntax, &native)?;
        let expected = vec![
            StructureItem {
                kind: StructureKind::Interface,
                signature: Some("@Trace internal interface Reader {}".into()),
                visibility: Some("internal".into()),
                decorators: vec!["@Trace".into()],
                ..native[0].clone()
            },
            StructureItem { name: Some("brew".into()), ..native[1].clone() },
        ];
        assert_eq!(items, expected);
        assert_eq!(native, original);
    }
    Ok(())
}

#[test]
fn kotlin_interface_kind_does_not_depend_on_native_name_identity() -> Result<()> {
    for (text, native_name, expected_name) in [
        ("interface <T> {}", Some("legacy"), Some("legacy")),
        ("interface : Base {}", None, None),
        ("interface : Base {}", Some(""), None),
        ("interface : Base {}", Some("legacy"), Some("legacy")),
        ("@Trace internal interface Reader {}", None, None),
        ("@Trace internal interface Reader {}", Some(""), None),
        ("@Trace internal interface Reader {}", Some("legacy"), Some("legacy")),
    ] {
        let source = crate::domain::models::SourceFile {
            path: "sample.kt".into(),
            absolute_path: "sample.kt".into(),
            language: "kotlin".into(),
            source: text.as_bytes().to_vec(),
        };
        let tree = parse("kotlin", &source.source)?;
        let node = crate::syntax::walk(tree.root_node()).find(|node| node.kind() == "class_declaration").unwrap();
        let syntax = ParsedSyntax { source: &source, root: tree.root_node() };
        let native = StructureItem {
            name: native_name.map(str::to_owned),
            kind: StructureKind::Class,
            span: span(node),
            signature: Some("native header".into()),
            visibility: Some("private".into()),
            decorators: vec!["@native".into()],
            doc_comment: Some("native documentation".into()),
            body_span: Some(span(node)),
            ..Default::default()
        };
        let items = extract_structure(&syntax, std::slice::from_ref(&native))?;
        let merged = items.iter().find(|item| item.signature.as_deref() == Some("native header")).unwrap();
        assert_eq!(
            *merged,
            StructureItem { name: expected_name.map(str::to_owned), kind: StructureKind::Interface, ..native }
        );
        assert!(items.iter().all(|item| item.kind == StructureKind::Interface));
    }
    Ok(())
}

fn capture<'a>(node: Node<'a>, name: Option<&str>, kind: &str) -> CapturedDeclaration<'a> {
    CapturedDeclaration { node, name: name.map(str::to_owned), kind: kind.to_owned(), has_name: name.is_some() }
}

#[test]
fn captures_keep_first_position_and_last_named_metadata_without_rebuilding_text() -> Result<()> {
    let source = b"@Trace public class First {} class Second {}";
    let tree = parse("java", source)?;
    let nodes = named_children(tree.root_node());
    let extraction = Extraction { source, adapter: get_adapter("java")? };
    let mut captures = CapturedDeclarations::default();
    extraction.add_capture(&mut captures, capture(nodes[0], None, "class"));
    extraction.add_capture(&mut captures, capture(nodes[1], Some("Second"), "class"));
    let signature_pointer = captures.entries[0].1.signature.as_ref().unwrap().as_ptr();
    extraction.add_capture(&mut captures, capture(nodes[0], Some("Named"), "function"));
    extraction.add_capture(&mut captures, capture(nodes[0], Some("Last"), "interface"));
    extraction.add_capture(&mut captures, capture(nodes[0], None, "class"));
    let first = &captures.entries[0].1;
    assert_eq!(captures.entries.len(), 2);
    assert_eq!(captures.positions[&nodes[0].id()], 0);
    assert_eq!(captures.entries[1].0, nodes[1]);
    assert_eq!(first.name.as_deref(), Some("Last"));
    assert_eq!(first.kind, StructureKind::Interface);
    assert_eq!(first.visibility.as_deref(), Some("public"));
    assert_eq!(first.decorators, ["@Trace"]);
    assert_eq!(first.signature.as_ref().unwrap().as_ptr(), signature_pointer);
    extraction.add_capture(&mut captures, capture(nodes[0], Some(""), "method"));
    assert_eq!(captures.entries[0].1.name.as_deref(), Some(""));
    assert_eq!(captures.entries[0].1.kind, StructureKind::Method);
    Ok(())
}

#[test]
fn many_declarations_keep_source_order_and_identity() -> Result<()> {
    let source: String = (0..5000).map(|index| format!("function f{index}() {{}}\n")).collect();
    let tree = parse("javascript", source.as_bytes())?;
    let extraction = Extraction { source: source.as_bytes(), adapter: get_adapter("javascript")? };
    let query = declaration_query("javascript")?.unwrap();
    let declarations = extraction.captured_declarations(tree.root_node(), &query);
    assert_eq!(declarations.len(), 5000);
    assert_eq!(declarations[0].1.name.as_deref(), Some("f0"));
    assert_eq!(declarations[4999].1.name.as_deref(), Some("f4999"));
    assert!(declarations.windows(2).all(|pair| pair[0].0.start_byte() < pair[1].0.start_byte()));
    Ok(())
}

#[test]
fn native_items_copy_fields_without_copying_descendants() -> Result<()> {
    let source = b"class Outer:\n def method(self):\n  def nested():\n   pass\n";
    let tree = parse("python", source)?;
    let native = process(std::str::from_utf8(source)?, &ProcessConfig::new("python"))?;
    let original = native.structure.clone();
    let mut declarations = vec![];
    native_declarations(tree.root_node(), &native.structure, &mut declarations);
    assert_eq!(declarations.len(), 3);
    assert!(declarations.iter().all(|(_, item)| item.children.is_empty()));
    assert_eq!(native.structure, original);
    let first = &native.structure[0];
    let shallow = without_children(first);
    assert_eq!(shallow.name, first.name);
    assert_eq!(shallow.kind, first.kind);
    assert_eq!(shallow.span, first.span);
    assert_eq!(shallow.signature, first.signature);
    assert_eq!(shallow.body_span, first.body_span);
    assert_eq!(shallow.doc_comment, first.doc_comment);
    assert_eq!(shallow.decorators, first.decorators);
    assert_eq!(shallow.visibility, first.visibility);
    Ok(())
}

#[test]
fn unchanged_carriers_preserve_native_fields_without_rebuilding_signatures() -> Result<()> {
    let source = b"@Trace public class Declared {}";
    let tree = parse("java", source)?;
    let node = tree.root_node().named_child(0).unwrap();
    let extraction = Extraction { source, adapter: get_adapter("java")? };
    let native = StructureItem {
        name: Some("old".to_owned()),
        kind: StructureKind::Class,
        span: span(node),
        signature: Some("native header".to_owned()),
        visibility: Some("private".to_owned()),
        decorators: vec!["@native".to_owned()],
        doc_comment: Some("native documentation".to_owned()),
        body_span: Some(span(node)),
        ..Default::default()
    };
    let pointer = native.signature.as_ref().unwrap().as_ptr();
    let (_, result) = extraction.canonical_declaration(node, native).unwrap();
    assert_eq!(result.name.as_deref(), Some("Declared"));
    assert_eq!(result.signature.as_deref(), Some("native header"));
    assert_eq!(result.signature.as_ref().unwrap().as_ptr(), pointer);
    assert_eq!(result.visibility.as_deref(), Some("private"));
    assert_eq!(result.decorators, ["@native"]);
    assert_eq!(result.doc_comment.as_deref(), Some("native documentation"));
    assert_eq!(result.body_span, Some(span(node)));
    Ok(())
}

#[test]
fn changed_carriers_rebuild_signature_and_keep_decorators() -> Result<()> {
    let source = b"@decorate\ndef run():\n pass\n";
    let tree = parse("python", source)?;
    let wrapper = tree.root_node().named_child(0).unwrap();
    let definition = wrapper.child_by_field_name("definition").unwrap();
    let native = StructureItem { span: span(wrapper), signature: Some("old".to_owned()), ..Default::default() };
    let extraction = Extraction { source, adapter: get_adapter("python")? };
    let (node, item) = extraction.canonical_declaration(wrapper, native).unwrap();
    assert_eq!(node, definition);
    assert_eq!(item.span, span(definition));
    assert_eq!(item.signature.as_deref(), Some("def run():\n pass"));
    assert_eq!(item.decorators, ["@decorate"]);
    Ok(())
}
