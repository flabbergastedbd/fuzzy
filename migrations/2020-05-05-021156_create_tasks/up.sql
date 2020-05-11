-- Your SQL goes here
CREATE TYPE executor_t AS ENUM ('NATIVE', 'DOCKER');
CREATE TYPE fuzz_driver_t AS ENUM ('AFLPP', 'FUZZILLI');

CREATE TABLE tasks (
	id SERIAL PRIMARY KEY,
	name VARCHAR(100) NOT NULL,
	active BOOLEAN NOT NULL DEFAULT FALSE,
	profile VARCHAR NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE worker_tasks (
	id SERIAL PRIMARY KEY,
	task_id SERIAL REFERENCES tasks(id) ON DELETE CASCADE,
	worker_id SERIAL REFERENCES workers(id) ON DELETE CASCADE,
	cpus INT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE corpora (
	id SERIAL PRIMARY KEY,
	content bytea NOT NULL,
	checksum VARCHAR(64) UNIQUE NOT NULL,
	label VARCHAR(100) NOT NULL,
	worker_task_id INTEGER REFERENCES worker_tasks(id) ON DELETE SET NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE crashes (
	id SERIAL PRIMARY KEY,
	content bytea NOT NULL,
	checksum VARCHAR(64),
	label VARCHAR(100) NOT NULL,
	verified BOOLEAN NOT NULL DEFAULT FALSE,
	worker_task_id INTEGER REFERENCES worker_tasks(id) ON DELETE SET NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	UNIQUE(checksum, label, worker_task_id)
);

CREATE TABLE fuzz_stats (
	id SERIAL PRIMARY KEY,
	coverage INTEGER NOT NULL,
	execs INTEGER NOT NULL,
	memory INTEGER,
	worker_task_id INTEGER REFERENCES worker_tasks(id) ON DELETE CASCADE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE sys_stats (
	id SERIAL PRIMARY KEY,
	cpu INTEGER NOT NULL,
	memory INTEGER NOT NULL,
	worker INTEGER REFERENCES workers(id) ON DELETE CASCADE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('tasks');
