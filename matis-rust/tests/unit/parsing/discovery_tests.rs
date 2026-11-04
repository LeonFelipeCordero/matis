//! Rule-policy tests need paths and ignore specs, not a filesystem or parser.
use super::*;

fn rules(base: &str, text: &str) -> Rc<Rules> {
    Rc::new(Rules { base: base.into(), spec: IgnoreSpec::new(text).unwrap() })
}

#[test]
fn child_contexts_share_rules_without_leaking_local_overrides() {
    let parent = DirectoryContext {
        base: "/repo".into(),
        git: vec![rules("/repo", "*.py\n")],
        matis: vec![rules("/repo", "blocked.py\n")],
        use_gitignore: true,
    };
    let mut child = DirectoryContext { base: "/repo/src".into(), ..parent.clone() };
    assert!(Rc::ptr_eq(&parent.git[0], &child.git[0]));
    assert!(Rc::ptr_eq(&parent.matis[0], &child.matis[0]));
    child.git.push(rules("/repo/src", "!keep.py\n!blocked.py\n"));
    let sibling = DirectoryContext { base: "/repo/other".into(), ..parent.clone() };
    assert!(child.include_file(Path::new("/repo/src/keep.py")));
    assert!(!child.include_file(Path::new("/repo/src/blocked.py")));
    assert!(!parent.include_file(Path::new("/repo/keep.py")));
    assert!(!sibling.include_file(Path::new("/repo/other/keep.py")));
}

#[test]
#[ignore = "focused repeatable timing; run with --ignored --nocapture --test-threads=1"]
fn benchmark_inherited_rule_cloning() {
    let patterns = (0..32).map(|i| format!("ignored-{i}/\n!keep-{i}.py\n")).collect::<String>();
    let context = DirectoryContext {
        base: "/repo/src".into(),
        git: (0..4).map(|_| rules("/repo", &patterns)).collect(),
        matis: vec![rules("/repo", &patterns)],
        use_gitignore: true,
    };
    let start = std::time::Instant::now();
    for _ in 0..500 {
        std::hint::black_box(std::hint::black_box(&context).clone());
    }
    eprintln!("500 directory-context clones, 320 patterns: {:?}", start.elapsed());
}

#[test]
fn nested_rules_only_override_matching_parent_rules() {
    let inherited = vec![rules("/repo", "*.py\ncache/\n")];
    let mut nested = inherited.clone();
    nested.push(rules("/repo/src", "!keep.py\nunrelated.txt\n"));
    for (name, directory, ignored) in [
        ("keep.py", false, false),
        ("drop.py", false, true),
        ("other.txt", false, false),
        ("cache", true, true),
        ("cache", false, false),
    ] {
        let path = Path::new("/repo/src").join(name);
        assert_eq!(is_ignored(&path, &nested, directory), ignored, "{name}");
    }
    assert!(is_ignored(Path::new("/repo/src/keep.py"), &inherited, false));
}

#[test]
fn git_mode_replaces_defaults_but_cannot_override_matis_or_metadata_exclusions() {
    assert_directory_policy(false);
    assert_directory_policy(true);
}

fn assert_directory_policy(use_gitignore: bool) {
    let context = DirectoryContext {
        base: "/repo".into(),
        git: vec![rules("/repo", "git-only/\n!matis-only/\n!.git/\n")],
        matis: vec![rules("/repo", "matis-only/\n!node_modules/\n")],
        use_gitignore,
    };
    for (name, expected) in [
        (".git", false),
        ("node_modules", use_gitignore),
        ("my-generated-code", use_gitignore),
        ("git-only", !use_gitignore),
        ("matis-only", false),
        ("src", true),
    ] {
        assert_eq!(context.include_directory(&context.base.join(name)), expected, "{name}");
    }
    if !use_gitignore {
        for name in IGNORED_DIRECTORIES {
            assert!(!context.include_directory(&context.base.join(name)), "{name}");
        }
    }
}

#[test]
fn files_require_both_rule_sets_to_allow_them() {
    let context = DirectoryContext {
        base: "/repo".into(),
        git: vec![rules("/repo", "*.py\n!keep.py\n!matis.py\n")],
        matis: vec![rules("/repo", "matis.py\n!drop.py\n!.git\n")],
        use_gitignore: true,
    };
    for (name, included) in
        [(".git", false), ("keep.py", true), ("drop.py", false), ("matis.py", false), ("note.txt", true)]
    {
        assert_eq!(context.include_file(&context.base.join(name)), included, "{name}");
    }
}
