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
        task_id -> Int4,
        worker_id -> Int4,
        verified -> Bool,
        created_at -> Timestamp,
    }
}

table! {
    tasks (id) {
        id -> Int4,
        name -> Varchar,
        active -> Bool,
        profile -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    worker_tasks (id) {
        id -> Int4,
        task_id -> Int4,
        worker_id -> Int4,
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

joinable!(corpora -> tasks (worker_task_id));
joinable!(crashes -> tasks (task_id));
joinable!(crashes -> workers (worker_id));
joinable!(worker_tasks -> tasks (task_id));
joinable!(worker_tasks -> workers (worker_id));

allow_tables_to_appear_in_same_query!(
    corpora,
    crashes,
    tasks,
    worker_tasks,
    workers,
);
