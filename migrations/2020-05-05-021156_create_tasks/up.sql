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
	active BOOLEAN NOT NULL DEFAULT FALSE,
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
	output VARCHAR,
	worker_task_id INTEGER REFERENCES worker_tasks(id) ON DELETE SET NULL,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	UNIQUE(checksum, label, worker_task_id)
);

CREATE TABLE fuzz_stats (
	id SERIAL PRIMARY KEY,
	coverage INTEGER NOT NULL,
	execs INTEGER,
	memory INTEGER,
	worker_task_id INTEGER REFERENCES worker_tasks(id) ON DELETE CASCADE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE TABLE sys_stats (
	id SERIAL PRIMARY KEY,
	cpu_user_time FLOAT8 NOT NULL,
	cpu_system_time FLOAT8 NOT NULL,
	cpu_idle_time FLOAT8 NOT NULL,
	memory_total INTEGER NOT NULL,
	memory_used INTEGER NOT NULL,
	swap_total INTEGER NOT NULL,
	swap_used INTEGER NOT NULL,
	worker_id INTEGER REFERENCES workers(id) ON DELETE CASCADE,
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('tasks');
