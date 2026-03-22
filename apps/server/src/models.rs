use serde::{Deserialize, Serialize};

// Request models
#[derive(Deserialize)]
pub struct ParseRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct RunEntryRequest {
    pub content: String,
    pub entry_index: usize,
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct RunToEndRequest {
    pub content: String,
    pub entry_index: usize,
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct RunFromBeginRequest {
    pub content: String,
    pub entry_index: usize,
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct RunSelectionRequest {
    pub content: String,
    pub selection: SelectionRange,
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Clone, Deserialize)]
pub struct SelectionRange {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Deserialize)]
pub struct RunFileRequest {
    pub content: String,
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct CancelRequest {
    pub run_id: String,
}

// Response models
#[derive(Serialize)]
pub struct ParseResponse {
    pub entries: Vec<EntryInfo>,
}

#[derive(Serialize)]
pub struct EntryInfo {
    pub index: usize,
    pub start_line: u32,
    pub end_line: u32,
    pub method: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct RunFileResponse {
    pub results: Vec<ExecutionResult>,
}

#[derive(Serialize)]
pub struct ExecutionResult {
    pub entry_index: usize,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub timing: Option<TimingInfo>,
    pub assertions: Vec<AssertionResult>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct TimingInfo {
    pub duration_ms: u64,
    pub connect_time_ms: Option<u64>,
    pub tls_time_ms: Option<u64>,
    pub transfer_time_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct AssertionResult {
    pub query: String,
    pub predicate: String,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}

#[derive(Serialize)]
pub struct TestFileResponse {
    pub overall_pass: bool,
    pub total_assertions: usize,
    pub passed_assertions: usize,
    pub failed_assertions: usize,
    pub results: Vec<ExecutionResult>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct CancelResponse {
    pub success: bool,
    pub message: String,
}

// File query models
#[derive(Deserialize)]
pub struct FileQuery {
    pub path: String,
}

#[derive(Deserialize)]
pub struct FileReadQuery {
    pub path: String,
}

#[derive(Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub content: Option<String>,
}

#[derive(Serialize)]
pub struct FileContentResponse {
    pub content: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct CreateFileResponse {
    pub success: bool,
    pub path: String,
}
