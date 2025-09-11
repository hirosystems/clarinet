use std::time::{SystemTime, UNIX_EPOCH};

use hiro_system_kit::slog;
use hyper::header::CONTENT_TYPE;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use prometheus::core::{AtomicU64, GenericGauge};
use prometheus::{self, Encoder, IntGauge, Registry, TextEncoder};
use serde_json::{json, Value as JsonValue};

use crate::utils::Context;

type UInt64Gauge = GenericGauge<AtomicU64>;

#[derive(Debug, Clone)]
pub struct PrometheusMonitoring {
    pub stx_highest_block_appended: UInt64Gauge,
    pub stx_highest_block_received: UInt64Gauge,
    pub stx_highest_block_evaluated: UInt64Gauge,
    pub stx_canonical_fork_lag: UInt64Gauge,
    pub stx_block_evaluation_lag: UInt64Gauge,
    pub stx_last_reorg_timestamp: IntGauge,
    pub stx_last_reorg_applied_blocks: UInt64Gauge,
    pub stx_last_reorg_rolled_back_blocks: UInt64Gauge,
    pub stx_last_block_ingestion_time: UInt64Gauge,
    pub stx_registered_predicates: UInt64Gauge,
    pub stx_deregistered_predicates: UInt64Gauge,
    //
    pub btc_highest_block_appended: UInt64Gauge,
    pub btc_highest_block_received: UInt64Gauge,
    pub btc_highest_block_evaluated: UInt64Gauge,
    pub btc_canonical_fork_lag: UInt64Gauge,
    pub btc_block_evaluation_lag: UInt64Gauge,
    pub btc_last_reorg_timestamp: IntGauge,
    pub btc_last_reorg_applied_blocks: UInt64Gauge,
    pub btc_last_reorg_rolled_back_blocks: UInt64Gauge,
    pub btc_last_block_ingestion_time: UInt64Gauge,
    pub btc_registered_predicates: UInt64Gauge,
    pub btc_deregistered_predicates: UInt64Gauge,
    pub registry: Registry,
}

impl Default for PrometheusMonitoring {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusMonitoring {
    pub fn new() -> PrometheusMonitoring {
        let registry = Registry::new();
        // stacks metrics
        let stx_highest_block_appended = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_highest_block_appended",
            "The highest Stacks block successfully appended to a Chainhook node fork.",
        );
        let stx_highest_block_received = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_highest_block_received",
            "The highest Stacks block received by the Chainhook node from the Stacks node.",
        );
        let stx_highest_block_evaluated = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_highest_block_evaluated",
            "The highest Stacks block successfully evaluated against predicates.",
        );
        let stx_canonical_fork_lag = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_canonical_fork_lag",
            "The difference between the highest Stacks block received and the highest Stacks block appended.",
        );
        let stx_block_evaluation_lag = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_block_evaluation_lag",
            "The difference between the highest Stacks block appended and the highest Stacks block evaluated.",
        );
        let stx_last_reorg_timestamp = PrometheusMonitoring::create_and_register_int_gauge(
            &registry,
            "chainhook_stx_last_reorg_timestamp",
            "The timestamp of the latest Stacks reorg ingested by the Chainhook node.",
        );
        let stx_last_reorg_applied_blocks = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_last_reorg_applied_blocks",
            "The number of blocks applied to the Stacks chain as part of the latest Stacks reorg.",
        );
        let stx_last_reorg_rolled_back_blocks =
            PrometheusMonitoring::create_and_register_uint64_gauge(
                &registry,
                "chainhook_stx_last_reorg_rolled_back_blocks",
                "The number of blocks rolled back from the Stacks chain as part of the latest Stacks reorg.",
            );
        let stx_last_block_ingestion_time = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_last_block_ingestion_time",
            "The time that the Chainhook node last ingested a Stacks block.",
        );
        let stx_registered_predicates = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_registered_predicates",
            "The number of Stacks predicates that have been registered by the Chainhook node.",
        );
        let stx_deregistered_predicates = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_stx_deregistered_predicates",
            "The number of Stacks predicates that have been deregistered by the Chainhook node.",
        );

        // bitcoin metrics
        let btc_highest_block_appended = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_highest_block_appended",
            "The highest Bitcoin block successfully appended to a Chainhook node fork.",
        );
        let btc_highest_block_received = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_highest_block_received",
            "The highest Bitcoin block received by the Chainhook node from the Bitcoin node.",
        );
        let btc_highest_block_evaluated = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_highest_block_evaluated",
            "The highest Bitcoin block successfully evaluated against predicates.",
        );
        let btc_canonical_fork_lag = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_canonical_fork_lag",
            "The difference between the highest Bitcoin block received and the highest Bitcoin block appended.",
        );
        let btc_block_evaluation_lag = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_block_evaluation_lag",
            "The difference between the highest Bitcoin block appended and the highest Bitcoin block evaluated.",
        );
        let btc_last_reorg_timestamp = PrometheusMonitoring::create_and_register_int_gauge(
            &registry,
            "chainhook_btc_last_reorg_timestamp",
            "The timestamp of the latest Bitcoin reorg ingested by the Chainhook node.",
        );
        let btc_last_reorg_applied_blocks =
            PrometheusMonitoring::create_and_register_uint64_gauge(
                &registry,
                "chainhook_btc_last_reorg_applied_blocks",
                "The number of blocks applied to the Bitcoin chain as part of the latest Bitcoin reorg.",
            );
        let btc_last_reorg_rolled_back_blocks =
            PrometheusMonitoring::create_and_register_uint64_gauge(
                &registry,
                "chainhook_btc_last_reorg_rolled_back_blocks",
                "The number of blocks rolled back from the Bitcoin chain as part of the latest Bitcoin reorg.",
            );
        let btc_last_block_ingestion_time = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_last_block_ingestion_time",
            "The time that the Chainhook node last ingested a Bitcoin block.",
        );
        let btc_registered_predicates = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_registered_predicates",
            "The number of Bitcoin predicates that have been registered by the Chainhook node.",
        );
        let btc_deregistered_predicates = PrometheusMonitoring::create_and_register_uint64_gauge(
            &registry,
            "chainhook_btc_deregistered_predicates",
            "The number of Bitcoin predicates that have been deregistered by the Chainhook node.",
        );

        PrometheusMonitoring {
            stx_highest_block_appended,
            stx_highest_block_received,
            stx_highest_block_evaluated,
            stx_canonical_fork_lag,
            stx_block_evaluation_lag,
            stx_last_reorg_timestamp,
            stx_last_reorg_applied_blocks,
            stx_last_reorg_rolled_back_blocks,
            stx_last_block_ingestion_time,
            stx_registered_predicates,
            stx_deregistered_predicates,
            //
            btc_highest_block_appended,
            btc_highest_block_received,
            btc_highest_block_evaluated,
            btc_canonical_fork_lag,
            btc_block_evaluation_lag,
            btc_last_reorg_timestamp,
            btc_last_reorg_applied_blocks,
            btc_last_reorg_rolled_back_blocks,
            btc_last_block_ingestion_time,
            btc_registered_predicates,
            btc_deregistered_predicates,
            registry,
        }
    }
    // setup helpers
    pub fn create_and_register_uint64_gauge(
        registry: &Registry,
        name: &str,
        help: &str,
    ) -> UInt64Gauge {
        let g = UInt64Gauge::new(name, help).unwrap();
        registry.register(Box::new(g.clone())).unwrap();
        g
    }

    pub fn create_and_register_int_gauge(registry: &Registry, name: &str, help: &str) -> IntGauge {
        let g = IntGauge::new(name, help).unwrap();
        registry.register(Box::new(g.clone())).unwrap();
        g
    }

    pub fn initialize(
        &self,
        stx_predicates: u64,
        btc_predicates: u64,
        initial_stx_block: Option<u64>,
    ) {
        self.stx_metrics_set_registered_predicates(stx_predicates);
        self.btc_metrics_set_registered_predicates(btc_predicates);
        if let Some(initial_stx_block) = initial_stx_block {
            self.stx_metrics_block_received(initial_stx_block);
            self.stx_metrics_block_appeneded(initial_stx_block);
            self.stx_metrics_block_evaluated(initial_stx_block);
        }
    }

    // stx helpers
    pub fn stx_metrics_deregister_predicate(&self) {
        self.stx_registered_predicates.dec();
        self.stx_deregistered_predicates.inc();
    }

    pub fn stx_metrics_register_predicate(&self) {
        self.stx_registered_predicates.inc();
    }
    pub fn stx_metrics_set_registered_predicates(&self, registered_predicates: u64) {
        self.stx_registered_predicates.set(registered_predicates);
    }

    pub fn stx_metrics_set_reorg(
        &self,
        timestamp: i64,
        applied_blocks: u64,
        rolled_back_blocks: u64,
    ) {
        self.stx_last_reorg_timestamp.set(timestamp);
        self.stx_last_reorg_applied_blocks.set(applied_blocks);
        self.stx_last_reorg_rolled_back_blocks
            .set(rolled_back_blocks);
    }

    pub fn stx_metrics_block_appeneded(&self, new_block_height: u64) {
        let highest_appended = self.stx_highest_block_appended.get();
        if new_block_height > highest_appended {
            self.stx_highest_block_appended.set(new_block_height);

            let highest_received = self.stx_highest_block_received.get();
            self.stx_canonical_fork_lag
                .set(highest_received.saturating_sub(new_block_height));

            let highest_evaluated = self.stx_highest_block_evaluated.get();
            self.stx_block_evaluation_lag
                .set(new_block_height.saturating_sub(highest_evaluated));
        }
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Could not get current time in ms")
            .as_secs();
        self.stx_last_block_ingestion_time.set(time);
    }

    pub fn stx_metrics_block_received(&self, new_block_height: u64) {
        let highest_received = self.stx_highest_block_received.get();
        if new_block_height > highest_received {
            self.stx_highest_block_received.set(new_block_height);

            let highest_appended = self.stx_highest_block_appended.get();
            self.stx_canonical_fork_lag
                .set(new_block_height.saturating_sub(highest_appended));
        }
    }

    pub fn stx_metrics_block_evaluated(&self, new_block_height: u64) {
        let highest_evaluated = self.stx_highest_block_evaluated.get();
        if new_block_height > highest_evaluated {
            self.stx_highest_block_evaluated.set(new_block_height);

            let highest_appended = self.stx_highest_block_appended.get();
            self.stx_block_evaluation_lag
                .set(highest_appended.saturating_sub(new_block_height));
        }
    }

    // btc helpers
    pub fn btc_metrics_deregister_predicate(&self) {
        self.btc_registered_predicates.dec();
        self.btc_deregistered_predicates.inc();
    }

    pub fn btc_metrics_register_predicate(&self) {
        self.btc_registered_predicates.inc();
    }

    pub fn btc_metrics_set_registered_predicates(&self, registered_predicates: u64) {
        self.btc_registered_predicates.set(registered_predicates);
    }

    pub fn btc_metrics_set_reorg(
        &self,
        timestamp: i64,
        applied_blocks: u64,
        rolled_back_blocks: u64,
    ) {
        self.btc_last_reorg_timestamp.set(timestamp);
        self.btc_last_reorg_applied_blocks.set(applied_blocks);
        self.btc_last_reorg_rolled_back_blocks
            .set(rolled_back_blocks);
    }

    pub fn btc_metrics_block_appended(&self, new_block_height: u64) {
        let highest_appended = self.btc_highest_block_appended.get();
        if new_block_height > highest_appended {
            self.btc_highest_block_appended.set(new_block_height);

            let highest_received = self.btc_highest_block_received.get();
            self.btc_canonical_fork_lag
                .set(highest_received.saturating_sub(new_block_height));

            let highest_evaluated = self.btc_highest_block_evaluated.get();
            self.btc_block_evaluation_lag
                .set(new_block_height.saturating_sub(highest_evaluated));
        }
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Could not get current time in ms")
            .as_secs();
        self.btc_last_block_ingestion_time.set(time);
    }

    pub fn btc_metrics_block_received(&self, new_block_height: u64) {
        let highest_received = self.btc_highest_block_received.get();
        if new_block_height > highest_received {
            self.btc_highest_block_received.set(new_block_height);

            let highest_appended = self.btc_highest_block_appended.get();

            self.btc_canonical_fork_lag
                .set(new_block_height.saturating_sub(highest_appended));
        }
    }

    pub fn btc_metrics_block_evaluated(&self, new_block_height: u64) {
        let highest_evaluated = self.btc_highest_block_evaluated.get();
        if new_block_height > highest_evaluated {
            self.btc_highest_block_evaluated.set(new_block_height);

            let highest_appended = self.btc_highest_block_appended.get();
            self.btc_block_evaluation_lag
                .set(highest_appended.saturating_sub(new_block_height));
        }
    }

    pub fn get_metrics(&self) -> JsonValue {
        json!({
            "bitcoin": {
                "last_received_block_height": self.btc_highest_block_received.get(),
                "last_appended_block_height": self.btc_highest_block_appended.get(),
                "last_evaluated_block_height": self.btc_highest_block_evaluated.get(),
                "canonical_fork_lag": self.btc_canonical_fork_lag.get(),
                "block_evaluation_lag": self.btc_block_evaluation_lag.get(),
                "last_block_ingestion_at": self.btc_last_block_ingestion_time.get(),
                "last_reorg": {
                    "timestamp": self.btc_last_reorg_timestamp.get(),
                    "applied_blocks": self.btc_last_reorg_applied_blocks.get(),
                    "rolled_back_blocks": self.btc_last_reorg_rolled_back_blocks.get(),
                },
                "registered_predicates": self.btc_registered_predicates.get(),
                "deregistered_predicates": self.btc_deregistered_predicates.get(),
            },
            "stacks": {
                "last_received_block_height": self.stx_highest_block_received.get(),
                "last_appended_block_height": self.stx_highest_block_appended.get(),
                "last_evaluated_block_height": self.stx_highest_block_evaluated.get(),
                "canonical_fork_lag": self.stx_canonical_fork_lag.get(),
                "block_evaluation_lag": self.btc_block_evaluation_lag.get(),
                "last_block_ingestion_at": self.stx_last_block_ingestion_time.get(),
                "last_reorg": {
                    "timestamp": self.stx_last_reorg_timestamp.get(),
                    "applied_blocks": self.stx_last_reorg_applied_blocks.get(),
                    "rolled_back_blocks": self.stx_last_reorg_rolled_back_blocks.get(),
                },
                "registered_predicates": self.stx_registered_predicates.get(),
                "deregistered_predicates": self.stx_deregistered_predicates.get(),
            }
        })
    }
}

async fn serve_req(
    req: Request<Body>,
    registry: Registry,
    ctx: Context,
) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            ctx.try_log(|logger| {
                slog::debug!(
                    logger,
                    "Prometheus monitoring: responding to metrics request"
                )
            });

            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = vec![];
            let response = match encoder.encode(&metric_families, &mut buffer) {
                Ok(_) => Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, encoder.format_type())
                    .body(Body::from(buffer))
                    .unwrap(),
                Err(e) => {
                    ctx.try_log(|logger| {
                        slog::debug!(
                            logger,
                            "Prometheus monitoring: failed to encode metrics: {}",
                            e.to_string()
                        )
                    });
                    Response::builder().status(500).body(Body::empty()).unwrap()
                }
            };
            Ok(response)
        }
        (_, _) => {
            ctx.try_log(|logger| {
                slog::debug!(
                    logger,
                    "Prometheus monitoring: received request with invalid method/route: {}/{}",
                    req.method(),
                    req.uri().path()
                )
            });
            let response = Response::builder().status(404).body(Body::empty()).unwrap();

            Ok(response)
        }
    }
}

pub async fn start_serving_prometheus_metrics(port: u16, registry: Registry, ctx: Context) {
    let addr = ([0, 0, 0, 0], port).into();
    let ctx_clone = ctx.clone();
    let make_svc = make_service_fn(|_| {
        let registry = registry.clone();
        let ctx_clone = ctx_clone.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |r| {
                serve_req(r, registry.clone(), ctx_clone.clone())
            }))
        }
    });
    let serve_future = Server::bind(&addr).serve(make_svc);

    ctx.try_log(|logger| slog::info!(logger, "Prometheus monitoring: listening on port {}", port));

    if let Err(err) = serve_future.await {
        ctx.try_log(|logger| slog::warn!(logger, "Prometheus monitoring: server error: {}", err));
    }
}

#[cfg(test)]
mod test {
    use std::thread::sleep;
    use std::time::Duration;

    use super::PrometheusMonitoring;

    #[test]
    fn it_tracks_stx_predicate_registration_deregistration_with_defaults() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.stx_registered_predicates.get(), 0);
        assert_eq!(prometheus.stx_deregistered_predicates.get(), 0);
        prometheus.stx_metrics_set_registered_predicates(10);
        assert_eq!(prometheus.stx_registered_predicates.get(), 10);
        assert_eq!(prometheus.stx_deregistered_predicates.get(), 0);
        prometheus.stx_metrics_register_predicate();
        assert_eq!(prometheus.stx_registered_predicates.get(), 11);
        assert_eq!(prometheus.stx_deregistered_predicates.get(), 0);
        prometheus.stx_metrics_deregister_predicate();
        assert_eq!(prometheus.stx_registered_predicates.get(), 10);
        assert_eq!(prometheus.stx_deregistered_predicates.get(), 1);
    }

    #[test]
    fn it_tracks_stx_reorgs() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.stx_last_reorg_timestamp.get(), 0);
        assert_eq!(prometheus.stx_last_reorg_applied_blocks.get(), 0);
        assert_eq!(prometheus.stx_last_reorg_rolled_back_blocks.get(), 0);
        prometheus.stx_metrics_set_reorg(10000, 1, 1);
        assert_eq!(prometheus.stx_last_reorg_timestamp.get(), 10000);
        assert_eq!(prometheus.stx_last_reorg_applied_blocks.get(), 1);
        assert_eq!(prometheus.stx_last_reorg_rolled_back_blocks.get(), 1);
    }

    #[test]
    fn it_tracks_stx_block_ingestion() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.stx_highest_block_appended.get(), 0);
        assert_eq!(prometheus.stx_last_block_ingestion_time.get(), 0);
        // receive a block
        prometheus.stx_metrics_block_received(100);
        // verify our lag in block appendation
        assert_eq!(prometheus.stx_canonical_fork_lag.get(), 100);
        // now append the block
        prometheus.stx_metrics_block_appeneded(100);
        assert_eq!(prometheus.stx_highest_block_appended.get(), 100);
        let time = prometheus.stx_last_block_ingestion_time.get();
        assert!(time > 0);
        // verify our lag is resolved after appending
        assert_eq!(prometheus.stx_canonical_fork_lag.get(), 0);
        // verify our lag in block evaluation
        assert_eq!(prometheus.stx_block_evaluation_lag.get(), 100);
        // now evaluate a block
        prometheus.stx_metrics_block_evaluated(100);
        // verify our lag is resolved after evaluating
        assert_eq!(prometheus.stx_block_evaluation_lag.get(), 0);
        // ingesting a block lower than previous tip will
        // update ingestion time but not highest block ingested
        sleep(Duration::new(1, 0));
        prometheus.stx_metrics_block_appeneded(99);
        assert_eq!(prometheus.stx_highest_block_appended.get(), 100);
        assert!(prometheus.stx_last_block_ingestion_time.get() > time);
    }

    #[test]
    fn it_tracks_btc_predicate_registration_deregistration_with_defaults() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.btc_registered_predicates.get(), 0);
        assert_eq!(prometheus.btc_deregistered_predicates.get(), 0);
        prometheus.btc_metrics_set_registered_predicates(10);
        assert_eq!(prometheus.btc_registered_predicates.get(), 10);
        assert_eq!(prometheus.btc_deregistered_predicates.get(), 0);
        prometheus.btc_metrics_register_predicate();
        assert_eq!(prometheus.btc_registered_predicates.get(), 11);
        assert_eq!(prometheus.btc_deregistered_predicates.get(), 0);
        prometheus.btc_metrics_deregister_predicate();
        assert_eq!(prometheus.btc_registered_predicates.get(), 10);
        assert_eq!(prometheus.btc_deregistered_predicates.get(), 1);
    }

    #[test]
    fn it_tracks_btc_reorgs() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.btc_last_reorg_timestamp.get(), 0);
        assert_eq!(prometheus.btc_last_reorg_applied_blocks.get(), 0);
        assert_eq!(prometheus.btc_last_reorg_rolled_back_blocks.get(), 0);
        prometheus.btc_metrics_set_reorg(10000, 1, 1);
        assert_eq!(prometheus.btc_last_reorg_timestamp.get(), 10000);
        assert_eq!(prometheus.btc_last_reorg_applied_blocks.get(), 1);
        assert_eq!(prometheus.btc_last_reorg_rolled_back_blocks.get(), 1);
    }

    #[test]
    fn it_tracks_btc_block_ingestion() {
        let prometheus = PrometheusMonitoring::new();
        assert_eq!(prometheus.btc_highest_block_appended.get(), 0);
        assert_eq!(prometheus.btc_last_block_ingestion_time.get(), 0);
        // receive a block
        prometheus.btc_metrics_block_received(100);
        // verify our lag in block appendation
        assert_eq!(prometheus.btc_canonical_fork_lag.get(), 100);
        // now append the block
        prometheus.btc_metrics_block_appended(100);
        assert_eq!(prometheus.btc_highest_block_appended.get(), 100);
        let time = prometheus.btc_last_block_ingestion_time.get();
        assert!(time > 0);
        // verify our lag is resolved after appending
        assert_eq!(prometheus.btc_canonical_fork_lag.get(), 0);
        // verify our lag in block evaluation
        assert_eq!(prometheus.btc_block_evaluation_lag.get(), 100);
        // now evaluate a block
        prometheus.btc_metrics_block_evaluated(100);
        // verify our lag is resolved after evaluating
        assert_eq!(prometheus.btc_block_evaluation_lag.get(), 0);
        // ingesting a block lower than previous tip will
        // update ingestion time but not highest block ingested
        sleep(Duration::new(1, 0));
        prometheus.btc_metrics_block_appended(99);
        assert_eq!(prometheus.btc_highest_block_appended.get(), 100);
        assert!(prometheus.btc_last_block_ingestion_time.get() > time);
    }
}
