table! {
    corpora (id) {
        id -> Int4,
        content -> Bytea,
        checksum -> Varchar,
        label -> Varchar,
        worker_task_id -> Nullable<Int4>,
        created_at -> Timestamp,
    }
}

table! {
    crashes (id) {
        id -> Int4,
        content -> Bytea,
        checksum -> Nullable<Varchar>,
        label -> Varchar,
        verified -> Bool,
        output -> Nullable<Varchar>,
        worker_task_id -> Nullable<Int4>,
        created_at -> Timestamp,
    }
}

table! {
    fuzz_stats (id) {
        id -> Int4,
        branch_coverage -> Nullable<Int4>,
        line_coverage -> Nullable<Int4>,
        function_coverage -> Nullable<Int4>,
        execs -> Nullable<Int4>,
        memory -> Nullable<Int4>,
        worker_task_id -> Nullable<Int4>,
        created_at -> Timestamp,
    }
}

table! {
    sys_stats (id) {
        id -> Int4,
        cpu_user_time -> Float8,
        cpu_system_time -> Float8,
        cpu_idle_time -> Float8,
        memory_total -> Int4,
        memory_used -> Int4,
        swap_total -> Int4,
        swap_used -> Int4,
        worker_id -> Nullable<Int4>,
        created_at -> Timestamp,
    }
}

table! {
    tasks (id) {
        id -> Int4,
        name -> Varchar,
        active -> Bool,
        profile -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    worker_tasks (id) {
        id -> Int4,
        task_id -> Int4,
        worker_id -> Int4,
        cpus -> Int4,
        active -> Bool,
        created_at -> Timestamp,
    }
}

table! {
    workers (id) {
        id -> Int4,
        uuid -> Varchar,
        name -> Nullable<Varchar>,
        cpus -> Int4,
        memory -> Int4,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(corpora -> worker_tasks (worker_task_id));
joinable!(crashes -> worker_tasks (worker_task_id));
joinable!(fuzz_stats -> worker_tasks (worker_task_id));
joinable!(sys_stats -> workers (worker_id));
joinable!(worker_tasks -> tasks (task_id));
joinable!(worker_tasks -> workers (worker_id));

allow_tables_to_appear_in_same_query!(
    corpora,
    crashes,
    fuzz_stats,
    sys_stats,
    tasks,
    worker_tasks,
    workers,
);
