use serde::{Deserialize, Serialize};
// Need to be here for generated rust code by prost
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

// Insert table names
use super::schema::{corpora, crashes, fuzz_stats, sys_stats, tasks, worker_tasks, workers, trace_events};

tonic::include_proto!("xpc"); // The string specified here must match the proto package name
