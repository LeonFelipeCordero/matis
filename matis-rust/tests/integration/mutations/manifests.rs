use super::*;
use crate::{
    domain::{models::SourceFile, mutation_candidates::MutationCandidate},
    parsing::mutation_candidates::extract_candidates,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}
fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn source(path: &str, language: &str, data: &[u8]) -> SourceFile {
    SourceFile { path: path.into(), absolute_path: path.into(), language: language.into(), source: data.to_vec() }
}
fn candidates(root: &Path, items: &[Value]) -> Result<Vec<Value>> {
    let paths: BTreeSet<_> =
        items.iter().map(|item| (item["path"].as_str().unwrap(), item["language"].as_str().unwrap())).collect();
    let mut result = Vec::new();
    for (path, language) in paths {
        result.extend(extract_candidates(&source(path, language, &std::fs::read(root.join(path))?))?);
    }
    result.sort_by(|a, b| {
        (
            a["path"].as_str(),
            a["span"]["startByte"].as_i64(),
            a["span"]["endByte"].as_i64(),
            a["kind"].as_str(),
            a["id"].as_str(),
        )
            .cmp(&(
                b["path"].as_str(),
                b["span"]["startByte"].as_i64(),
                b["span"]["endByte"].as_i64(),
                b["kind"].as_str(),
                b["id"].as_str(),
            ))
    });
    Ok(result)
}

#[test]
fn all_coffee_candidate_and_markdown_snapshots() -> Result<()> {
    for language in rules::LANGUAGES {
        let graph = read_json(fixtures().join(format!("snapshots/coffee/{language}/graph.json")));
        let root = fixtures().join(format!("code_samples/{language}"));
        let expected = graph["mutationCandidates"].as_array().unwrap();
        let files: Vec<_> =
            graph["nodes"].as_array().unwrap().iter().filter(|node| node["kind"] == "file").cloned().collect();
        let found = candidates(&root, &files)?;
        assert_eq!(&found, expected, "candidate snapshot for {language}");
        let cli = read_json(fixtures().join(format!("snapshots/coffee/{language}/cli.json")));
        let markdown = cli["report-markdown"]["output"].as_str().unwrap();
        let expected = markdown.split_once("## Mutation candidates").map(|(_, rest)| rest).unwrap_or("");
        let actual = report::render_candidates_markdown(&json!({"mutationCandidates": found}));
        assert_eq!(
            format!("{actual}\n").strip_prefix("## Mutation candidates").unwrap_or(""),
            expected,
            "Markdown {language}"
        );
    }
    Ok(())
}

#[test]
fn all_manifest_snapshots() -> Result<()> {
    let selected = config::MutationConfig {
        rules: rules::REGISTRY.iter().map(|rule| rule.id.clone()).collect(),
        ..Default::default()
    };
    let samples = rules::LANGUAGES.iter().flat_map(|language| [false, true].map(|operators| (*language, operators)));
    for (language, operators) in samples {
        let (sample, suffix) =
            if operators { (format!("mutations/{language}"), "-operators") } else { (language.to_string(), "") };
        let root = fixtures().join(format!("code_samples/{sample}"));
        let expected = read_json(fixtures().join(format!("snapshots/mutations/{language}{suffix}.json")));
        let found = candidates(&root, expected["mutations"].as_array().unwrap())?;
        let actual = report::build_mutation_report(&root, &json!({"mutationCandidates": found}), &selected)?;
        assert_eq!(actual, expected, "manifest snapshot for {sample}");
    }
    Ok(())
}

fn temp_candidates(data: &[u8], language: &str) -> Result<(tempfile::TempDir, Vec<MutationCandidate>)> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("sample"), data)?;
    let found = extract_candidates(&source("sample", language, data))?
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok((dir, found))
}

#[test]
fn validates_all_fields_and_sources_even_when_disabled() -> Result<()> {
    let (root, found) = temp_candidates(b"def brew(a):\n    return a + 2\n", "python")?;
    assert_eq!(found.len(), 1);
    let original = &found[0];
    for field in [
        "id",
        "path",
        "language",
        "sourceHash",
        "kind",
        "operator",
        "span",
        "expressionSpan",
        "originalText",
        "expressionText",
        "callable",
    ] {
        let mut value = serde_json::to_value(original)?;
        value[field] = match field {
            "span" | "expressionSpan" => {
                let mut span = value[field].clone();
                span["startByte"] = json!(-1);
                span
            }
            "callable" => Value::Null,
            "kind" => json!("logical"),
            _ => json!("forged"),
        };
        let forged = serde_json::from_value(value)?;
        assert!(manifests::generate_manifests(root.path(), &[forged], &[]).is_err(), "field {field}");
    }
    std::fs::write(root.path().join("sample"), b"# changed\n")?;
    assert!(
        manifests::generate_manifests(root.path(), &found, &[])
            .unwrap_err()
            .to_string()
            .starts_with("Stale or invalid mutation candidate")
    );
    std::fs::remove_file(root.path().join("sample"))?;
    assert!(
        manifests::generate_manifests(root.path(), &found, &[])
            .unwrap_err()
            .to_string()
            .starts_with("Cannot read mutation source")
    );
    Ok(())
}

#[test]
fn unsafe_paths_are_rejected() -> Result<()> {
    let (root, found) = temp_candidates(b"x = a + b\n", "python")?;
    for path in ["/sample", "../sample", "a/../sample", "./sample", "a//sample", "sample/", "a/./sample", ""] {
        let mut candidate = found[0].clone();
        candidate.path = path.into();
        let error = manifests::generate_manifests(root.path(), &[candidate], &[]).unwrap_err();
        assert!(error.to_string().starts_with("Unsafe mutation path"), "{path}: {error}");
    }
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("source"), b"a+b")?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path().join("source"), root.path().join("link"))?;
        let mut candidate = found[0].clone();
        candidate.path = "link".into();
        assert!(
            manifests::generate_manifests(root.path(), &[candidate], &[])
                .unwrap_err()
                .to_string()
                .starts_with("Mutation path escapes root")
        );
    }
    Ok(())
}

#[test]
fn strict_candidate_validation_and_empty_input() -> Result<()> {
    let (root, found) = temp_candidates(b"a + b\n", "python")?;
    let value = serde_json::to_value(&found[0])?;
    let rules = Vec::new();
    for invalid in [json!([]), json!(null), json!({}), json!({"id": "invalid"})] {
        assert!(manifests::generate_manifest_values(root.path(), &[invalid], &rules).is_err());
    }
    for field in ["span", "expressionSpan", "callable"] {
        let mut invalid = value.clone();
        invalid[field] = json!([]);
        assert!(manifests::generate_manifest_values(root.path(), &[invalid], &rules).is_err());
    }
    let mut extra = value.clone();
    extra["future"] = json!(true);
    assert!(manifests::generate_manifest_values(root.path(), &[extra], &rules)?.is_empty());
    assert!(manifests::generate_manifests(&root.path().join("absent"), &[], &[])?.is_empty());
    assert!(report::build_mutation_report(root.path(), &json!({}), &config::MutationConfig::default()).is_err());
    Ok(())
}

#[test]
fn unary_dedup_and_lexical_collisions() -> Result<()> {
    let (root, found) = temp_candidates(b"x = -a + b\n", "python")?;
    let selected: Vec<_> = rules::REGISTRY.iter().map(|rule| rule.id.clone()).collect();
    let mut repeated = found.clone();
    repeated.extend(found);
    let manifests = manifests::generate_manifests(root.path(), &repeated, &selected)?;
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].replacement, "-");
    let (root, found) = temp_candidates(b"const x = a+--b;", "javascript")?;
    let result = manifests::generate_manifests(root.path(), &found, &["arithmetic.add-to-subtract".into()]);
    assert!(result.unwrap_err().to_string().starts_with("Unsafe mutation replacement"));
    Ok(())
}
