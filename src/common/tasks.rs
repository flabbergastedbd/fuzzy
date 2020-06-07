use std::error::Error;
use std::io::{self, ErrorKind};

use tonic::{transport::channel::Channel, Request};

use crate::models::Task;
use crate::xpc::{orchestrator_client::OrchestratorClient, FilterTask};

pub async fn get_task(id: i32, client: &mut OrchestratorClient<Channel>) -> Result<Task, Box<dyn Error>> {
    let mut tasks = get_tasks(Some(id), None, client).await?;
    if let Some(task) = tasks.pop() {
        Ok(task)
    } else {
        Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!("No task found with id: {}", id),
        )))
    }
}

pub async fn get_tasks(
    id: Option<i32>,
    active: Option<bool>,
    client: &mut OrchestratorClient<Channel>,
) -> Result<Vec<Task>, Box<dyn Error>> {
    let filter_task = FilterTask { id, active };

    let response = client.get_tasks(Request::new(filter_task)).await?;
    let tasks = response.into_inner().data;
    Ok(tasks)
}
