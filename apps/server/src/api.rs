use crate::execution;
use crate::filescanner;
use std::collections::HashMap;
use std::sync::OnceLock;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

pub mod models {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    // Request models
    #[derive(Deserialize)]
    pub struct ParseRequest {
        pub content: String,
    }

    #[derive(Deserialize)]
    pub struct RunEntryRequest {
        pub content: String,
        pub entry_index: usize,
        pub env: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    pub struct RunToEndRequest {
        pub content: String,
        pub entry_index: usize,
        pub env: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    pub struct RunFromBeginRequest {
        pub content: String,
        pub entry_index: usize,
        pub env: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    pub struct RunSelectionRequest {
        pub content: String,
        pub selection: SelectionRange,
        pub env: Option<HashMap<String, String>>,
    }

    #[derive(Clone, Deserialize)]
    pub struct SelectionRange {
        pub start_line: u32,
        pub end_line: u32,
    }

    #[derive(Deserialize)]
    pub struct RunFileRequest {
        pub content: String,
        pub env: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    pub struct BuildAssertionsRequest {
        pub content: String,
        pub entry_index: usize,
        pub env: Option<HashMap<String, String>>,
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
        pub request: RequestInfo,
        pub status: u16,
        pub headers: HashMap<String, String>,
        pub body: String,
        pub timing: Option<TimingInfo>,
        pub assertions: Vec<AssertionResult>,
        pub error: Option<String>,
    }

    #[derive(Serialize)]
    pub struct RequestInfo {
        pub method: String,
        pub url: String,
        pub headers: HashMap<String, String>,
        pub body: Option<String>,
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
    pub struct BuildAssertionsResponse {
        pub content: String,
        pub assertions_added: usize,
    }

    #[derive(Serialize)]
    pub struct ErrorResponse {
        pub error: String,
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

    #[derive(Deserialize)]
    pub struct UpdateFileRequest {
        pub path: String,
        pub content: String,
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
}

use models::*;

static DEFAULT_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();
static DEFAULT_ENV_SOURCE: OnceLock<String> = OnceLock::new();
static ROOT_DIR: OnceLock<String> = OnceLock::new();

pub fn set_root_dir(dir: String) {
    let _ = ROOT_DIR.set(dir);
}

pub fn set_default_env(vars: HashMap<String, String>, source: Option<String>) {
    let _ = DEFAULT_ENV.set(vars);
    if let Some(source) = source {
        let _ = DEFAULT_ENV_SOURCE.set(source);
    }
}

fn resolve_path(path: &str) -> String {
    let root = ROOT_DIR.get().map(|s| s.as_str()).unwrap_or(".");
    let path = path.trim();
    if path.is_empty() || path == "." {
        root.to_string()
    } else {
        format!(
            "{}/{}",
            root.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[derive(serde::Serialize)]
struct EnvDefaultResponse {
    loaded: bool,
    source: Option<String>,
    count: usize,
}

fn merged_env(request_env: Option<HashMap<String, String>>) -> Option<HashMap<String, String>> {
    let defaults = DEFAULT_ENV.get().cloned().unwrap_or_default();
    if defaults.is_empty() {
        return request_env;
    }
    let mut merged = defaults;
    if let Some(overrides) = request_env {
        for (key, value) in overrides {
            merged.insert(key, value);
        }
    }
    Some(merged)
}

fn ok_json<T: serde::Serialize>(value: &T) -> impl Reply {
    warp::reply::with_status(warp::reply::json(value), StatusCode::OK)
}

fn err_json(status: StatusCode, error: String) -> impl Reply {
    warp::reply::with_status(warp::reply::json(&ErrorResponse { error }), status)
}

pub fn parse_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("parse"))
        .and(warp::body::json())
        .map(
            |request: ParseRequest| match execution::parse_hurl_entries(&request.content) {
                Ok(entries) => ok_json(&ParseResponse { entries }).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
}

pub fn run_entry_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-entry"))
        .and(warp::body::json())
        .map(|request: RunEntryRequest| {
            match execution::run_entry(
                &request.content,
                request.entry_index,
                merged_env(request.env),
            ) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn run_to_end_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-to-end"))
        .and(warp::body::json())
        .map(|request: RunToEndRequest| {
            match execution::run_to_end(
                &request.content,
                request.entry_index,
                merged_env(request.env),
            ) {
                Ok(results) => ok_json(&RunFileResponse { results }).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn run_from_begin_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-from-begin"))
        .and(warp::body::json())
        .map(|request: RunFromBeginRequest| {
            match execution::run_from_begin(
                &request.content,
                request.entry_index,
                merged_env(request.env),
            ) {
                Ok(results) => ok_json(&RunFileResponse { results }).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn run_selection_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-selection"))
        .and(warp::body::json())
        .map(|request: RunSelectionRequest| {
            match execution::run_selection(
                &request.content,
                request.selection,
                merged_env(request.env),
            ) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn run_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-file"))
        .and(warp::body::json())
        .map(|request: RunFileRequest| {
            match execution::run_file(&request.content, merged_env(request.env)) {
                Ok(results) => ok_json(&RunFileResponse { results }).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn test_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("test-file"))
        .and(warp::body::json())
        .map(|request: RunFileRequest| {
            match execution::test_file(&request.content, merged_env(request.env)) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn build_assertions_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("build-assertions"))
        .and(warp::body::json())
        .map(|request: BuildAssertionsRequest| {
            match execution::build_assertions_for_entry(
                &request.content,
                request.entry_index,
                merged_env(request.env),
            ) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn rerun_last_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("rerun-last"))
        .map(|| match execution::rerun_last() {
            Ok(result) => ok_json(&result).into_response(),
            Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
        })
}

pub fn files_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("files"))
        .and(warp::query::<FileQuery>())
        .map(|query: FileQuery| {
            let path = resolve_path(&query.path);
            match filescanner::scan_directory(&path) {
                Ok(tree) => ok_json(&tree).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn read_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("file"))
        .and(warp::query::<FileReadQuery>())
        .map(
            |query: FileReadQuery| match filescanner::read_file(&query.path) {
                Ok(content) => ok_json(&FileContentResponse {
                    content,
                    path: query.path,
                })
                .into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
}

pub fn create_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("file"))
        .and(warp::body::json())
        .map(|request: CreateFileRequest| {
            match filescanner::create_file(&request.path, request.content.as_deref()) {
                Ok(_) => ok_json(&CreateFileResponse {
                    success: true,
                    path: request.path,
                })
                .into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn update_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::put()
        .and(warp::path("file"))
        .and(warp::body::json())
        .map(|request: UpdateFileRequest| {
            match filescanner::write_file(&request.path, &request.content) {
                Ok(content) => ok_json(&FileContentResponse {
                    content,
                    path: request.path,
                })
                .into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn env_default_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get().and(warp::path("env-default")).map(|| {
        let count = DEFAULT_ENV.get().map(|m| m.len()).unwrap_or(0);
        let source = DEFAULT_ENV_SOURCE.get().cloned();
        ok_json(&EnvDefaultResponse {
            loaded: source.is_some(),
            source,
            count,
        })
        .into_response()
    })
}
