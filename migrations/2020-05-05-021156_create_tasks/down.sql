-- This file should undo anything in `up.sql`
DROP TABLE crashes, corpora, worker_tasks, tasks;

-- Drop types
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'executor_t') THEN
	DROP TYPE executor_t;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'fuzz_driver_t') THEN
	DROP TYPE fuzz_driver_t;
    END IF;
    --more types here...
END$$;
