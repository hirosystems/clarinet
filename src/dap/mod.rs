use self::codec::{DebugAdapterCodec, ParseError};
use crate::poke::load_session;
use crate::types::Network;
use clarity_repl::repl::Session;
use dap_types::events::*;
use dap_types::requests::*;
use dap_types::responses::*;
use dap_types::types::*;
use dap_types::*;
use futures::{SinkExt, StreamExt};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use tokio;
use tokio::io::{Stdin, Stdout};
use tokio_util::codec::{FramedRead, FramedWrite};

mod codec;

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
    send_seq: i64,
    reader: FramedRead<Stdin, DebugAdapterCodec<ProtocolMessage>>,
    writer: FramedWrite<Stdout, DebugAdapterCodec<ProtocolMessage>>,
    session: Option<Session>,
}

impl DapSession {
    pub fn new(
        reader: FramedRead<Stdin, DebugAdapterCodec<ProtocolMessage>>,
        writer: FramedWrite<Stdout, DebugAdapterCodec<ProtocolMessage>>,
    ) -> Self {
        Self {
            log_file: File::create("/Users/brice/work/debugger-demo/dap.txt").unwrap(),
            send_seq: 0,
            reader,
            writer,
            session: None,
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

                    use dap_types::MessageKind::*;
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
        writeln!(self.log_file, "::::response: {}", response_json);
        println!("::::response: {}", response_json);

        let message = ProtocolMessage {
            seq: self.send_seq,
            message: MessageKind::Response(response),
        };

        match self.writer.send(message).await {
            Ok(_) => (),
            Err(e) => {
                writeln!(self.log_file, "ERROR: sending response: {}", e);
            }
        };

        self.send_seq += 1;
    }

    async fn send_event(&mut self, body: EventBody) {
        let event_json = serde_json::to_string(&body).unwrap();
        writeln!(self.log_file, "::::event: {}", event_json);
        println!("::::event: {}", event_json);

        let message = ProtocolMessage {
            seq: self.send_seq,
            message: MessageKind::Event(Event { body: Some(body) }),
        };

        match self.writer.send(message).await {
            Ok(_) => (),
            Err(e) => {
                writeln!(self.log_file, "ERROR: sending response: {}", e);
            }
        };

        self.send_seq += 1;
    }

    pub async fn handle_request(&mut self, seq: i64, command: RequestCommand) {
        use dap_types::requests::RequestCommand::*;
        let result = match command {
            Initialize(arguments) => self.initialize(arguments),
            Launch(arguments) => self.launch(arguments).await,
            // SetBreakpoints(arguments) => self.setBreakpoints(arguments),
            _ => Err("unsupported request".to_string()),
        };

        let response = match result {
            Ok(body) => Response {
                request_seq: seq,
                success: true,
                message: None,
                body,
            },
            Err(err_msg) => Response {
                request_seq: seq,
                success: false,
                message: Some(err_msg),
                body: None,
            },
        };

        println!("SENDING RESPONSE");
        self.send_response(response).await;
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

    // Request handlers

    fn initialize(
        &mut self,
        arguments: InitializeRequestArguments,
    ) -> Result<Option<ResponseBody>, String> {
        println!("INITIALIZE");
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
        Ok(Some(ResponseBody::Initialize(InitializeResponse {
            capabilities,
        })))
    }

    async fn launch(
        &mut self,
        arguments: LaunchRequestArguments,
    ) -> Result<Option<ResponseBody>, String> {
        println!("LAUNCH");
        // Verify that the manifest and expression were specified
        let manifest = match arguments.manifest {
            Some(manifest) => manifest,
            None => return Err("manifest must be specified".to_string()),
        };
        let expression = match arguments.expression {
            Some(expression) => expression,
            None => return Err("expression to debug must be specified".to_string()),
        };

        // Initiate the session
        let manifest_path = PathBuf::from(manifest);
        let session = match load_session(&manifest_path, false, &Network::Devnet) {
            Ok((session, _, _, _)) => session,
            Err((_, e)) => return Err(e),
        };
        self.session = Some(session);

        // Begin execution of the expression in debug mode
        // if self
        //     .session
        //     .as_mut()
        //     .unwrap()
        //     .interpret(expression, None, false, true, None)
        //     .is_err()
        // {
        //     return Err("unable to start session".to_string());
        // }

        self.send_event(EventBody::Initialized).await;

        Ok(Some(ResponseBody::Launch))
    }

    // fn setBreakpoints(arguments: SetBreakpointsArguments) -> Result<Option<ResponseBody>, String> {
    //     println!("SET BREAKPOINTS");
    // }
}
