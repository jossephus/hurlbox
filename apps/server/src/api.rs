use crate::execution;
use crate::models::*;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

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
            match execution::run_entry(&request.content, request.entry_index, request.env) {
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
            match execution::run_to_end(&request.content, request.entry_index, request.env) {
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
            match execution::run_from_begin(&request.content, request.entry_index, request.env) {
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
            match execution::run_selection(&request.content, request.selection, request.env) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            }
        })
}

pub fn run_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("run-file"))
        .and(warp::body::json())
        .map(
            |request: RunFileRequest| match execution::run_file(&request.content, request.env) {
                Ok(results) => ok_json(&RunFileResponse { results }).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
}

pub fn test_file_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("test-file"))
        .and(warp::body::json())
        .map(
            |request: RunFileRequest| match execution::test_file(&request.content, request.env) {
                Ok(result) => ok_json(&result).into_response(),
                Err(error) => err_json(StatusCode::BAD_REQUEST, error).into_response(),
            },
        )
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
