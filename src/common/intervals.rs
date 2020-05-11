use std::time::Duration;

// Worker related
pub const WORKER_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
pub const WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL: Duration = Duration::from_secs(60);
pub const WORKER_TASK_REFRESH_INTERVAL: Duration = Duration::from_secs(300);

// Master related
pub const MASTER_SCHEDULER_INTERVAL: Duration = Duration::from_secs(300);
