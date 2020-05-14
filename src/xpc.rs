// Need to be here for generated rust code by prost
use diesel::{Queryable, Identifiable, Insertable, AsChangeset};

// Insert table names
use super::schema::{tasks, workers, corpora, crashes, worker_tasks, fuzz_stats, sys_stats};

tonic::include_proto!("xpc"); // The string specified here must match the proto package name
