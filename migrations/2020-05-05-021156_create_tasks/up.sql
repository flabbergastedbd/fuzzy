-- Your SQL goes here
CREATE TABLE executors (
	id SERIAL PRIMARY KEY,
	name VARCHAR(50)
);

CREATE TABLE tasks (
	id SERIAL PRIMARY KEY,
	name VARCHAR(100) NOT NULL,
	active BOOLEAN NOT NULL DEFAULT FALSE,
	executor_id SERIAL REFERENCES executors(id),
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
	updated_at TIMESTAMP
);

CREATE TABLE worker_tasks (
	id SERIAL PRIMARY KEY,
	worker_id SERIAL REFERENCES workers(id),
	task_id SERIAL REFERENCES tasks(id),
	created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('tasks');
