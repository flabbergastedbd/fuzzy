-- Your SQL goes here
CREATE TABLE trace_events (
	id SERIAL PRIMARY KEY,
	worker_id SERIAL REFERENCES workers(id) ON DELETE CASCADE,
	level INT NOT NULL,
	span VARCHAR NOT NULL,
	content VARCHAR NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);
