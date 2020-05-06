table! {
    tasks (id) {
        id -> Int4,
        name -> Varchar,
        active -> Bool,
        executor -> Nullable<Varchar>,
        fuzz_driver -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    worker_tasks (id) {
        id -> Int4,
        worker_id -> Int4,
        task_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    workers (id) {
        id -> Int4,
        uuid -> Varchar,
        name -> Nullable<Varchar>,
        cpus -> Int4,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(worker_tasks -> tasks (task_id));
joinable!(worker_tasks -> workers (worker_id));

allow_tables_to_appear_in_same_query!(
    tasks,
    worker_tasks,
    workers,
);
