table! {
    workers (id) {
        id -> Uuid,
        name -> Nullable<Varchar>,
        cpus -> Int4,
        active -> Bool,
        created -> Nullable<Timestamp>,
        updated -> Nullable<Timestamp>,
    }
}
