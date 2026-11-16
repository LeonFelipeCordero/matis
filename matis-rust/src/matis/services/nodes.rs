use crate::domain::{
    navigation::{ChildrenRequest, error},
    storage::{DecoratorPort, FilePort, NodePort, SymbolPort},
};
use anyhow::Result;
use serde_json::{Value, json};
pub struct NodeRepositories {
    pub nodes: Box<dyn NodePort>,
    pub files: Box<dyn FilePort>,
    pub symbols: Box<dyn SymbolPort>,
    pub decorators: Box<dyn DecoratorPort>,
}
pub struct NodeService {
    nodes: Box<dyn NodePort>,
    files: Box<dyn FilePort>,
    symbols: Box<dyn SymbolPort>,
    decorators: Box<dyn DecoratorPort>,
}
impl NodeService {
    pub fn new(repositories: NodeRepositories) -> Self {
        let NodeRepositories { nodes, files, symbols, decorators } = repositories;
        Self { nodes, files, symbols, decorators }
    }
    fn symbols(&self, id: &str, mut nodes: Vec<Value>) -> Result<Vec<Value>> {
        let keys: Vec<_> = nodes.iter().map(|n| n["stableKey"].as_str().unwrap().to_owned()).collect();
        let mut symbols = self.symbols.get_many(id, &keys)?;
        for node in &mut nodes {
            if let Some(symbol) = symbols.remove(node["stableKey"].as_str().unwrap()) {
                node.as_object_mut().unwrap().extend(symbol.as_object().unwrap().clone());
            }
        }
        Ok(nodes)
    }
    pub(crate) fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.symbols(id, self.nodes.all(id)?)
    }
    pub(crate) fn levels(&self, id: &str) -> Result<Vec<(String, i64)>> {
        self.nodes.levels(id)
    }
    pub(crate) fn symbol_count(&self, id: &str) -> Result<i64> {
        self.symbols.count(id)
    }
    pub(crate) fn roots(&self, id: &str, limit: usize) -> Result<Vec<Value>> {
        self.symbols(id, self.nodes.roots(id, limit)?)
    }
    pub(crate) fn find(&self, id: &str, query: &str, limit: usize) -> Result<Vec<Value>> {
        self.symbols(id, self.nodes.find(id, query, limit)?)
    }
    pub(crate) fn children(&self, id: &str, request: &ChildrenRequest<'_>) -> Result<Vec<Value>> {
        self.symbols(id, self.nodes.children(id, request)?)
    }
    pub(crate) fn node(&self, id: &str, key: &str) -> Result<Value> {
        let node = self
            .nodes
            .get(id, key)?
            .ok_or_else(|| error(format!("Node not found: {key}. Use find to obtain a stableKey.")))?;
        let mut node = self.symbols(id, vec![node])?.remove(0);
        node["external"] = json!(node["level"] == "external");
        node["file"] = self.files.get_many(id, &[key.to_owned()])?.remove(key).unwrap_or(Value::Null);
        let mut decorators = self.decorators.get(id, key, 101)?;
        node["decoratorsTruncated"] = json!(decorators.len() > 100);
        decorators.truncate(100);
        node["decorators"] = json!(decorators);
        Ok(node)
    }
    pub(crate) fn file(&self, id: &str, path: &str) -> Result<Vec<Value>> {
        let nodes = self.nodes.files_at(id, path, 2)?;
        let keys: Vec<_> = nodes.iter().map(|n| n["stableKey"].as_str().unwrap().to_owned()).collect();
        let files = self.files.get_many(id, &keys)?;
        Ok(nodes
            .into_iter()
            .filter_map(|n| {
                let key = n["stableKey"].as_str().unwrap();
                files.get(key).map(|f| json!({"stableKey":key,"sha256":f["sha256"]}))
            })
            .collect())
    }
    pub(crate) fn replace(&self, id: &str, artifact: &Value) -> Result<()> {
        self.nodes.clear(id)?;
        self.nodes.insert(id, artifact)?;
        self.files.insert(id, artifact)?;
        self.symbols.insert(id, artifact)?;
        self.decorators.insert(id, artifact)
    }
}
