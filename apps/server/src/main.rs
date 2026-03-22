mod api;
mod execution;
mod filescanner;

use clap::Parser;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::path::Path;
use warp::Filter;
use warp::Reply;

#[derive(Parser)]
#[command(name = "hurlbox-server", version, about = "Hurlbox API server")]
struct Cli {
    /// Path to an env file to load default variables
    #[arg(short, long = "env-file")]
    env_file: Option<String>,

    /// Root directory to scan for .hurl files (default: current directory)
    #[arg(short, long)]
    dir: Option<String>,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3030)]
    port: u16,
}

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

fn load_env_file(path: impl AsRef<Path>) -> Result<HashMap<String, String>, String> {
    let path = path.as_ref();
    let vars: HashMap<String, String> = dotenvy::from_path_iter(path)
        .map_err(|e| format!("Failed to read env file {}: {}", path.display(), e))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| format!("Failed to parse env file {}: {}", path.display(), e))?;
    Ok(vars)
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let root_dir = cli.dir.unwrap_or_else(|| ".".to_string());
    api::set_root_dir(root_dir.clone());

    let default_env: HashMap<String, String> = if let Some(path) = cli.env_file.as_ref() {
        match load_env_file(path) {
            Ok(values) => values,
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(2);
            }
        }
    } else {
        HashMap::new()
    };
    api::set_default_env(default_env, cli.env_file.clone());

    // API routes
    let api_routes = warp::path("api").and(
        api::parse_route()
            .or(api::run_entry_route())
            .or(api::run_to_end_route())
            .or(api::run_from_begin_route())
            .or(api::run_selection_route())
            .or(api::run_file_route())
            .or(api::test_file_route())
            .or(api::build_assertions_route())
            .or(api::rerun_last_route())
            .or(api::files_route())
            .or(api::read_file_route())
            .or(api::create_file_route())
            .or(api::update_file_route())
            .or(api::env_default_route()),
    );

    let asset_routes =
        warp::path("assets")
            .and(warp::path::full())
            .map(|path: warp::path::FullPath| {
                let path = path.as_str().strip_prefix("/").unwrap_or("");
                match get_embedded_file(path) {
                    Some((data, mime)) => {
                        warp::reply::with_header(data, "content-type", mime).into_response()
                    }
                    None => warp::http::StatusCode::NOT_FOUND.into_response(),
                }
            });

    let favicon_route = warp::path("favicon.ico").map(|| match get_embedded_file("favicon.ico") {
        Some((data, mime)) => warp::reply::with_header(data, "content-type", mime).into_response(),
        None => warp::http::StatusCode::NOT_FOUND.into_response(),
    });

    let index_route = warp::path::end().map(|| match get_embedded_file("index.html") {
        Some((data, _)) => {
            warp::reply::with_header(data, "content-type", "text/html").into_response()
        }
        None => warp::http::StatusCode::NOT_FOUND.into_response(),
    });

    let routes = api_routes
        .or(asset_routes)
        .or(favicon_route)
        .or(index_route)
        .with(warp::cors().allow_any_origin());

    let host: std::net::IpAddr = cli.host.parse().expect("Invalid host address");
    println!("Hurlbox server starting on http://{}:{}", host, cli.port);
    warp::serve(routes).run((host, cli.port)).await;
}
