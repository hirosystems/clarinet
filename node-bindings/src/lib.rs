use clarinet_lib::integrate::{self, DevnetOrchestrator, LogData, NodeObserverEvent};
use clarinet_lib::types::DevnetConfigFile;
use neon::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{env, process};

type DevnetCallback = Box<dyn FnOnce(&Channel) + Send>;

struct StacksDevnet {
    tx: mpsc::Sender<DevnetCommand>,
    devnet_event_rx: mpsc::Receiver<NodeObserverEvent>,
}

enum DevnetCommand {
    Start(Option<DevnetCallback>),
    Stop(Option<DevnetCallback>),
}

impl Finalize for StacksDevnet {}

impl StacksDevnet {
    fn new<'a, C>(cx: &mut C, manifest_path: String) -> Self
    where
        C: Context<'a>,
    {
        // Channel for sending callbacks to execute on the sqlite connection thread
        let (tx, rx) = mpsc::channel::<DevnetCommand>();
        let (devnet_events_tx, devnet_events_rx) = mpsc::channel();
        let (log_tx, log_rx) = mpsc::channel();

        // Create an `Channel` for calling back to JavaScript. It is more efficient
        // to create a single channel and re-use it for all database callbacks.
        // The JavaScript process will not exit as long as this channel has not been
        // dropped.
        let channel = cx.channel();

        thread::spawn(move || {
            let manifest_path = get_manifest_path_or_exit(Some(manifest_path.into()));
            let devnet_overrides = DevnetConfigFile::default();
            let devnet = DevnetOrchestrator::new(manifest_path, Some(devnet_overrides));

            if let Ok(DevnetCommand::Start(callback)) = rx.recv() {
                // Start devnet
                integrate::run_devnet(devnet, Some(devnet_events_tx), Some(log_tx), false);
                if let Some(c) = callback {
                    c(&channel);
                }

                // Start run loop
                while let Ok(message) = rx.recv() {
                    match message {
                        DevnetCommand::Stop(callback) => {
                            // The connection and channel are owned by the thread, but _lent_ to
                            // the callback. The callback has exclusive access to the connection
                            // for the duration of the callback.
                            if let Some(c) = callback {
                                c(&channel);
                            }
                            break;
                        }
                        DevnetCommand::Start(_) => break,
                    }
                }
            } else {
                // todo(ludo): Graceful termination.
            }
        });

        thread::spawn(move || {
            while let Ok(ref message) = log_rx.recv() {
                println!("{:?}", message.message);
            }
        });

        Self {
            tx,
            devnet_event_rx: devnet_events_rx,
        }
    }

    fn start(
        &self,
        callback: Option<DevnetCallback>,
    ) -> Result<(), mpsc::SendError<DevnetCommand>> {
        self.tx.send(DevnetCommand::Start(callback))
    }

    fn stop(&self, callback: Option<DevnetCallback>) -> Result<(), mpsc::SendError<DevnetCommand>> {
        self.tx.send(DevnetCommand::Stop(callback))
    }
}

impl StacksDevnet {
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<StacksDevnet>> {
        let manifest_path = cx.argument::<JsString>(0)?.value(&mut cx);

        let devnet = StacksDevnet::new(&mut cx, manifest_path);
        Ok(cx.boxed(devnet))
    }

    fn js_start(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Get the first argument as a `JsFunction`
        // let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
        // let callback = callback.into_inner(&mut cx);

        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .start(None)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_stop(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .stop(None)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_on_stacks_block(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Get the first argument as a `JsFunction`
        let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
        let callback = callback.into_inner(&mut cx);

        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        while let Ok(message) = devnet.devnet_event_rx.recv() {
            match message {
                NodeObserverEvent::NewStacksBlock => {
                    println!("New stacks block");
                    let args: Vec<Handle<JsValue>> =
                        vec![cx.null().upcast(), cx.number(1 as f64).upcast()];
                    let _res = callback.call(&mut cx, devnet, args)?;
                    // let expected = cx.boolean(true);
                    // if res.strict_equals(&mut cx, expected) {
                    //     break;
                    // }
                    break;
                }
                _ => {}
            }
        }
        Ok(cx.undefined())
    }

    fn js_on_bitcoin_block(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Get the first argument as a `JsFunction`
        let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
        let callback = callback.into_inner(&mut cx);

        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        while let Ok(message) = devnet.devnet_event_rx.recv() {
            match message {
                NodeObserverEvent::NewBitcoinBlock => {
                    println!("New bitcoin block");
                    let args: Vec<Handle<JsValue>> =
                        vec![cx.null().upcast(), cx.number(1 as f64).upcast()];
                    let _res = callback.call(&mut cx, devnet, args)?;
                    // let expected = cx.boolean(true);
                    // if res.strict_equals(&mut cx, expected) {
                    //     break;
                    // }
                    break;
                }
                _ => {}
            }
        }

        Ok(cx.undefined())
    }

    // fn js_on_log(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    //     let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
    //     let callback = callback.into_inner(&mut cx);

    //     let devnet = cx
    //         .this()
    //         .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

    //     thread::spawn(|| {
    //         while let Ok(ref message) = devnet.log_rx.recv() {
    //             // match message {
    //             //     DevnetCommand::Stop(callback) => {
    //             //         // The connection and channel are owned by the thread, but _lent_ to
    //             //         // the callback. The callback has exclusive access to the connection
    //             //         // for the duration of the callback.
    //             //         if let Some(c) = callback {
    //             //             c(&channel);
    //             //         }
    //             //         break;
    //             //     }
    //             //     DevnetCommand::Start(_) => break,
    //             // }
    //         }
    //     });
    //     Ok(cx.undefined())
    // }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("stacksDevnetNew", StacksDevnet::js_new)?;
    cx.export_function("stacksDevnetStart", StacksDevnet::js_start)?;
    cx.export_function("stacksDevnetStop", StacksDevnet::js_stop)?;
    cx.export_function(
        "stacksDevnetWaitForStacksBlock",
        StacksDevnet::js_on_stacks_block,
    )?;
    cx.export_function(
        "stacksDevnetWaitForBitcoinBlock",
        StacksDevnet::js_on_bitcoin_block,
    )?;
    Ok(())
}

fn get_manifest_path_or_exit(path: Option<String>) -> PathBuf {
    println!("");
    if let Some(path) = path {
        let manifest_path = PathBuf::from(path);
        if !manifest_path.exists() {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
        manifest_path
    } else {
        let mut current_dir = env::current_dir().unwrap();
        loop {
            current_dir.push("Clarinet.toml");

            if current_dir.exists() {
                break current_dir;
            }
            current_dir.pop();

            if !current_dir.pop() {
                println!("Could not find Clarinet.toml");
                process::exit(1);
            }
        }
    }
}
