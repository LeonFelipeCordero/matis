//! Native parser integration coverage for the Python and script adapter group.
use crate::domain::models::{Call, ParsedFile, ParsedRepository, SourceFile, Symbol};
use crate::metrics::adapters::resolution::{CallableIdentity, ImportBindings, Modules};
use crate::metrics::adapters::{adapter_for_language, javascript, python};
use crate::metrics::resolution_contract::{ImportRequest, ResolutionRequest};
use crate::metrics::resolution_syntax::Syntax;
use crate::metrics::resolver::CallResolver;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn file(path: &str, language: &str, source: &str) -> ParsedFile {
    crate::parsing::parser::parse_file(SourceFile {
        path: path.into(),
        language: language.into(),
        absolute_path: PathBuf::from("/matis-script-tests").join(path),
        source: source.as_bytes().to_vec(),
    })
    .unwrap()
}
fn syntax(files: &[ParsedFile]) -> BTreeMap<String, Syntax> {
    files.iter().map(|f| (f.source.path.clone(), Syntax::new(f, adapter_for_language(&f.source.language)))).collect()
}
fn imports(files: Vec<ParsedFile>) -> ImportBindings {
    let syntax = syntax(&files);
    let repository = ParsedRepository { files };
    ImportBindings::new(&repository, &syntax, &Modules::new(&repository, &syntax))
}
fn names(symbols: &[Symbol]) -> Vec<&str> {
    symbols.iter().map(|s| s.name.as_str()).collect()
}
fn call(file: &ParsedFile, name: &str) -> Call {
    Call {
        file_path: file.source.path.clone(),
        name: name.into(),
        start_byte: 0,
        end_byte: 0,
        line: 1,
        evidence: format!("{name}()"),
        bare_target: true,
        qualifier: None,
        relationship: "CALLS".into(),
    }
}
fn selected(file: &ParsedFile, name: &str) -> Vec<Symbol> {
    let syntax = syntax(std::slice::from_ref(file));
    let identity = CallableIdentity::new(&ParsedRepository { files: vec![file.clone()] }, &syntax);
    identity.select(file, &call(file, name), file.symbols.iter().filter(|s| s.name == name).cloned().collect())
}
fn request<'a>(file: &'a ParsedFile, path: &str, source: &str) -> ImportRequest<'a> {
    let syntax = Syntax::new(file, adapter_for_language(&file.source.language));
    let position = syntax.nodes().into_iter().find(|n| syntax.text(*n) == source).map(|n| n.start_byte());
    ImportRequest { file, path: path.into(), source: source.into(), position }
}

#[test]
fn python_indexes_package_and_src_roots_without_guessing_absolute_children() {
    let main = file("main.py", "python", "");
    let index = imports(vec![
        main.clone(),
        file("lib/pkg/__init__.py", "python", ""),
        file("lib/pkg/tools.py", "python", "def run(): pass"),
        file("src/standalone.py", "python", ""),
        file("plain.py", "python", ""),
        file("plain/child.py", "python", ""),
    ]);
    assert_eq!(index.files(&main, "pkg.tools", None)[0].source.path, "lib/pkg/tools.py");
    assert_eq!(index.files(&main, "standalone", None)[0].source.path, "src/standalone.py");
    assert!(index.files(&main, "plain.child", None).is_empty());
    let duplicate =
        imports(vec![main.clone(), file("a/pkg/__init__.py", "python", ""), file("b/pkg/__init__.py", "python", "")]);
    assert!(duplicate.files(&main, "pkg", None).is_empty());
}
#[test]
fn python_relative_imports_prefer_declared_and_assigned_attributes_over_children() {
    let main = file("pkg/main.py", "python", "from . import tools");
    let tools = file("pkg/tools.py", "python", "def run(): pass");
    for source in ["def tools(): pass", "tools = object()", "import elsewhere as tools"] {
        let ctx = imports(vec![main.clone(), tools.clone(), file("pkg/__init__.py", "python", source)]);
        let entries = python::import_item(&ctx, &request(&main, ".", "from . import tools"));
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].2);
        assert_eq!(entries[0].1.len(), usize::from(source.starts_with("def")));
    }
    let ctx = imports(vec![main.clone(), tools, file("pkg/__init__.py", "python", "tools: object")]);
    let req = request(&main, ".", "from . import tools");
    let entries = python::import_item(&ctx, &req);
    assert!(entries[0].2);
    assert_eq!(names(&entries[0].1), ["run"]);
    assert_eq!(python::import_targets(&ctx, &req), vec![(".tools".into(), Some("file:pkg/tools.py".into()))]);
}
#[test]
fn python_import_targets_expand_multiple_modules_and_keep_unresolved_keys() {
    let main = file("main.py", "python", "import first as local, absent");
    let ctx = imports(vec![main.clone(), file("first.py", "python", "def run(): pass")]);
    let req = request(&main, "first", "import first as local, absent");
    assert_eq!(
        python::import_targets(&ctx, &req),
        vec![("first".into(), Some("file:first.py".into())), ("absent".into(), None)]
    );
    let entries = python::import_item(&ctx, &req);
    assert_eq!(entries[0].0, "local");
    assert_eq!(names(&entries[0].1), ["run"]);
    assert_eq!(entries[1].0, "absent");
}
#[test]
fn python_overloads_require_proven_stable_imports_and_last_runtime_body() {
    let valid = file(
        "a.py",
        "python",
        "from typing import overload as ov\n@ov\ndef run(x: int): ...\n@ov\ndef run(x: str): ...\ndef run(x): return x\n",
    );
    assert_eq!(selected(&valid, "run").len(), 1);
    assert_eq!(
        selected(&valid, "run")[0].start_byte,
        String::from_utf8_lossy(&valid.source.source).rfind("def run").unwrap()
    );
    for source in [
        "from typing import overload\n@overload\ndef run(x): ...",
        "def run(x): pass\ndef run(x): pass",
        "from typing import overload\noverload = replacement\n@overload\ndef run(x): ...\ndef run(x): pass",
        "import typing\ntyping.overload = replacement\n@typing.overload\ndef run(x): ...\ndef run(x): pass",
        "from elsewhere import overload\n@overload\ndef run(x): ...\ndef run(x): pass",
    ] {
        assert!(selected(&file("a.py", "python", source), "run").is_empty(), "{source}");
    }
}
#[test]
fn python_self_calls_require_instance_owner_and_unwritten_first_self() {
    for (header, body, allowed) in [
        ("", "", true),
        ("@staticmethod\n ", "", false),
        ("@classmethod\n ", "", false),
        ("", "self = other\n  ", false),
    ] {
        let source = format!("class C:\n def run(self): pass\n {header}def caller(self):\n  {body}self.run()\n");
        let file = file("a.py", "python", &source);
        let syntax = syntax(std::slice::from_ref(&file));
        let identity = CallableIdentity::new(&ParsedRepository { files: vec![file.clone()] }, &syntax);
        let mut site = call(&file, "run");
        site.start_byte = source.rfind("self.run()").unwrap();
        site.end_byte = site.start_byte + "self.run()".len();
        site.qualifier = Some("self".into());
        assert_eq!(
            !identity
                .select(&file, &site, file.symbols.iter().filter(|s| s.name == "run").cloned().collect())
                .is_empty(),
            allowed,
            "{source}"
        );
    }
}
#[test]
fn script_exports_keep_runtime_aliases_default_and_namespace_member_names() {
    let target = file(
        "tools.ts",
        "typescript",
        "function run() {}\nfunction hidden() {}\nexport {run as execute, run as execute};\nexport default run;\nexport type {hidden};\nexport {hidden as remote} from './remote';",
    );
    let main = file(
        "main.ts",
        "typescript",
        "import Default, {execute as local, type hidden} from './tools';\nimport * as tools from './tools';",
    );
    let ctx = imports(vec![main.clone(), target.clone()]);
    let exports = javascript::export_bindings(&target, &ctx.syntax["tools.ts"]);
    assert_eq!(exports.keys().map(String::as_str).collect::<Vec<_>>(), ["default", "execute"]);
    assert_eq!(names(&exports["execute"]), ["run"]);
    let named = ctx.get(&main, "local", 0).unwrap();
    assert_eq!(names(&named.targets), ["run"]);
    assert!(ctx.get(&main, "hidden", 0).unwrap().targets.is_empty());
    let namespace = ctx.get(&main, "tools", 0).unwrap();
    assert_eq!(names(&namespace.member("execute")), ["run"]);
    assert!(namespace.member("run").is_empty());
}
#[test]
fn script_relative_imports_resolve_source_and_emitted_extensions() {
    for ((specifier, path, language), importer) in [
        ("./tools.js", "tools.ts", "typescript"),
        ("./tools.js", "tools.tsx", "tsx"),
        ("./tools.jsx", "tools.tsx", "tsx"),
        ("./tools.ts", "tools.ts", "typescript"),
        ("./tools.tsx", "tools.tsx", "tsx"),
        ("./tools.js", "tools.js", "javascript"),
        ("./tools", "tools.ts", "typescript"),
        ("./tools/index.js", "tools/index.ts", "typescript"),
    ]
    .into_iter()
    .flat_map(|case| ["javascript", "typescript", "tsx"].map(|importer| (case, importer)))
    {
        let main = file("main.ts", importer, &format!("import {{run}} from '{specifier}';"));
        let ctx = imports(vec![main.clone(), file(path, language, "export function run() {}")]);
        assert_eq!(ctx.files(&main, specifier, None)[0].source.path, path, "{importer}: {specifier}");
        assert_eq!(names(&ctx.get(&main, "run", 0).unwrap().targets), ["run"]);
    }
}
#[test]
fn script_extension_substitution_preserves_exact_paths_and_rejects_guesses() {
    let main = file("main.ts", "typescript", "");
    let ts = file("tools.ts", "typescript", "");
    let tsx = file("tools.tsx", "tsx", "");
    let js = file("tools.js", "javascript", "");
    assert_eq!(javascript::paths(&main, "./tools.js", &[ts.clone(), js.clone()])[0].source.path, "tools.js");
    assert!(
        javascript::paths(&main, "./tools.js", &[js, file("tools.js.ts", "typescript", ""), ts.clone()]).is_empty()
    );
    assert!(javascript::paths(&main, "./tools.js", &[ts.clone(), tsx.clone()]).is_empty());
    assert!(javascript::paths(&main, "./tools", &[ts.clone(), tsx]).is_empty());
    for specifier in ["./tools.jsx", "./tools.json", "./missing.js", "tools.js", "package/tools.js"] {
        assert!(javascript::paths(&main, specifier, std::slice::from_ref(&ts)).is_empty(), "{specifier}");
    }
    assert!(javascript::paths(&main, "./tools.js", &[file("tools.py", "python", "")]).is_empty());
}
#[test]
fn script_namespace_targets_keep_export_insertion_order() {
    let target = file(
        "tools.js",
        "javascript",
        "export function zebra() {}\nexport function alpha() {}\nexport {zebra as again};",
    );
    let main = file("main.js", "javascript", "import * as tools from './tools';");
    let ctx = imports(vec![main.clone(), target]);
    let entries = javascript::import_item(&ctx, &request(&main, "./tools", "import * as tools from './tools';"));
    assert_eq!(names(&entries[0].1), ["zebra", "alpha", "zebra"]);
}
#[test]
fn script_selection_uses_runtime_method_bodies_and_equal_static_status() {
    let js = file("a.js", "javascript", "class C { run() { return 1; } run() { return 2; } }");
    let selected_js = selected(&js, "run");
    assert_eq!(selected_js.len(), 1);
    assert_eq!(selected_js[0].start_byte, String::from_utf8_lossy(&js.source.source).rfind("run()").unwrap());
    let ts = file(
        "a.ts",
        "typescript",
        "function run(x: string): void;\nfunction run(x: number): void;\nfunction run(x: unknown) {}\n",
    );
    assert_eq!(selected(&ts, "run").len(), 1);
    for (language, source) in [
        ("typescript", "function run(x: string): void;"),
        ("typescript", "class C { run(x: string): void; static run(x: unknown) {} }"),
        ("javascript", "class C { run() {} static run() {} }"),
        ("javascript", "class C { get run() {} run() {} }"),
    ] {
        assert!(selected(&file("a.ts", language, source), "run").is_empty(), "{source}");
    }
}
#[test]
fn script_inheritance_rejects_receiver_alias_writes_and_own_fields() {
    for (language, base, child, allowed) in [
        ("python", "class Base:\n def run(self): pass\n", "class Child(Base): pass\n", true),
        (
            "python",
            "class Base:\n def __init__(me):\n  alias = me\n  alias.run = other\n def run(self): pass\n",
            "class Child(Base):\n def run(self): pass\n",
            false,
        ),
        ("javascript", "class Base { run() {} }\n", "class Child extends Base {}", true),
        ("javascript", "class Base { run() {} }\n", "class Child extends Base { run = other; }", false),
        (
            "javascript",
            "class Base { constructor() { const alias = this; alias[key] = value; } run() {} }\n",
            "class Child extends Base {}",
            false,
        ),
        (
            "javascript",
            "class Base { run() {} }\n",
            "class Child extends Base {}\nconst proto = Child.prototype; Object.assign(proto, changes);",
            false,
        ),
    ] {
        let file = file("a.py", language, &format!("{base}{child}"));
        let resolver = CallResolver::new(&ParsedRepository { files: vec![file.clone()] });
        let class = file.symbols.iter().find(|s| s.name == "Child").unwrap();
        let members = file.source.language.as_str();
        assert_eq!(
            !adapter_for_language(members).instance_members(&resolver, class, "run").is_empty(),
            allowed,
            "{language}: {base}{child}"
        );
    }
}
#[test]
fn python_comprehension_iterables_and_nested_methods_hide_class_bindings() {
    let source = "class C:\n x = 1\n values = [x for x in range(x)]\n def f(self):\n  return x\n";
    let file = file("a.py", "python", source);
    let syntax = Syntax::new(&file, adapter_for_language("python"));
    let class = syntax.nodes().into_iter().find(|n| n.kind() == "class_definition").unwrap();
    let comprehension = syntax.nodes().into_iter().find(|n| n.kind() == "list_comprehension").unwrap();
    let iterable = source.find("range(x)").unwrap();
    assert!(!syntax.visible(comprehension, iterable));
    assert!(syntax.visible(class, iterable));
    assert!(!syntax.visible(class, source.find("return x").unwrap() + 7));
    assert!(!syntax.visible(class, source.find("[x").unwrap() + 1));
}
#[test]
fn script_implicit_this_allows_only_nested_arrow_functions() {
    for (nested, allowed) in [("const f = () => this.run();", true), ("function f() { this.run(); }", false)] {
        let source = format!("class C {{ run() {{}} caller() {{ {nested} }} }}");
        let file = file("a.js", "javascript", &source);
        let resolver = CallResolver::new(&ParsedRepository { files: vec![file.clone()] });
        let method = file.symbols.iter().find(|s| s.name == "caller").unwrap();
        let mut site = call(&file, "run");
        site.start_byte = source.find("this.run()").unwrap();
        site.end_byte = site.start_byte + "this.run()".len();
        let req = ResolutionRequest {
            file: &file,
            call: &site,
            owner: resolver.owner(&site),
            name: "run".into(),
            qualifier: Some("this".into()),
        };
        assert_eq!(javascript::implicit_receiver(&resolver, &req, method), allowed);
    }
}
