use std::sync::mpsc;
use std::thread;
use std::path::PathBuf;
use std::{env, process};
use clarinet_lib::types::DevnetConfigFile;
use neon::prelude::*;
use clarinet_lib::integrate::{self, DevnetOrchestrator, NodeObserverEvent};

type DevnetCallback = Box<dyn FnOnce(&Channel) + Send>;

struct StacksDevnet {
    tx: mpsc::Sender<DevnetMessage>,
    devnet_event_rx: mpsc::Receiver<NodeObserverEvent>
}

enum DevnetMessage {
    Callback(DevnetCallback),
    Close,
}

impl Finalize for StacksDevnet {}

impl StacksDevnet {
    // Creates a new instance of `Database`
    //
    // 1. Creates a connection and a channel
    // 2. Spawns a thread and moves the channel receiver and connection to it
    // 3. On a separate thread, read closures off the channel and execute with access
    //    to the connection.
    fn new<'a, C>(cx: &mut C) -> Result<Self, String>
    where
        C: Context<'a>,
    {
        // Channel for sending callbacks to execute on the sqlite connection thread
        let (tx, rx) = mpsc::channel::<DevnetMessage>();
        let (devnet_events_tx, devnet_events_rx) = mpsc::channel();

        // Open a connection sqlite, this will be moved to the thread
        // let mut conn = Connection::open_in_memory()?;

        // Create an `Channel` for calling back to JavaScript. It is more efficient
        // to create a single channel and re-use it for all database callbacks.
        // The JavaScript process will not exit as long as this channel has not been
        // dropped.
        let channel = cx.channel();

        // Create a table in the in-memory database
        // In production code, this would likely be handled somewhere else
        // conn.execute(
        //     r#"
        //         CREATE TABLE person (
        //             id   INTEGER PRIMARY KEY AUTOINCREMENT,
        //             name TEXT NOT NULL
        //         )
        //     "#,
        //     [],
        // )?;

        // Spawn a thread for processing database queries
        // This will not block the JavaScript main thread and will continue executing
        // concurrently.
        thread::spawn(move || {


            let manifest_path = get_manifest_path_or_exit(Some("/Users/ludovic/Coding/clarinet/clarinet-cli/examples/counter/Clarinet.toml".into()));
            let devnet_overrides = DevnetConfigFile::default();
            let devnet = DevnetOrchestrator::new(manifest_path, Some(devnet_overrides));
            integrate::run_devnet(devnet, Some(devnet_events_tx),false);

            // Blocks until a callback is available
            // When the instance of `Database` is dropped, the channel will be closed
            // and `rx.recv()` will return an `Err`, ending the loop and terminating
            // the thread.
            while let Ok(message) = rx.recv() {
                match message {
                    DevnetMessage::Callback(f) => {
                        // The connection and channel are owned by the thread, but _lent_ to
                        // the callback. The callback has exclusive access to the connection
                        // for the duration of the callback.
                        f(&channel);
                    }
                    // Immediately close the connection, even if there are pending messages
                    DevnetMessage::Close => break,
                }
            }
        });

        Ok(Self { tx, devnet_event_rx: devnet_events_rx })
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    fn close(&self) -> Result<(), mpsc::SendError<DevnetMessage>> {
        self.tx.send(DevnetMessage::Close)
    }

    fn send(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DevnetMessage>> {
        self.tx.send(DevnetMessage::Callback(Box::new(callback)))
    }
}

// Methods exposed to JavaScript
// The `JsBox` boxed `Database` is expected as the `this` value on all methods except `js_new`
impl StacksDevnet {

    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<StacksDevnet>> {
        let devnet = StacksDevnet::new(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;
        Ok(cx.boxed(devnet))
    }

    fn js_start(mut cx: FunctionContext) -> JsResult<JsUndefined> {


        Ok(cx.undefined())
    }

    fn js_terminate(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .close()
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    // Inserts a `name` into the database
    // Accepts a `name` and a `callback` as parameters
    fn js_on_stacks_block(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Get the first argument as a `JsString` and convert to a Rust `String`
        // let name = cx.argument::<JsString>(0)?.value(&mut cx);

        // Get the first argument as a `JsFunction`
        let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let this = cx.this().downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;


        let devnet = cx.this().downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;
        let callback = callback.into_inner(&mut cx);

        while let Ok(message) = devnet.devnet_event_rx.recv() {
            match message {
                _ => {
                println!("Hello world :)");
                // let this = cx.undefined();
                let args: Vec<Handle<JsValue>> = vec![cx.null().upcast(), cx.number(1 as f64).upcast()];
                let res = callback.call(&mut cx, this, args)?;
                let expected = cx.boolean(true);
                if res.strict_equals(&mut cx, expected) {
                    break;
                }
                //     // callback.call(&mut cx, this, vec![])?;

                // }
                // DevnetMessage::Callback(f) => {
                //     // The connection and channel are owned by the thread, but _lent_ to
                //     // the callback. The callback has exclusive access to the connection
                //     // for the duration of the callback.
                //     f(&channel);
                // }
                // // Immediately close the connection, even if there are pending messages
                // DevnetMessage::Close => break,
                
                }
            }
        }   



        // db.send(move |conn, channel| {
        //     let result = conn
        //         .execute(
        //             "INSERT INTO person (name) VALUES (?)",
        //             rusqlite::params![name],
        //         )
        //         .map(|_| conn.last_insert_rowid());

        //     channel.send(move |mut cx| {
        //         let callback = callback.into_inner(&mut cx);
        //         let this = cx.undefined();
        //         let args: Vec<Handle<JsValue>> = match result {
        //             Ok(id) => vec![cx.null().upcast(), cx.number(id as f64).upcast()],
        //             Err(err) => vec![cx.error(err.to_string())?.upcast()],
        //         };

        //         callback.call(&mut cx, this, args)?;

        //         Ok(())
        //     });
        // })
        // .or_else(|err| cx.throw_error(err.to_string()))?;

        // This function does not have a return value
        Ok(cx.undefined())
    }

    // Inserts a `name` into the database
    // Accepts a `name` and a `callback` as parameters
    fn js_on_bitcoin_block(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Get the first argument as a `JsString` and convert to a Rust `String`
        // let name = cx.argument::<JsString>(0)?.value(&mut cx);

        // Get the first argument as a `JsFunction`
        let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let this = cx.this().downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let devnet = cx.this().downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        while let Ok(message) = devnet.devnet_event_rx.recv() {
            match message {
                _ => {
                println!("Hello world :)");
                // let this = cx.undefined();
                let args: Vec<Handle<JsValue>> = vec![cx.null().upcast(), cx.number(1 as f64).upcast()];
                let callback = callback.into_inner(&mut cx);
                let res = callback.call(&mut cx, this, args)?;
                break;
                //     // callback.call(&mut cx, this, vec![])?;

                // }
                // DevnetMessage::Callback(f) => {
                //     // The connection and channel are owned by the thread, but _lent_ to
                //     // the callback. The callback has exclusive access to the connection
                //     // for the duration of the callback.
                //     f(&channel);
                // }
                // // Immediately close the connection, even if there are pending messages
                // DevnetMessage::Close => break,
                
                }
            }
        }   

        // db.send(move |conn, channel| {
        //     let result = conn
        //         .execute(
        //             "INSERT INTO person (name) VALUES (?)",
        //             rusqlite::params![name],
        //         )
        //         .map(|_| conn.last_insert_rowid());

        //     channel.send(move |mut cx| {
        //         let callback = callback.into_inner(&mut cx);
        //         let this = cx.undefined();
        //         let args: Vec<Handle<JsValue>> = match result {
        //             Ok(id) => vec![cx.null().upcast(), cx.number(id as f64).upcast()],
        //             Err(err) => vec![cx.error(err.to_string())?.upcast()],
        //         };

        //         callback.call(&mut cx, this, args)?;

        //         Ok(())
        //     });
        // })
        // .or_else(|err| cx.throw_error(err.to_string()))?;

        // This function does not have a return value
        Ok(cx.undefined())
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("stackDevnetNew", StacksDevnet::js_new)?;
    cx.export_function("stackDevnetStart", StacksDevnet::js_start)?;
    cx.export_function("stackDevnetOnStacksBlock", StacksDevnet::js_on_stacks_block)?;
    cx.export_function("stackDevnetOnBitcoinBlock", StacksDevnet::js_on_bitcoin_block)?;
    cx.export_function("stackDevnetTerminate", StacksDevnet::js_terminate)?;
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
