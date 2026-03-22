# Hurlbox

> A web-based single-binary [hurl](https://hurl.dev) tests runner: run tests individually, see responses in real time.

<p align="center">
  <img src="./assets/hurlbox-dark.png" alt="Hurlbox Dark Theme" />
</p>

I love Hurl and its file based testing mechanisms, but I don't like that it doesn't have a proper way to run tests individually and see responses in real time. This is my attempt to build a web based Hurl runner that is a single Rust binary which uses the [hurl_core](https://crates.io/crates/hurl_core) crate (so having Hurl installed is not a requirement) and can also run as a Docker service.


## Features

- **Monaco Editor** with Hurl syntax highlighting and code lens
- **File Explorer** with create, read, write operations
- **Run & Test** individual entries or entire files
- **Build Assertions** - auto-generate JSONPath assertions from responses
- **Environment Variables** - load `.env` files for request substitution
- **Response Viewer** with JSON formatting
- **Request/Headers/Response** tabs for detailed inspection

## Installation

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jossephus/hurlbox/releases/latest/download/hurlbox-installer.sh | sh
```

## Quick Start

### Start the Server

```bash
hurlbox
```

The server runs on `http://localhost:3030` by default.

## Docker image (Docker Hub)

The public image is available on Docker Hub:

- `jossephus/hurlbox:latest`

Pull it directly:

```bash
docker pull jossephus/hurlbox:latest
```

Run it locally:

```bash
docker run --rm -p 3030:3030 \
  -v "$(pwd):/workspace" \
  jossephus/hurlbox:latest \
  --dir /workspace
```

You can checkout the setup at @examples/docker-compose.yml to see an example.

### CLI Options

```bash
# Specify server directory
hurlbox --dir /path/to/hurl/files

# Custom port
hurlbox --port 8080

# Load environment file
hurlbox --env-file .env
```

### Frontend Development

```bash
cd web
pnpm install
pnpm dev
```

## Desktop App (Work in Progress)

A native desktop application is in development using [GPUI](https://github.com/zed-industries/gpui), the GPU-accelerated UI framework from Zed Industries.

## Preview

<p align="center">
  <img src="./assets/hurlbox-light.png" alt="Hurlbox Light Theme" />
</p>

## License

MIT
