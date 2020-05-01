use capnpc;

fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/pulse.capnp")
        .run().expect("Schema compiler command");
}
