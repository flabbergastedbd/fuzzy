fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // The best magic to tie Protobuf Structs -> Diesel ORM Rust Structs
        // NewWorker (Worker with limited fields)
        .type_attribute("NewWorker", "#[derive(Queryable, Insertable, AsChangeset, Associations)]")
        .type_attribute("NewWorker", "#[table_name = \"workers\"]")

        // Worker
        .type_attribute("Worker", "#[derive(Queryable, Insertable, Identifiable, AsChangeset, Associations)]")
        .type_attribute("Worker", "#[table_name = \"workers\"]")

        // NewTask (Task with limited fields)
        .type_attribute("NewTask", "#[derive(Queryable, Insertable, AsChangeset, Associations)]")
        .type_attribute("NewTask", "#[table_name = \"tasks\"]")

        // Task
        .type_attribute("Task", "#[derive(Queryable, Insertable, Identifiable, AsChangeset, Associations)]")
        .type_attribute("Task", "#[table_name = \"tasks\"]")


        .compile(
            &["proto/xpc.proto"],
            &["proto"]
        )?;
    Ok(())
}
