use serde::{de::DeserializeOwned, ser, Serialize};
use serde_json::value::RawValue;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProtocolMessage {
    pub seq: i64,
    #[serde(flatten)]
    pub message: MessageKind,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum MessageKind {
    Request(RequestCommand),
    Response(Response),
    Event(Event),
}

// Requests

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", tag = "command", content = "arguments")]
pub enum RequestCommand {
    Initialize(InitializeRequestArguments),
    Launch(LaunchRequestArguments),
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum RequestArguments {
    Initialize(InitializeRequestArguments),
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArguments {
    #[serde(rename = "clientID", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    #[serde(rename = "adapterID")]
    pub adapter_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_start_at_1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns_start_at_1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_variable_type: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_variable_paging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_run_in_terminal_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_memory_references: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_progress_reporting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_invalidated_event: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_memory_event: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArguments {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub __restart: Option<bool>,
}

// Events

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct Event {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<EventBody>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct EventBody {}

// Responses

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct Response {
    pub request_seq: i64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ResponseBody>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(tag = "command", content = "body")]
pub enum ResponseBody {
    Initialize(InitializeResponse),
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct InitializeResponse {
    pub capabilities: Capabilities,
}

// Types

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(i32),
    String(String),
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_configuration_done_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_function_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_conditional_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_hit_conditional_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_evaluate_for_hovers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_breakpoint_filters: Option<Vec<ExceptionBreakpointsFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_back: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_variable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_restart_frame: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_goto_targets_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_in_targets_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_completions_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_trigger_characters: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_modules_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_module_columns: Option<Vec<ColumnDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_checksum_algorithms: Option<Vec<ChecksumAlgorithm>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_restart_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_value_formatting_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_info_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_terminate_debuggee: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_suspend_debuggee: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_delayed_stack_trace_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_loaded_sources_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_log_points: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_threads_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_expression: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_data_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_read_memory_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_write_memory_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_disassemble_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_cancel_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_breakpoint_locations_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_clipboard_context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_stepping_granularity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_instruction_breakpoints: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_filter_options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_single_thread_execution_requests: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    pub filter: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_condition: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_description: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_telemetry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_user: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_label: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub id: NumberOrString,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_optimized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_code: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time_stamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_range: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    String,
    Number,
    Boolean,
    #[serde(rename = "unixTimestampUTC")]
    UnixTimestampUTC,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDescriptor {
    pub attribute_name: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "type")]
    pub column_type: ColumnType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct ModulesViewDescriptor {
    pub columns: Vec<ColumnDescriptor>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PresentationHint {
    Normal,
    Emphasize,
    Deemphasize,
    Label,
    Subtle,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<PresentationHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub sources: Vec<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub line: i64,
    pub column: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_restart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_pointer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<NumberOrString>,
    pub presentation_hint: PresentationHint,
}

// TODO(brice): left off here copying types from https://microsoft.github.io/debug-adapter-protocol/specification#Types_Scope

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub enum ChecksumAlgorithm {
    MD5,
    SHA1,
    SHA256,
    #[serde(rename = "timestamp")]
    Timestamp,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct Checksum {
    pub algorithm: ChecksumAlgorithm,
    pub checksum: String,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn test_serialize() {
        let message = ProtocolMessage {
            seq: 1,
            message: MessageKind::Request(RequestCommand::Initialize(Some(
                InitializeRequestArguments {
                    client_id: Some("client".to_string()),
                    client_name: Some("name".to_string()),
                    adapter_id: "my-id".to_string(),
                    locale: None,
                    lines_start_at_1: Some(true),
                    columns_start_at_1: Some(true),
                    path_format: Some("path-format".to_string()),
                    supports_variable_type: None,
                    supports_variable_paging: Some(true),
                    supports_run_in_terminal_request: Some(false),
                    supports_memory_references: Some(false),
                    supports_progress_reporting: Some(false),
                    supports_invalidated_event: None,
                    supports_memory_event: None,
                },
            ))),
        };
        println!("{}", serde_json::to_string(&message).unwrap());
    }

    #[test]
    fn test_deserialize() {
        let json = r#"{"command":"initialize","arguments":{"clientID":"vscode","clientName":"Visual Studio Code","adapterID":"clarinet","pathFormat":"path","linesStartAt1":true,"columnsStartAt1":true,"supportsVariableType":true,"supportsVariablePaging":true,"supportsRunInTerminalRequest":true,"locale":"en-us","supportsProgressReporting":true,"supportsInvalidatedEvent":true,"supportsMemoryReferences":true},"type":"request","seq":1}"#;
        let message = serde_json::from_str::<ProtocolMessage>(json);
        println!("{:?}", message);
    }
}
