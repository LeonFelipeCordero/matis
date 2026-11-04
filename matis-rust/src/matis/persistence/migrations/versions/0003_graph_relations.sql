CREATE TABLE graph_node_metrics (
	repository_id TEXT NOT NULL, 
	node_key TEXT NOT NULL, 
	line_count INTEGER, 
	parameter_count INTEGER, 
	base_complexity INTEGER, 
	nested_complexity INTEGER, 
	loop_complexity INTEGER, 
	total_complexity INTEGER, 
	max_decision_depth INTEGER, 
	PRIMARY KEY (repository_id, node_key), 
	FOREIGN KEY(repository_id, node_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE CASCADE
);

CREATE TABLE graph_node_decorators (
	repository_id TEXT NOT NULL, 
	node_key TEXT NOT NULL, 
	ordinal INTEGER NOT NULL, 
	decorator TEXT NOT NULL, 
	PRIMARY KEY (repository_id, node_key, ordinal), 
	FOREIGN KEY(repository_id, node_key) REFERENCES graph_symbols (repository_id, node_key) ON DELETE CASCADE
);

CREATE TABLE graph_edges (
	repository_id TEXT NOT NULL, 
	ordinal INTEGER NOT NULL, 
	source_key TEXT NOT NULL, 
	target_key TEXT NOT NULL, 
	relationship TEXT NOT NULL, 
	confidence TEXT NOT NULL, 
	path TEXT, 
	evidence TEXT, 
	line INTEGER, 
	type_only BOOLEAN NOT NULL, 
	PRIMARY KEY (repository_id, ordinal), 
	FOREIGN KEY(repository_id, source_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE NO ACTION, 
	FOREIGN KEY(repository_id, target_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE NO ACTION, 
	FOREIGN KEY(repository_id) REFERENCES repository_graphs (repository_id) ON DELETE CASCADE
);

CREATE INDEX graph_edges_source ON graph_edges (repository_id, source_key, relationship, ordinal);

CREATE INDEX graph_edges_target ON graph_edges (repository_id, target_key, relationship, ordinal);

CREATE TABLE graph_diagnostics (
	repository_id TEXT NOT NULL, 
	ordinal INTEGER NOT NULL, 
	severity TEXT NOT NULL, 
	path TEXT NOT NULL, 
	language TEXT NOT NULL, 
	message TEXT NOT NULL, 
	PRIMARY KEY (repository_id, ordinal), 
	FOREIGN KEY(repository_id) REFERENCES repository_graphs (repository_id) ON DELETE CASCADE
);

CREATE INDEX graph_diagnostics_path ON graph_diagnostics (repository_id, path);
