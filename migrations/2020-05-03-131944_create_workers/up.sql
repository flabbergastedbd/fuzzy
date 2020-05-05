-- Your SQL goes here
CREATE TABLE workers (
	id VARCHAR(50) PRIMARY KEY,
	name VARCHAR(100),
	cpus INT NOT NULL DEFAULT 0,
	active BOOLEAN NOT NULL DEFAULT TRUE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	updated_at TIMESTAMP
);
SELECT diesel_manage_updated_at('workers');
