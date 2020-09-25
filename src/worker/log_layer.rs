use std::fmt;

use tokio::sync::mpsc::Sender;
use tracing::{field::{Visit, Field}, error, Subscriber};
use tracing_core::{Event, Level};
use tracing_subscriber::layer::{Context, Layer};

use crate::TraceEvent;
use crate::models::NewTraceEvent;

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

impl<S:Subscriber> Layer<S> for NetworkLoggingLayer {
    /*
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        match *metadata.level() {
            Level::ERROR => true,
            Level::WARN  => true,
            _            => false,
        }
    }
    */

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let level = match *metadata.level() {
            Level::ERROR => 2,
            Level::WARN => 3,
            Level::INFO => 4,
            Level::DEBUG => 5,
            Level::TRACE => 6,
        };

        // Only record warnings and errors
            let mut new_trace_event = NewTraceEvent {
                message: String::new(),
                target: metadata.target().to_string(),
                level,
                worker_id: 0,
            };
            event.record(&mut new_trace_event);
            println!("{:?}", new_trace_event);

        if level < 4 {
            if let Err(e) = self.tx.clone().try_send(TraceEvent::NewEvent(new_trace_event)) {
                error!("Failed to send event to master: {}", e);
            }
        }
    }
}
