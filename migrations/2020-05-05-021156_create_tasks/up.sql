-- Your SQL goes here
CREATE TYPE executor_t AS ENUM ('NATIVE', 'DOCKER');
CREATE TYPE fuzz_driver_t AS ENUM ('AFLPP', 'FUZZILLI');

CREATE TABLE tasks (
	id SERIAL PRIMARY KEY,
	name VARCHAR(100) NOT NULL,
	active BOOLEAN NOT NULL DEFAULT FALSE,
	executor VARCHAR(20),
	fuzz_driver VARCHAR(20),
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE worker_tasks (
	id SERIAL PRIMARY KEY,
	worker_id SERIAL REFERENCES workers(id) ON DELETE CASCADE,
	task_id SERIAL REFERENCES tasks(id) ON DELETE CASCADE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('tasks');
