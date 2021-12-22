mod clarity_language_backend;
mod utils;

use clarity_language_backend::ClarityLanguageBackend;
use tokio;
use tower_lsp::{LspService, Server};

pub fn run_lsp() {
    match block_on(do_run_lsp()) {
        Err(_e) => std::process::exit(1),
        _ => {}
    };
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = crate::utils::create_basic_runtime();
    rt.block_on(future)
}

async fn do_run_lsp() -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| ClarityLanguageBackend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
    Ok(())
}
