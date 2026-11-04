pub mod repositories {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "repositories")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub location: String,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use repositories::Model as RepositoryRow;
impl sea_orm::Related<repository_graphs::Entity> for repositories::Entity {
    /// Define the registration-to-graph join by repository ID without executing a query.
    fn to() -> sea_orm::RelationDef {
        use sea_orm::EntityTrait;
        repositories::Entity::belongs_to(repository_graphs::Entity)
            .from(repositories::Column::Id)
            .to(repository_graphs::Column::RepositoryId)
            .into()
    }
}
pub mod repository_graphs {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "repository_graphs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        pub imported_at: i64,
        pub graph_revision: Option<String>,
        pub schema_version: Option<i64>,
        pub indexed_at: Option<String>,
        pub root_name: Option<String>,
        pub root_path: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use repository_graphs::Model as RepositoryGraphRow;
pub mod graph_nodes {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_nodes")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub node_key: String,
        pub parent_key: Option<String>,
        pub ordinal: i64,
        #[sea_orm(column_name = "type")]
        pub type_: String,
        pub kind: String,
        pub level: String,
        pub name: String,
        pub language: Option<String>,
        pub qualified_name: Option<String>,
        pub path: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_nodes::Model as GraphNodeRow;
impl sea_orm::Related<graph_symbols::Entity> for graph_nodes::Entity {
    /// Define the node-to-symbol join by repository ID and node key without querying.
    fn to() -> sea_orm::RelationDef {
        use sea_orm::EntityTrait;
        graph_nodes::Entity::belongs_to(graph_symbols::Entity)
            .from((graph_nodes::Column::RepositoryId, graph_nodes::Column::NodeKey))
            .to((graph_symbols::Column::RepositoryId, graph_symbols::Column::NodeKey))
            .into()
    }
}
impl sea_orm::Related<graph_files::Entity> for graph_nodes::Entity {
    /// Define the node-to-file join by repository ID and node key without querying.
    fn to() -> sea_orm::RelationDef {
        use sea_orm::EntityTrait;
        graph_nodes::Entity::belongs_to(graph_files::Entity)
            .from((graph_nodes::Column::RepositoryId, graph_nodes::Column::NodeKey))
            .to((graph_files::Column::RepositoryId, graph_files::Column::NodeKey))
            .into()
    }
}
pub mod graph_files {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_files")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub node_key: String,
        pub byte_count: Option<i64>,
        pub sha256: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_files::Model as GraphFileRow;
pub mod graph_symbols {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_symbols")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub node_key: String,
        pub start_line: Option<i64>,
        pub start_column: Option<i64>,
        pub end_line: Option<i64>,
        pub end_column: Option<i64>,
        pub signature: Option<String>,
        pub visibility: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_symbols::Model as GraphSymbolRow;
pub mod graph_node_metrics {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_node_metrics")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub node_key: String,
        pub line_count: Option<i64>,
        pub parameter_count: Option<i64>,
        pub base_complexity: Option<i64>,
        pub nested_complexity: Option<i64>,
        pub loop_complexity: Option<i64>,
        pub total_complexity: Option<i64>,
        pub max_decision_depth: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_node_metrics::Model as GraphNodeMetricRow;
pub mod graph_node_decorators {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_node_decorators")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub node_key: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub ordinal: i64,
        pub decorator: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_node_decorators::Model as GraphNodeDecoratorRow;
pub mod graph_edges {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_edges")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub ordinal: i64,
        pub source_key: String,
        pub target_key: String,
        pub relationship: String,
        pub confidence: String,
        pub path: Option<String>,
        pub evidence: Option<String>,
        pub line: Option<i64>,
        pub type_only: bool,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_edges::Model as GraphEdgeRow;
pub mod graph_diagnostics {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_diagnostics")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub ordinal: i64,
        pub severity: String,
        pub path: String,
        pub language: String,
        pub message: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_diagnostics::Model as GraphDiagnosticRow;
pub mod graph_mutation_candidates {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "graph_mutation_candidates")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub repository_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub candidate_id: String,
        pub file_key: String,
        pub ordinal: i64,
        pub path: String,
        pub language: String,
        pub source_hash: String,
        pub kind: String,
        pub operator: Option<String>,
        pub payload: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
pub use graph_mutation_candidates::Model as GraphMutationCandidateRow;
