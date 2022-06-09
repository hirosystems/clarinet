use std::future::Future;
use tokio;

pub fn nestable_block_on<F: Future>(future: F) -> F::Output {
    let (handle, _rt) = match tokio::runtime::Handle::try_current() {
        Ok(h) => (h, None),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            (rt.handle().clone(), Some(rt))
        }
    };
    let response = handle.block_on(async { future.await });
    response
}
