table! {
    workers (id) {
        id -> Varchar,
        name -> Nullable<Varchar>,
        cpus -> Int4,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}
