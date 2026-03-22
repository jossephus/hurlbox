# Hurlbox

> A web-based single-binary hurl tests runner [Hurl](https://hurl.dev/) — run tests individually, see responses in real time.

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
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aranshi/hurlbox/releases/latest/download/hurlbox-installer.sh | sh
```

### Windows

```powershell
powershell -c "irm https://github.com/aranshi/hurlbox/releases/latest/download/hurlbox-installer.ps1 | iex"
```

## Quick Start

### Start the Server

```bash
hurlbox
```

The server runs on `http://localhost:3030` by default.

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
