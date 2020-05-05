fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // The best magic to tie Protobuf Structs -> Diesel ORM Rust Structs
        .type_attribute("Worker", "#[derive(Queryable, Insertable, AsChangeset, Associations)]")
        .type_attribute("Worker", "#[table_name = \"workers\"]")
        .compile(
            &["proto/xpc.proto"],
            &["proto"]
        )?;
    Ok(())
}
