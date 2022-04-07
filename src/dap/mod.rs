use futures::{SinkExt, StreamExt};
use std::fmt::Debug;
use std::fs::File;
use std::io::prelude::*;
use tokio;
use tokio::io::{Stdin, Stdout};
use tokio_util::codec::{Encoder, FramedRead, FramedWrite, LinesCodec};

use crate::dap::types::*;

use self::codec::{DebugAdapterCodec, ParseError};
use self::types::{Event, ProtocolMessage, RequestCommand, Response};

mod codec;
mod types;

pub fn run_dap() {
    match block_on(do_run_dap()) {
        Err(_) => std::process::exit(1),
        _ => (),
    };
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = crate::utils::create_basic_runtime();
    rt.block_on(future)
}

async fn do_run_dap() -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader = FramedRead::new(stdin, DebugAdapterCodec::<ProtocolMessage>::default());
    let mut writer = FramedWrite::new(stdout, DebugAdapterCodec::<ProtocolMessage>::default());
    let mut dap_session = DapSession::new(reader, writer);
    // let mut reader = FramedRead::new(stdin, LinesCodec::new());

    match dap_session.start().await {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("error: {}", e);
            Err(format!("error: {}", e))
        }
    }
}

struct DapSession {
    log_file: File,
    response_seq: i64,
    reader: FramedRead<Stdin, DebugAdapterCodec<ProtocolMessage>>,
    writer: FramedWrite<Stdout, DebugAdapterCodec<ProtocolMessage>>,
}

impl DapSession {
    pub fn new(
        reader: FramedRead<Stdin, DebugAdapterCodec<ProtocolMessage>>,
        writer: FramedWrite<Stdout, DebugAdapterCodec<ProtocolMessage>>,
    ) -> Self {
        Self {
            log_file: File::create("/Users/brice/work/debugger-demo/dap.txt").unwrap(),
            response_seq: 0,
            reader,
            writer,
        }
    }

    pub async fn start(&mut self) -> Result<(), ParseError> {
        writeln!(self.log_file, "STARTING");

        while let Some(msg) = self.reader.next().await {
            writeln!(self.log_file, "LOOPING");
            match msg {
                Ok(msg) => {
                    println!("got message: {:?}", msg);
                    writeln!(self.log_file, "message: {:?}", msg);

                    use crate::dap::types::MessageKind::*;
                    match msg.message {
                        Request(command) => self.handle_request(msg.seq, command).await,
                        Response(response) => self.handle_response(msg.seq, response).await,
                        Event(event) => self.handle_event(msg.seq, event).await,
                    }
                }
                Err(e) => {
                    println!("got error: {}", e);
                    writeln!(self.log_file, "error: {}", e);
                    return Err(e);
                }
            }
        }
        writeln!(self.log_file, "clean exit.");
        Ok(())
    }

    async fn send_response(&mut self, response: Response) {
        let response_json = serde_json::to_string(&response).unwrap();
        writeln!(self.log_file, "response: {}", response_json);
        println!("response: {}", response_json);

        let message = ProtocolMessage {
            seq: self.response_seq,
            message: MessageKind::Response(response),
        };

        match self.writer.send(message).await {
            Ok(_) => (),
            Err(e) => {
                writeln!(self.log_file, "ERROR: sending response: {}", e);
            }
        };

        self.response_seq += 1;
    }

    pub async fn handle_request(&mut self, seq: i64, command: RequestCommand) {
        use crate::dap::types::RequestCommand::*;
        match command {
            Initialize(arguments) => {
                let capabilities = Capabilities {
                    supports_function_breakpoints: Some(true),
                    supports_step_in_targets_request: Some(true),
                    support_terminate_debuggee: Some(true),
                    supports_loaded_sources_request: Some(true),
                    supports_data_breakpoints: Some(true),
                    supports_breakpoint_locations_request: Some(true),
                    supports_configuration_done_request: None,
                    supports_conditional_breakpoints: None,
                    supports_hit_conditional_breakpoints: None,
                    supports_evaluate_for_hovers: None,
                    exception_breakpoint_filters: None,
                    supports_step_back: None,
                    supports_set_variable: None,
                    supports_restart_frame: None,
                    supports_goto_targets_request: None,
                    supports_completions_request: None,
                    completion_trigger_characters: None,
                    supports_modules_request: None,
                    additional_module_columns: None,
                    supported_checksum_algorithms: None,
                    supports_restart_request: None,
                    supports_exception_options: None,
                    supports_value_formatting_options: None,
                    supports_exception_info_request: None,
                    support_suspend_debuggee: None,
                    supports_delayed_stack_trace_loading: None,
                    supports_log_points: None,
                    supports_terminate_threads_request: None,
                    supports_set_expression: None,
                    supports_terminate_request: None,
                    supports_read_memory_request: None,
                    supports_write_memory_request: None,
                    supports_disassemble_request: None,
                    supports_cancel_request: None,
                    supports_clipboard_context: None,
                    supports_stepping_granularity: None,
                    supports_instruction_breakpoints: None,
                    supports_exception_filter_options: None,
                    supports_single_thread_execution_requests: None,
                };
                let response = Response {
                    request_seq: seq,
                    success: true,
                    message: None,
                    body: Some(ResponseBody::Initialize(InitializeResponse {
                        capabilities,
                    })),
                };

                self.send_response(response).await;
            },
            Launch(_arguments) => {
                self.send_response(Response {
                    request_seq: seq,
                    success: true,
                    message: None,
                    body: None,
                }).await;
            }
        }
    }

    pub async fn handle_event(&mut self, seq: i64, event: Event) {
        let response = Response {
            request_seq: seq,
            success: true,
            message: None,
            body: None,
        };
        self.send_response(response).await;
    }

    pub async fn handle_response(&mut self, seq: i64, response: Response) {
        let response = Response {
            request_seq: seq,
            success: true,
            message: None,
            body: None,
        };
        self.send_response(response).await;
    }
}
