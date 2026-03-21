mod api;
mod envfile;
mod execution;
mod filescanner;
mod models;

use rust_embed::RustEmbed;
use std::collections::HashMap;
use warp::Filter;
use warp::Reply;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

fn get_embedded_file(path: &str) -> Option<(Vec<u8>, String)> {
    WebAssets::get(path).map(|content| {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();
        (content.data.into_owned(), mime)
    })
}

fn parse_cli_args() -> Result<Option<String>, String> {
    let mut args = std::env::args().skip(1);
    let mut env_file: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => {
                continue;
            }
            "--env-file" | "-e" => {
                let Some(path) = args.next() else {
                    return Err("Missing path for --env-file".to_string());
                };
                env_file = Some(path);
            }
            _ => {
                return Err(format!("Unknown argument: {}", arg));
            }
        }
    }

    Ok(env_file)
}

#[tokio::main]
async fn main() {
    let env_file = match parse_cli_args() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{}", error);
            eprintln!("Usage: hurlbox-server [--env-file <path>] [-e <path>]");
            std::process::exit(2);
        }
    };

    let default_env: HashMap<String, String> = if let Some(path) = env_file.as_ref() {
        match envfile::parse_env_file(path) {
            Ok(values) => {
                println!("Loaded {} env variable(s) from {}", values.len(), path);
                values
            }
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(2);
            }
        }
    } else {
        HashMap::new()
    };
    api::set_default_env(default_env, env_file.clone());

    // Shared state for managing running executions
    let _running_executions: Arc<Mutex<std::collections::HashMap<String, execution::ExecutionHandle>>> = 
        Arc::new(Mutex::new(std::collections::HashMap::new()));

    // API routes
    let api_routes = warp::path("api")
        .and(
            api::parse_route()
                .or(api::run_entry_route())
                .or(api::run_to_end_route())
                .or(api::run_from_begin_route())
                .or(api::run_selection_route())
                .or(api::run_file_route())
                .or(api::test_file_route())
                .or(api::rerun_last_route())
                .or(api::cancel_route())
                .or(api::files_route())
                .or(api::read_file_route())
                .or(api::create_file_route())
                .or(api::env_default_route())
        );

    // Serve embedded static files from web/dist/assets
    let asset_routes = warp::path("assets")
        .and(warp::path::full())
        .map(|path: warp::path::FullPath| {
            let path = path.as_str().strip_prefix("/").unwrap_or("");
            match get_embedded_file(path) {
                Some((data, mime)) => warp::reply::with_header(data, "content-type", mime).into_response(),
                None => warp::http::StatusCode::NOT_FOUND.into_response(),
            }
        });

    // Serve favicon
    let favicon_route = warp::path("favicon.ico")
        .map(|| {
            match get_embedded_file("favicon.ico") {
                Some((data, mime)) => warp::reply::with_header(data, "content-type", mime).into_response(),
                None => warp::http::StatusCode::NOT_FOUND.into_response(),
            }
        });

    // SPA fallback - serve index.html for root
    let index_route = warp::path::end()
        .map(|| {
            match get_embedded_file("index.html") {
                Some((data, _)) => warp::reply::with_header(data, "content-type", "text/html").into_response(),
                None => warp::http::StatusCode::NOT_FOUND.into_response(),
            }
        });

    // Combine all routes
    let routes = api_routes
        .or(asset_routes)
        .or(favicon_route)
        .or(index_route)
        .with(warp::cors().allow_any_origin());

    println!("Hurlbox server starting on http://localhost:3030");
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}
