use crate::models::Worker;

#[tarpc::service]
pub trait Collector {
    async fn heartbeat(worker: Worker) -> bool;
}
