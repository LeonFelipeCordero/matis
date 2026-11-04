ALTER TABLE repository_graphs ADD COLUMN graph_revision TEXT;

ALTER TABLE repository_graphs ADD COLUMN schema_version INTEGER;

ALTER TABLE repository_graphs ADD COLUMN indexed_at TEXT;

ALTER TABLE repository_graphs ADD COLUMN root_name TEXT;

ALTER TABLE repository_graphs ADD COLUMN root_path TEXT;

CREATE TABLE graph_nodes (
	repository_id TEXT NOT NULL, 
	node_key TEXT NOT NULL, 
	parent_key TEXT, 
	ordinal INTEGER NOT NULL, 
	type TEXT NOT NULL, 
	kind TEXT NOT NULL, 
	level TEXT NOT NULL, 
	name TEXT NOT NULL, 
	language TEXT, 
	qualified_name TEXT, 
	path TEXT, 
	PRIMARY KEY (repository_id, node_key), 
	UNIQUE (repository_id, ordinal), 
	FOREIGN KEY(repository_id, parent_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED, 
	FOREIGN KEY(repository_id) REFERENCES repository_graphs (repository_id) ON DELETE CASCADE
);

CREATE INDEX graph_nodes_parent ON graph_nodes (repository_id, parent_key);

CREATE INDEX graph_nodes_level ON graph_nodes (repository_id, level);

CREATE INDEX graph_nodes_name ON graph_nodes (repository_id, name COLLATE NOCASE);

CREATE INDEX graph_nodes_path ON graph_nodes (repository_id, path);

CREATE TABLE graph_files (
	repository_id TEXT NOT NULL, 
	node_key TEXT NOT NULL, 
	byte_count INTEGER, 
	sha256 TEXT, 
	PRIMARY KEY (repository_id, node_key), 
	FOREIGN KEY(repository_id, node_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE CASCADE
);

CREATE TABLE graph_symbols (
	repository_id TEXT NOT NULL, 
	node_key TEXT NOT NULL, 
	start_line INTEGER, 
	start_column INTEGER, 
	end_line INTEGER, 
	end_column INTEGER, 
	signature TEXT, 
	visibility TEXT, 
	PRIMARY KEY (repository_id, node_key), 
	FOREIGN KEY(repository_id, node_key) REFERENCES graph_nodes (repository_id, node_key) ON DELETE CASCADE
);
