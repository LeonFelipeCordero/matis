use matis::{
    domain::repositories::RepositoryProtocol, persistence::repositories::repository_store,
};
use serde_json::Value;
use std::{env, error::Error, fs, path::Path};

// The public CLI has bounded graph reads, not full artifact export or fixture writes.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let output = match args.as_slice() {
        [command, source] if command == "analyze" => {
            matis::services::repositories::analyze_folder(Path::new(source))?
        }
        [command, file, name] if command == "save" => {
            let artifact: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
            let graph = repository_store(None)?.save_new(name, name, &artifact)?;
            Value::String(graph.repository_id)
        }
        [command, file] if command == "cycles" => {
            let artifact: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
            matis::metrics::cycles::circular_dependencies(
                artifact["nodes"]
                    .as_array()
                    .ok_or("nodes must be an array")?,
                artifact["edges"]
                    .as_array()
                    .ok_or("edges must be an array")?,
            )["groups"]
                .clone()
        }
        _ => return Err("expected analyze SOURCE, save FILE NAME, or cycles FILE".into()),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
