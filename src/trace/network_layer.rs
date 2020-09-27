use std::fmt;

use tokio::sync::mpsc::Sender;
use tracing::{field::{Visit, Field}, Subscriber};
use tracing_core::{Event, Level};
use tracing_subscriber::{registry::LookupSpan, layer::{Context, Layer}};

use crate::{trace::TraceEvent, models::NewTraceEvent};

pub struct NetworkLoggingLayer {
    tx: Sender<TraceEvent>
}

impl NetworkLoggingLayer {
    pub fn new(tx: Sender<TraceEvent>) -> Self {
        Self { tx }
    }
}

impl Visit for NewTraceEvent {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}

impl<S> Layer<S> for NetworkLoggingLayer
where
     S: Subscriber + for<'span> LookupSpan<'span>
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let level = match *metadata.level() {
            Level::ERROR => 2,
            Level::WARN => 3,
            Level::INFO => 4,
            Level::DEBUG => 5,
            Level::TRACE => 6,
        };

        if level < 4 {
            // Only record warnings and errors
            let mut new_trace_event = NewTraceEvent {
                message: String::new(),
                target: metadata.target().to_string(),
                level,
                worker_id: None
            };
            event.record(&mut new_trace_event);

            if let Err(e) = self.tx.clone().try_send(TraceEvent::NewEvent(new_trace_event)) {
                // Don't use logging here, it will cause recursion
                println!("Failed to send event to master: {}", e);
            }
        }
    }
}
