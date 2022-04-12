use clarity_repl::clarity::util::hash::{bytes_to_hex, Hash160};
use mac_address::get_mac_address;
use segment::{
    message::{Message, Track, User},
    Client, HttpClient,
};

use crate::types::StacksNetwork;

pub enum DeveloperUsageEvent {
    NewProject(DeveloperUsageDigest),
    PokeExecuted(DeveloperUsageDigest),
    CheckExecuted(DeveloperUsageDigest),
    TestSuiteExecuted(DeveloperUsageDigest, bool, u32),
    DevnetExecuted(DeveloperUsageDigest),
    ContractPublished(DeveloperUsageDigest, StacksNetwork),
    DebugStarted(DeveloperUsageDigest, u32),
    UnknownCommand(DeveloperUsageDigest, String),
}

pub struct DeveloperUsageDigest {
    pub project_id: String,
    pub team_id: String,
}

impl DeveloperUsageDigest {
    pub fn new(project_id: &str, team_id: &Vec<String>) -> Self {
        let hashed_project_id = Hash160::from_data(project_id.as_bytes());
        let hashed_team_id = Hash160::from_data(team_id.join(",").as_bytes());
        Self {
            project_id: format!("0x{}", bytes_to_hex(&hashed_project_id.to_bytes().to_vec())),
            team_id: format!("0x{}", bytes_to_hex(&hashed_team_id.to_bytes().to_vec())),
        }
    }
}

pub fn telemetry_report_event(event: DeveloperUsageEvent) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(32)
        .build()
        .unwrap();

    rt.block_on(send_event(event));
}

async fn send_event(event: DeveloperUsageEvent) {
    let segment_api_key = "Q3xpmFRvy0psXnwBEXErtMBIeabOVjbC";

    let clarinet_version = option_env!("CARGO_PKG_VERSION")
        .expect("Unable to detect version")
        .to_string();
    let ci_mode = option_env!("CLARINET_MODE_CI").unwrap_or("0").to_string();

    let (event_name, properties) = match event {
        DeveloperUsageEvent::NewProject(digest) => (
            "NewProject",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
            }),
        ),
        DeveloperUsageEvent::TestSuiteExecuted(digest, success, count) => (
            "TestSuiteExecuted",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
                "success": success,
                "count": count,
            }),
        ),
        DeveloperUsageEvent::DevnetExecuted(digest) => (
            "DevnetExecuted",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
            }),
        ),
        DeveloperUsageEvent::ContractPublished(digest, network) => (
            "ContractPublished",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
                "network": format!("{:?}", network),
            }),
        ),
        DeveloperUsageEvent::CheckExecuted(digest) => (
            "CheckExecuted",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
            }),
        ),
        DeveloperUsageEvent::PokeExecuted(digest) => (
            "PokeExecuted",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
            }),
        ),
        DeveloperUsageEvent::DebugStarted(digest, num_sessions) => (
            "DebugStarted",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
                "sessions": num_sessions,
            }),
        ),
        DeveloperUsageEvent::UnknownCommand(digest, command) => (
            "UnknownCommand",
            json!({
                "project_id": digest.project_id,
                "team_id": digest.team_id,
                "clarinet_version": clarinet_version,
                "ci_mode": ci_mode,
                "command": command,
            }),
        ),
    };

    let user_id = match get_mac_address() {
        Ok(Some(ma)) => Hash160::from_data(&ma.bytes()),
        Ok(None) => Hash160::from_data(&[0]),
        Err(_e) => return,
    };

    let client = HttpClient::default();

    let _ = client
        .send(
            segment_api_key.to_string(),
            Message::from(Track {
                user: User::UserId {
                    user_id: format!("0x{}", bytes_to_hex(&user_id.to_bytes().to_vec())),
                },
                event: event_name.into(),
                properties: properties,
                ..Default::default()
            }),
        )
        .await;
}
