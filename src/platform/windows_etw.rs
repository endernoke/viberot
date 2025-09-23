use ferrisetw::EventRecord;
use ferrisetw::schema_locator::SchemaLocator;
use ferrisetw::parser::Parser;
use ferrisetw::provider::Provider;
use ferrisetw::provider::EventFilter;
use ferrisetw::trace;
use ferrisetw::trace::UserTrace;
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tasklist::get_proc_params;

use crate::platform::{PlatformProbeTrait, ProcessLifecycleEvent, ProcessEvent};

/// Windows ETW-based process probe
/// Uses Event Tracing for Windows to monitor process creation and termination events
pub struct WindowsEtwProbe {
    lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>,
    trace_handle: Arc<Mutex<Option<UserTrace>>>,
}

impl WindowsEtwProbe {
    pub fn new(lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>) -> Self {
        Self {
            lifecycle_sender,
            trace_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Callback function to process ETW events
    fn process_event_callback(
        record: &EventRecord,
        schema_locator: &SchemaLocator,
        sender: broadcast::Sender<ProcessLifecycleEvent>,
    ) {
        let event_id = record.event_id();
        match event_id {
            1 => Self::handle_process_start(record, schema_locator, &sender),
            2 => Self::handle_process_exit(record, schema_locator, &sender),
            _ => { /* Ignore other events */ },
        }
    }

    fn handle_process_start(
        record: &EventRecord,
        schema_locator: &SchemaLocator,
        sender: &broadcast::Sender<ProcessLifecycleEvent>,
    ) {
        match schema_locator.event_schema(record) {
            Ok(schema) => {
                let parser = Parser::create(record, &schema);
                
                let pid_result = parser.try_parse::<u32>("ProcessID");
                
                match pid_result {
                    Ok(pid) => {
                        debug!("Successfully parsed process start: PID={}", pid);
                        // Attempt to get command line via tasklist as fallback
                        let command_line = get_proc_params(pid).unwrap_or_else(|e| {
                            debug!("Failed to get command line via tasklist for PID {}: {:?}", pid, e);
                            String::from("<unknown>")
                        }).replace('\0', ""); // Remove null chars
                        let process_event = ProcessEvent::new(pid, command_line);
                        let lifecycle_event = ProcessLifecycleEvent::Started(process_event);
                        
                        if let Err(e) = sender.send(lifecycle_event) {
                            error!("Failed to send process start event: {}", e);
                        }
                    }
                    Err(pid_result) => {
                        debug!("Failed to parse process start event fields - PID: {:?}", pid_result);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to get schema for process start event: {:?}", e);
            }
        }
    }

    fn handle_process_exit(
        record: &EventRecord,
        schema_locator: &SchemaLocator,
        sender: &broadcast::Sender<ProcessLifecycleEvent>,
    ) {
        match schema_locator.event_schema(record) {
            Ok(schema) => {
                let parser = Parser::create(record, &schema);
                
                // debug!("Process exit event - Provider: {}", schema.provider_name());
                
                let pid_result = parser.try_parse::<u32>("ProcessID");
                let exit_code_result = parser.try_parse::<u32>("ExitCode");
                
                match pid_result {
                    Ok(pid) => {
                        let _exit_code = exit_code_result.ok();
                        // debug!("Successfully parsed process exit: PID={}, ExitCode={:?}", pid, exit_code);
                        let lifecycle_event = ProcessLifecycleEvent::Ended { pid };
                        
                        if let Err(e) = sender.send(lifecycle_event) {
                            debug!("Failed to send process exit event: {}", e);
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse process exit event PID: {:?}", e);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to get schema for process exit event: {:?}", e);
            }
        }
    }
}

impl PlatformProbeTrait for WindowsEtwProbe {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Windows ETW probe for process monitoring");

        let sender = self.lifecycle_sender.clone();
        let trace_handle = Arc::clone(&self.trace_handle);

        // Spawn the ETW trace in a separate thread since it's blocking
        let _join_handle = thread::spawn(move || {
            // Create callback closure that captures the sender
            let callback = move |record: &EventRecord, schema_locator: &SchemaLocator| {
                Self::process_event_callback(record, schema_locator, sender.clone());
            };
            let filter = EventFilter::ByEventIds(vec![1, 2]);

            // Microsoft-Windows-Kernel-Process provider GUID
            // This provider emits process creation and termination events
            let process_provider = Provider::by_guid("22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716")
                .add_callback(callback)
                .add_filter(filter)
                .build();
            
            // Stop any existing trace session with the same name to avoid conflicts.
            // This is useful if the application crashed previously and didn't clean up.
            let _ = trace::stop_trace_by_name("CommandSidekickTrace");

            // Start the trace session
            let trace = match UserTrace::new()
                .named(String::from("CommandSidekickTrace"))
                .enable(process_provider)
                .start_and_process()
            {
                Ok(trace) => {
                    info!("ETW trace session started successfully");
                    trace
                },
                Err(e) => {
                    error!("Failed to start ETW trace session: {:?}", e);
                    
                    // Check if it's an access denied error
                    let error_msg = format!("{:?}", e);
                    if error_msg.contains("Access is denied") || error_msg.contains("-2147024891") {
                        error!("ETW requires administrator privileges. Please run as administrator for optimal performance.");
                        error!("Alternatively, consider using the WMI fallback implementation.");
                    }
                    return;
                }
            };

            // Store the trace handle for later cleanup
            {
                let mut handle = trace_handle.lock().unwrap();
                *handle = Some(trace);
            }

            // The trace will run until stopped
            // Note: start_and_process() handles the trace processing in a separate thread
            // so this thread will continue and eventually exit, but the trace will keep running
        });

        // Give the ETW trace a moment to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Check if ETW initialization was successful
        {
            let handle = self.trace_handle.lock().unwrap();
            if handle.is_none() {
                return Err("ETW trace session failed to start. Run as administrator or use WMI fallback.".into());
            }
        }
        
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping Windows ETW probe");

        let mut handle = self.trace_handle.lock().unwrap();
        if let Some(trace) = handle.take() {
            let _ = trace.stop();  // Ignore result as suggested by compiler
            info!("ETW trace session stopped");
        }

        Ok(())
    }
}