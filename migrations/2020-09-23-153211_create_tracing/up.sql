-- Your SQL goes here
CREATE TABLE trace_events (
	id SERIAL PRIMARY KEY,
	worker_id INT REFERENCES workers(id) ON DELETE CASCADE,
	level INT NOT NULL,
	target VARCHAR NOT NULL,
	message VARCHAR NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);
