use std::future::Future;

pub fn create_basic_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(32)
        .build()
        .unwrap()
}

pub fn nestable_block_on<F: Future>(future: F) -> F::Output {
    let (handle, _rt) = match tokio::runtime::Handle::try_current() {
        Ok(h) => (h, None),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            (rt.handle().clone(), Some(rt))
        }
    };
    handle.block_on(future)
}

// pub fn spawn_async_thread_named<F: Future>(name: &str, f: F) -> io::Result<JoinHandle<F::Output>> {
//     thread_named(name).spawn(move || {
//         nestable_block_on(f)
//     })
// }
