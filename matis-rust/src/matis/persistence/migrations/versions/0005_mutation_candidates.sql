CREATE TABLE graph_mutation_candidates (
	repository_id TEXT NOT NULL, 
	candidate_id TEXT NOT NULL, 
	file_key TEXT NOT NULL, 
	ordinal INTEGER NOT NULL, 
	path TEXT NOT NULL, 
	language TEXT NOT NULL, 
	source_hash TEXT NOT NULL, 
	kind TEXT NOT NULL, 
	operator TEXT, 
	payload TEXT NOT NULL, 
	PRIMARY KEY (repository_id, candidate_id), 
	FOREIGN KEY(repository_id, file_key) REFERENCES graph_files (repository_id, node_key) ON DELETE CASCADE
);

CREATE INDEX graph_mutation_candidates_file ON graph_mutation_candidates (repository_id, file_key);
