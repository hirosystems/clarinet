mod net;

use clap::{App, Arg, SubCommand};
use libp2p::identity::Keypair;
use libp2p::Multiaddr;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("bbtc-node")
        .version("0.1.0")
        .author("Ludo Galabru <ludovic@galabru.com>")
        .about("BTC on Stacks chain")
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("start").arg(
                Arg::with_name("bootstrap-node")
                    .short("b")
                    .help("bootstrap node to use")
                    .takes_value(true),
            ),
        )
        .subcommand(
            SubCommand::with_name("stop").arg(
                Arg::with_name("debug")
                    .short("d")
                    .help("print debug information verbosely"),
            ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("start") {
        println!("Starting bbtc-node");
        let local_key = Keypair::generate_secp256k1();
        let bootstrap_node = match matches.value_of("bootstrap-node") {
            Some(bootstrap_node) => {
                let addr: Multiaddr = bootstrap_node.parse()?;
                Some(addr)
            }
            None => None,
        };

        net::start_networking(&local_key, bootstrap_node)
    } else {
        Ok(())
    }
}
