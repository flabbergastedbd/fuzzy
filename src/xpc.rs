use crate::models::Worker;

tonic::include_proto!("xpc"); // The string specified here must match the proto package name

// Easy conversions from grpc messages to native types
impl From<HeartbeatRequest> for Worker {
    fn from(w: HeartbeatRequest) -> Worker {
        Worker {
            uuid: w.uuid,
            name: Some(w.name),
            cpus: w.cpus,
            // Set agent as active always from heartbeat request
            active: true
        }
    }
}
