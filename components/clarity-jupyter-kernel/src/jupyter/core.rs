use clarity_repl::repl::{Session, SessionSettings};

use super::connection::Connection;
use super::control_file;
use super::jupyter_message::JupyterMessage;
use super::CommandContext;

use colored::Colorize;
use failure::Error;
use json;
use json::JsonValue;
use std;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time;
use zmq;

#[derive(Clone)]
pub struct Server {
    iopub: Arc<Mutex<Connection>>,
    _stdin: Arc<Mutex<Connection>>,
    latest_execution_request: Arc<Mutex<Option<JupyterMessage>>>,
    shutdown_requested_receiver: Arc<Mutex<mpsc::Receiver<()>>>,
    shutdown_requested_sender: Arc<Mutex<mpsc::Sender<()>>>,
    session: Session,
}

impl Server {
    pub fn start(config: &control_file::Control) -> Result<Server, Error> {
        use zmq::SocketType;

        let zmq_context = zmq::Context::new();
        let heartbeat = bind_socket(config, config.hb_port, zmq_context.socket(SocketType::REP)?)?;
        let shell_socket = bind_socket(
            config,
            config.shell_port,
            zmq_context.socket(SocketType::ROUTER)?,
        )?;
        let control_socket = bind_socket(
            config,
            config.control_port,
            zmq_context.socket(SocketType::ROUTER)?,
        )?;
        let stdin_socket = bind_socket(
            config,
            config.stdin_port,
            zmq_context.socket(SocketType::ROUTER)?,
        )?;
        let iopub = Arc::new(Mutex::new(bind_socket(
            config,
            config.iopub_port,
            zmq_context.socket(SocketType::PUB)?,
        )?));

        let session = Session::new(SessionSettings::default());

        let (shutdown_requested_sender, shutdown_requested_receiver) = mpsc::channel();

        let server = Server {
            session,
            iopub,
            latest_execution_request: Arc::new(Mutex::new(None)),
            _stdin: Arc::new(Mutex::new(stdin_socket)),
            shutdown_requested_receiver: Arc::new(Mutex::new(shutdown_requested_receiver)),
            shutdown_requested_sender: Arc::new(Mutex::new(shutdown_requested_sender)),
        };

        let (execution_sender, execution_receiver) = mpsc::channel();
        let (execution_response_sender, execution_response_receiver) = mpsc::channel();

        thread::spawn(move || Self::handle_hb(&heartbeat));
        server.start_thread(move |server: Server| server.handle_control(control_socket));
        server.start_thread(move |server: Server| {
            server.handle_shell(
                shell_socket,
                &execution_sender,
                &execution_response_receiver,
            )
        });
        let (mut context, outputs) = CommandContext::new()?;
        // context.execute(":load_config")?;
        server.start_thread(move |mut server: Server| {
            server.handle_execution_requests(
                context,
                &execution_receiver,
                &execution_response_sender,
            )
        });
        server
            .clone()
            .start_output_pass_through_thread("stdout", outputs.stdout);
        server
            .clone()
            .start_output_pass_through_thread("stderr", outputs.stderr);
        Ok(server)
    }

    pub fn wait_for_shutdown(&self) {
        self.shutdown_requested_receiver
            .lock()
            .unwrap()
            .recv()
            .unwrap();
    }

    fn signal_shutdown(&self) {
        self.shutdown_requested_sender
            .lock()
            .unwrap()
            .send(())
            .unwrap();
    }

    fn start_thread<F>(&self, body: F)
    where
        F: FnOnce(Server) -> Result<(), Error> + std::marker::Send + 'static,
    {
        let server_clone = self.clone();
        thread::spawn(|| {
            if let Err(error) = body(server_clone) {
                eprintln!("{:?}", error);
            }
        });
    }

    fn handle_hb(connection: &Connection) -> Result<(), Error> {
        let mut message = zmq::Message::new();
        let ping: &[u8] = b"ping";
        loop {
            connection.socket.recv(&mut message, 0)?;
            connection.socket.send(ping, 0)?;
        }
    }

    fn handle_execution_requests(
        &mut self,
        mut context: CommandContext,
        receiver: &mpsc::Receiver<JupyterMessage>,
        execution_reply_sender: &mpsc::Sender<JupyterMessage>,
    ) -> Result<(), Error> {
        let mut execution_count = 1;
        loop {
            let message = receiver.recv()?;

            *self.latest_execution_request.lock().unwrap() = Some(message.clone());
            let src = message.code();
            execution_count += 1;
            message
                .new_message("execute_input")
                .with_content(object! {
                    "execution_count" => execution_count,
                    "code" => src
                })
                .send(&mut *self.iopub.lock().unwrap())?;
            let mut has_error = false;
            for code in split_code_and_command(src) {
                match self
                    .session
                    .formatted_interpretation(code.to_string(), None, false, None)
                {
                    Ok((result, _)) => {
                        let res = result.join("\n");
                        let mut data: HashMap<String, JsonValue> = HashMap::new();
                        data.insert("text/plain".into(), json::from(res));
                        message
                            .new_message("execute_result")
                            .with_content(object! {
                                "execution_count" => execution_count,
                                "data" => data,
                                "metadata" => HashMap::new(),
                            })
                            .send(&mut *self.iopub.lock().unwrap())?;
                    }
                    Err(result) => {
                        let res = result.join("\n");
                        has_error = false;
                        message
                            .new_message("error")
                            .with_content(object! {
                                "ename" => "Error",
                                "evalue" => res.clone(),
                                "traceback" => array![res],
                            })
                            .send(&mut *self.iopub.lock().unwrap())?;
                        break;
                    }
                }
            }
            let reply = if has_error {
                message.new_reply().with_content(object! {
                    "status" => "error",
                    "execution_count" => execution_count,
                })
            } else {
                message.new_reply().with_content(object! {
                    "status" => "ok",
                    "execution_count" => execution_count
                })
            };
            execution_reply_sender.send(reply)?;
        }
    }

    fn handle_shell(
        self,
        mut connection: Connection,
        execution_channel: &mpsc::Sender<JupyterMessage>,
        execution_reply_receiver: &mpsc::Receiver<JupyterMessage>,
    ) -> Result<(), Error> {
        loop {
            let message = JupyterMessage::read(&mut connection)?;
            message
                .new_message("status")
                .with_content(object! {"execution_state" => "busy"})
                .send(&mut *self.iopub.lock().unwrap())?;
            let idle = message
                .new_message("status")
                .with_content(object! {"execution_state" => "idle"});
            if message.message_type() == "kernel_info_request" {
                message
                    .new_reply()
                    .with_content(kernel_info())
                    .send(&mut connection)?;
            } else if message.message_type() == "is_complete_request" {
                message
                    .new_reply()
                    .with_content(object! {"status" => "complete"})
                    .send(&mut connection)?;
            } else if message.message_type() == "execute_request" {
                execution_channel.send(message)?;
                execution_reply_receiver.recv()?.send(&mut connection)?;
            } else if message.message_type() == "comm_open" {
                message
                    .new_message("comm_close")
                    .with_content(message.get_content().clone())
                    .send(&mut connection)?;
            } else {
                eprintln!(
                    "Got unrecognized message type on shell channel: {}",
                    message.message_type()
                );
            }
            idle.send(&mut *self.iopub.lock().unwrap())?;
        }
    }

    fn handle_control(self, mut connection: Connection) -> Result<(), Error> {
        loop {
            let message = JupyterMessage::read(&mut connection)?;
            match message.message_type() {
                "shutdown_request" => self.signal_shutdown(),
                "interrupt_request" => {
                    message.new_reply().send(&mut connection)?;
                    eprintln!(
                        "Rust doesn't support interrupting execution. Perhaps restart kernel?"
                    );
                }
                _ => {
                    eprintln!(
                        "Got unrecognized message type on control channel: {}",
                        message.message_type()
                    );
                }
            }
        }
    }

    fn start_output_pass_through_thread(
        self,
        output_name: &'static str,
        channel: mpsc::Receiver<String>,
    ) {
        thread::spawn(move || {
            while let Ok(line) = channel.recv() {
                let mut message = None;
                if let Some(exec_request) = &*self.latest_execution_request.lock().unwrap() {
                    message = Some(exec_request.new_message("stream"));
                }
                if let Some(message) = message {
                    if let Err(error) = message
                        .with_content(object! {
                            "name" => output_name,
                            "text" => format!("{}\n", line),
                        })
                        .send(&mut *self.iopub.lock().unwrap())
                    {
                        eprintln!("{}", error);
                    }
                }
            }
        });
    }

    fn emit_errors(&self, errors: &Error, parent_message: &JupyterMessage) -> Result<(), Error> {
        Ok(())
    }
}

fn bind_socket(
    config: &control_file::Control,
    port: u16,
    socket: zmq::Socket,
) -> Result<Connection, Error> {
    let endpoint = format!("{}://{}:{}", config.transport, config.ip, port);
    socket.bind(&endpoint)?;
    Ok(Connection::new(socket, &config.key)?)
}

/// See [Kernel info documentation](https://jupyter-client.readthedocs.io/en/stable/messaging.html#kernel-info)
fn kernel_info() -> JsonValue {
    object! {
        "protocol_version" => "5.3",
        "implementation" => env!("CARGO_PKG_NAME"),
        "implementation_version" => env!("CARGO_PKG_VERSION"),
        "language_info" => object!{
            "name" => "Clarity",
            "version" => "",
            "mimetype" => "text/clarity",
            "file_extension" => ".clar",
            // Pygments lexer, for highlighting Only needed if it differs from the 'name' field.
            // see http://pygments.org/docs/lexers/#lexers-for-the-rust-language
            "pygment_lexer" => "clarity",
            // Codemirror mode, for for highlighting in the notebook. Only needed if it differs from the 'name' field.
            // codemirror use text/x-rustsrc as mimetypes
            // see https://codemirror.net/mode/rust/
            "codemirror_mode" => "clarity",
        },
        "banner" => format!("Evaluation Context for Clarity"),
        "help_links" => array![
            object!{"text" => "Clarity std docs",
                    "url" => "https://doc.rust-lang.org/stable/std/"}
        ],
        "status" => "ok"
    }
}

//TODO optimize by avoiding creation of new String
fn split_code_and_command(src: &str) -> Vec<String> {
    src.lines().fold(vec![], |mut acc, l| {
        if l.starts_with(':') {
            acc.push(l.to_owned());
        } else if let Some(last) = acc.pop() {
            if !last.starts_with(':') {
                acc.push(last + "\n" + l);
            } else {
                acc.push(last);
                acc.push(l.to_owned());
            }
        } else {
            acc.push(l.to_owned());
        }
        acc
    })
}
