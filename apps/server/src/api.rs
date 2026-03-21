use crate::execution;
use crate::filescanner;
use crate::models::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

static DEFAULT_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();
static DEFAULT_ENV_SOURCE: OnceLock<String> = OnceLock::new();

pub fn set_default_env(vars: HashMap<String, String>, source: Option<String>) {
    let _ = DEFAULT_ENV.set(vars);
    if let Some(source) = source {
        let _ = DEFAULT_ENV_SOURCE.set(source);
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

pub fn rerun_last_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("rerun-last"))
        .map(|| match execution::rerun_last() {
            Ok(result) => ok_json(&result).into_response(),
            Err(error) => err_json(StatusCode::NOT_IMPLEMENTED, error).into_response(),
        })
}

pub fn cancel_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("cancel"))
        .and(warp::body::json())
        .map(
            |request: CancelRequest| match execution::cancel_execution(&request.run_id) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
}

pub fn files_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("files"))
        .and(warp::query::<FileQuery>())
        .map(
            |query: FileQuery| match filescanner::scan_directory(&query.path) {
                Ok(tree) => ok_json(&tree).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
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
                Ok(path) => ok_json(&CreateFileResponse {
                    success: true,
                    path,
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
