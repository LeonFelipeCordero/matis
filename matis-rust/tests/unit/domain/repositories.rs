use super::*;
use anyhow::Result;
use serde_json::json;

struct SavedFacts(Option<Value>);

impl RepositoryProtocol for SavedFacts {
    fn report_facts(&mut self, _: &str) -> Result<Option<ReportFactsDto>> {
        Ok(self.0.clone())
    }
    fn register(&mut self, _: &str, _: &str) -> Result<RepositoryDto> {
        unreachable!()
    }
    fn list(&mut self) -> Result<Vec<RepositoryDto>> {
        unreachable!()
    }
    fn exists(&mut self, _: &str) -> Result<bool> {
        unreachable!()
    }
    fn resolve(&mut self, _: &str) -> Result<Option<String>> {
        unreachable!()
    }
    fn graph(&mut self, _: &str) -> Result<Option<GraphDto>> {
        unreachable!()
    }
    fn save(&mut self, _: &str, _: &Value) -> Result<GraphDto> {
        unreachable!()
    }
    fn save_new(&mut self, _: &str, _: &str, _: &Value) -> Result<GraphDto> {
        unreachable!()
    }
}

#[test]
fn candidate_fallback_distinguishes_absent_graph_legacy_data_and_malformed_fields() -> Result<()> {
    assert_eq!(SavedFacts(None).mutation_candidates("id")?, None);
    assert_eq!(SavedFacts(Some(json!({}))).mutation_candidates("id")?, Some(vec![]));
    let candidates = vec![json!({"id": "second"}), json!({"id": "first"})];
    let facts = json!({"mutationCandidates": candidates});
    assert_eq!(SavedFacts(Some(facts)).mutation_candidates("id")?, Some(candidates));
    for invalid in [Value::Null, json!({}), json!("candidates")] {
        let error = SavedFacts(Some(json!({"mutationCandidates": invalid}))).mutation_candidates("id").unwrap_err();
        assert_eq!(error.to_string(), "Invalid mutationCandidates: expected an array");
    }
    Ok(())
}
