CREATE TABLE repositories (
	id TEXT NOT NULL, 
	name TEXT NOT NULL, 
	location TEXT NOT NULL, 
	created_at INTEGER NOT NULL, 
	PRIMARY KEY (id)
);

CREATE UNIQUE INDEX repositories_name_unique ON repositories (lower(name));

CREATE TABLE repository_graphs (
	repository_id TEXT NOT NULL, 
	artifact TEXT NOT NULL, 
	imported_at INTEGER NOT NULL, 
	PRIMARY KEY (repository_id), 
	FOREIGN KEY(repository_id) REFERENCES repositories (id) ON DELETE CASCADE
);
