//! Complete extraction, analysis, and graph parity, including structures and grammar variants.
#[path = "helpers.rs"]
mod pipeline;
use crate::support;
use support::{assert_json, fixtures, snapshot};

macro_rules! pipeline_tests {
    ($($language:ident),+ $(,)?) => {$ (
        mod $language {
            use super::*;
            fn expected(stage: &str) -> serde_json::Value {
                snapshot(&format!("coffee/{}/{stage}.json", stringify!($language)))
            }
            // The full bundle includes files, all three structure seams, excluded sources, and grammar variants.
            #[test]
            fn complete_pipeline() {
                let bundle = pipeline::project(&fixtures().join(stringify!($language)));
                assert_json(&bundle.extraction, &expected("extraction"), "/extraction");
                assert_json(&bundle.analysis, &expected("analysis"), "/analysis");
                assert_json(&bundle.graph, &expected("graph"), "/graph");
            }
        }
    )+};
}

pipeline_tests!(java, kotlin, go, rust, javascript, typescript, elixir, python, tsx);
