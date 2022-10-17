use crate::config::Config;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use std::cmp::min;
use std::io::Read;
use std::io::{self, Cursor};
use std::{fs::File, io::Write};
use tar::Archive;

pub async fn download_tsv_file(config: &Config) -> Result<(), String> {
    let destination_path = config.expected_cache_path();
    let url = config.expected_remote_tsv_url();
    let res = reqwest::get(url)
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    // Download chunks
    let (tx, rx) = flume::bounded(0);

    let decoder_thread = std::thread::spawn(move || {
        let input = ChannelRead::new(rx);
        let gz = GzDecoder::new(input);
        let mut archive = Archive::new(gz);
        archive.unpack(destination_path).unwrap();
    });

    if res.status() == reqwest::StatusCode::OK {
        let mut stream = res.bytes_stream();
        while let Some(item) = stream.next().await {
            let chunk = item
                .or(Err(format!("Error while downloading file")))
                .unwrap();
            tx.send_async(chunk.to_vec()).await.unwrap();
        }
        drop(tx);
    }

    tokio::task::spawn_blocking(|| decoder_thread.join())
        .await
        .unwrap()
        .unwrap();

    Ok(())
}

// Wrap a channel into something that impls `io::Read`
struct ChannelRead {
    rx: flume::Receiver<Vec<u8>>,
    current: Cursor<Vec<u8>>,
}

impl ChannelRead {
    fn new(rx: flume::Receiver<Vec<u8>>) -> ChannelRead {
        ChannelRead {
            rx,
            current: Cursor::new(vec![]),
        }
    }
}

impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            // We've exhausted the previous chunk, get a new one.
            if let Ok(vec) = self.rx.recv() {
                self.current = io::Cursor::new(vec);
            }
            // If recv() "fails", it means the sender closed its part of
            // the channel, which means EOF. Propagate EOF by allowing
            // a read from the exhausted cursor.
        }
        self.current.read(buf)
    }
}
