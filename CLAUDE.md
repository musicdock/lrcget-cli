# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LRCGET CLI is a Rust-based command-line utility for mass-downloading synchronized and plain lyrics for offline music libraries. It integrates with the LRCLIB API and features intelligent caching, fuzzy search, parallel processing, and real-time file monitoring.

## Development Commands

### Build and Run
```bash
# Build in development mode
cargo build

# Build optimized release binary
cargo build --release

# Run with arguments (development)
cargo run -- <command> [args]

# Run with arguments (release binary)
./target/release/lrcget <command> [args]

# Run tests
cargo test
```

### Development Testing
```bash
# Test with local LRCLIB database dump
export LRCGET_LRCLIB_DATABASE_PATH="/path/to/lrclib-db-dump.sqlite3"

# Test search functionality
./target/debug/lrcget search "song title" --artist "artist name" --limit 5

# Test download with dry run
./target/debug/lrcget download --track-id 123 --parallel 1 --dry-run

# Test scanning
./target/debug/lrcget scan /path/to/music --verbose

# Force terminal UI for testing (bypasses Docker detection)
export LRCGET_FORCE_TERMINAL_UI=1
```

## Architecture

### Module Structure
- **cli/**: Command-line interface implementations for each subcommand
  - Each subcommand (init, scan, download, search, etc.) has its own module
  - All CLI modules follow the pattern: `Args` struct + `execute()` function

- **core/**: Core business logic
  - `database.rs`: SQLite database operations and schema
  - `lyrics.rs`: Lyrics fetching, processing, and file operations
  - `cache.rs`: Hybrid caching system (Redis + file cache)
  - `lrclib.rs`: LRCLIB API client and search logic
  - `lrclib_db.rs`: Local LRCLIB database integration
  - `scanner.rs`: Audio file scanning and metadata extraction

- **config/**: Configuration management
  - TOML config files + environment variable overrides
  - Hierarchical configuration system

- **ui/**: User interface systems
  - `terminal_ui.rs`: Rich terminal UI with progress bars (using ratatui)
  - `docker_ui.rs`: Simplified logging-based UI for Docker environments
  - Auto-detection based on environment (TTY, Docker, CI flags)

- **utils/**: Utility functions and logging setup

### Key Design Patterns

1. **Hybrid Caching**: Three-tier system (Redis → File Cache → API)
2. **UI Mode Detection**: Automatically switches between terminal UI and logging UI
3. **Parallel Processing**: Configurable concurrency with tokio and rayon
4. **Fuzzy Search**: Multiple search strategies with intelligent ranking
5. **File System Watching**: Event-driven with debounced batch processing

## Configuration

### Environment Variables (for testing)
```bash
# Database paths
export LRCGET_DATABASE_PATH="/custom/path/lrcget.db"
export LRCGET_LRCLIB_DATABASE_PATH="/path/to/lrclib-dump.sqlite3"

# Performance settings
export LRCGET_REDIS_URL="redis://localhost:6379"

# Behavior flags
export LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS=false
export LRCGET_FORCE_TERMINAL_UI=1

# Logging
export RUST_LOG=debug
```

### Important Environment Variables
- `LRCGET_LRCLIB_DATABASE_PATH`: Path to local LRCLIB database dump for offline searches
- `LRCGET_FORCE_TERMINAL_UI`: Force terminal UI mode (useful for development)
- `DOCKER`: Auto-detected, switches to logging-based UI
- `CI`: Auto-detected, disables interactive UI elements

## Testing Workflows

### Basic Development Cycle
1. Use debug build for development: `cargo build`
2. Test with sample music files in `/home/diego/Music/`
3. Use environment variables to override config during testing
4. Test both with and without local LRCLIB database
5. Verify Redis caching when available

### Common Test Commands
```bash
# Initialize and scan test directory
./target/debug/lrcget init "/home/diego/Music/Who Let The Dogs Out"
./target/debug/lrcget scan --verbose

# Test search with database
LRCGET_LRCLIB_DATABASE_PATH="data/lrclib/lrclib-db-dump.sqlite3" ./target/debug/lrcget search "Big Dick Energy" --artist "Lambrini Girls" --format detailed

# Test download with Redis cache
LRCGET_REDIS_URL="redis://127.0.0.1:6379" ./target/debug/lrcget download --track-id 123 --parallel 1

# Test watch mode
./target/debug/lrcget watch /path/to/music --initial-scan --dry-run
```

## Docker Development

The application automatically detects Docker environment and switches UI modes. Use docker-compose.yml for container testing:

```bash
# Build and test
docker-compose build
docker-compose run --rm lrcget config show
docker-compose run --rm lrcget scan /music
```