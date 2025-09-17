# LRCGET CLI

A powerful command-line utility for mass-downloading synchronized and plain lyrics for your offline music library. Built with Rust for performance and reliability.

## 🎵 Overview

LRCGET CLI integrates with the LRCLIB API to automatically fetch lyrics for your music collection. It features intelligent caching, fuzzy search, parallel processing, and real-time file monitoring to keep your library synchronized with the latest lyrics.

### Key Features

- **🚀 High Performance**: Parallel downloads with configurable concurrency
- **🎯 Smart Search**: Fuzzy matching and intelligent fallback algorithms
- **💾 Hybrid Caching**: Redis + file cache with automatic fallback
- **📁 Real-time Monitoring**: Watch directories for new files automatically
- **🐳 Docker Ready**: Production-ready containerization with volume persistence
- **🔧 Flexible Configuration**: TOML files + environment variables
- **📊 Rich UI**: Terminal interface with progress tracking or structured logging

## 📋 Table of Contents

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Commands](#-commands)
- [Configuration](#-configuration)
- [Docker Deployment](#-docker-deployment)
- [Advanced Features](#-advanced-features)
- [Examples](#-examples)
- [Troubleshooting](#-troubleshooting)

## 🚀 Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/musicdock/lrcget
cd lrcget-cli

# Build release binary
cargo build --release

# The binary will be available at target/release/lrcget
```

### System Requirements

- **Rust**: 1.80+ (for building from source)
- **Operating System**: Linux, macOS, Windows
- **Audio Files**: MP3, M4A, FLAC, OGG, Opus, WAV
- **Optional**: Redis (for enhanced caching), FFprobe/MediaInfo (for duration extraction)

## ⚡ Quick Start

```bash
# Initialize a music library
lrcget init /path/to/your/music

# Scan for music files
lrcget scan

# Download missing lyrics with parallel processing
lrcget download --missing-lyrics --parallel 8

# Search for specific lyrics
lrcget search "Bohemian Rhapsody" --artist "Queen"

# Real-time monitoring for new files
lrcget watch /path/to/music --initial-scan
```

## 📖 Commands

### Library Management

#### `lrcget init <DIRECTORY>`
Initialize a new music library by configuring a directory to scan.

```bash
lrcget init ~/Music
lrcget init /mnt/music --force  # Force re-initialization
```

**Options:**
- `--force`: Force re-initialization even if directory is already configured

#### `lrcget scan [DIRECTORY]`
Scan music directories for tracks and add them to the database.

```bash
lrcget scan                    # Scan configured directories
lrcget scan ~/Music/NewAlbum   # Scan specific directory
lrcget scan --force            # Rescan all files (ignore existing entries)
```

**Options:**
- `--force`: Rescan all files, ignoring existing database entries

### Lyrics Operations

#### `lrcget download`
Download lyrics for tracks in your library with advanced filtering options.

```bash
# Basic usage
lrcget download --missing-lyrics

# Advanced filtering
lrcget download --artist "Queen" --album "A Night at the Opera"
lrcget download --track-id 123
lrcget download --missing-lyrics --parallel 16 --dry-run
```

**Options:**
- `--track-id <ID>`: Download lyrics for specific track ID
- `--missing-lyrics`: Only download for tracks missing lyrics
- `--artist <ARTIST>`: Filter by artist name
- `--album <ALBUM>`: Filter by album name
- `--parallel <N>`: Maximum parallel downloads (default: 4, max: 100)
- `--dry-run`: Preview operations without downloading
- `--force`: Re-download existing lyrics

#### `lrcget search <TITLE>`
Manually search for lyrics with advanced matching.

```bash
lrcget search "Bohemian Rhapsody" --artist "Queen"
lrcget search "Yesterday" --artist "Beatles" --duration 125
lrcget search "Song Title" --limit 10 --format detailed
```

**Options:**
- `--artist <ARTIST>`: Artist name for better matching
- `--album <ALBUM>`: Album name for context
- `--duration <SECONDS>`: Track duration in seconds (improves accuracy)
- `--limit <N>`: Maximum number of results (default: 5)
- `--format <FORMAT>`: Output format (`table`, `json`, `detailed`)
- `--synced-only`: Only show results with synchronized lyrics

#### `lrcget fetch <FILE>`
Fetch lyrics for a specific audio file.

```bash
lrcget fetch /path/to/song.mp3
lrcget fetch ~/Music/album/track.flac --dry-run
```

### Real-time Monitoring

#### `lrcget watch <DIRECTORY>`
Monitor directories for new audio files and automatically download lyrics.

```bash
# Basic watching
lrcget watch ~/Music --initial-scan

# Advanced configuration
lrcget watch /music --debounce-seconds 5 --batch-size 25
lrcget watch ~/Downloads --extensions mp3,flac --dry-run
```

**Options:**
- `--initial-scan`: Scan entire directory on startup before watching
- `--debounce-seconds <N>`: Wait time before processing detected files (default: 10)
- `--batch-size <N>`: Maximum files to process in one batch (default: 50)
- `--extensions <LIST>`: Comma-separated list of file extensions to watch
- `--dry-run`: Show what would be processed without downloading

### Configuration Management

#### `lrcget config`
Advanced configuration management with persistent settings.

```bash
# View current configuration
lrcget config show

# Modify settings
lrcget config set skip_tracks_with_synced_lyrics false
lrcget config set redis_url "redis://localhost:6379"

# Get specific values
lrcget config get lrclib_instance

# List all available keys with descriptions
lrcget config keys

# Reset to defaults
lrcget config reset

# Show config file location
lrcget config path
```

**Subcommands:**
- `show`: Display current configuration with values
- `set <KEY> <VALUE>`: Set a configuration value
- `get <KEY>`: Get a specific configuration value
- `keys`: List all available configuration keys with descriptions
- `path`: Show configuration file path
- `reset`: Reset configuration to defaults

### Data Operations

#### `lrcget export`
Export library data in various formats.

```bash
lrcget export --format json --output library.json
lrcget export --format csv --missing-only
```

#### `lrcget batch <FILE>`
Execute batch operations from a file.

```bash
lrcget batch operations.json --dry-run
lrcget batch download_list.yaml
```

#### `lrcget cache`
Manage cache operations and statistics.

```bash
lrcget cache stats    # Show cache statistics
lrcget cache clear    # Clear all caches
lrcget cache cleanup  # Remove expired entries
```

## ⚙️ Configuration

Configuration follows a hierarchy: **Environment Variables** > **TOML Config File** > **Defaults**

### Configuration File

Location: `~/.config/lrcget-cli/config.toml` (Linux/macOS) or `%APPDATA%\lrcget-cli\config.toml` (Windows)

```toml
# Core Settings
database_path = "~/.local/share/lrcget-cli/lrcget.db"
lrclib_instance = "https://lrclib.net"
skip_tracks_with_synced_lyrics = true
skip_tracks_with_plain_lyrics = false
try_embed_lyrics = false
show_line_count = true

# Performance & Caching
redis_url = "redis://localhost:6379"  # Optional: enables hybrid cache
lrclib_database_path = "/path/to/lrclib-db-dump.sqlite3"  # Optional: local database

# Watch Mode Settings
watch_debounce_seconds = 10
watch_batch_size = 50
```

### Environment Variables

All configuration options support environment variable overrides:

```bash
# Core configuration
export LRCGET_DATABASE_PATH="/custom/path/lrcget.db"
export LRCGET_LRCLIB_INSTANCE="https://lrclib.net"
export LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS=true
export LRCGET_SKIP_TRACKS_WITH_PLAIN_LYRICS=false
export LRCGET_TRY_EMBED_LYRICS=false
export LRCGET_SHOW_LINE_COUNT=true

# Performance settings
export LRCGET_REDIS_URL="redis://localhost:6379"
export LRCGET_LRCLIB_DATABASE_PATH="/path/to/lrclib-db-dump.sqlite3"

# Watch mode configuration
export LRCGET_WATCH_DEBOUNCE_SECONDS=10
export LRCGET_WATCH_BATCH_SIZE=50

# Logging
export RUST_LOG=debug  # Logging level (error, warn, info, debug, trace)

# UI mode override (for testing)
export LRCGET_FORCE_TERMINAL_UI=1  # Force terminal UI in all environments
```

## 🐳 Docker Deployment

### Quick Start with Docker

```bash
# Build the image
docker build -t lrcget-cli .

# Create persistent volume
docker volume create lrcget_data

# Run a command
docker run --rm \
  -v lrcget_data:/data \
  -v /path/to/music:/music:ro \
  -e LRCGET_DATABASE_PATH=/data/lrcget.db \
  lrcget-cli scan /music
```

### Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  lrcget:
    build: .
    volumes:
      - lrcget_data:/data
      - /path/to/your/music:/music:ro  # Mount your music library as read-only
    environment:
      - LRCGET_DATABASE_PATH=/data/lrcget.db
      - LRCGET_LRCLIB_DATABASE_PATH=/data/lrclib.db  # Optional
      - LRCGET_REDIS_URL=redis://redis:6379  # Optional Redis cache
      - RUST_LOG=info
    depends_on:
      - redis  # Optional

  redis:  # Optional caching service
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes

volumes:
  lrcget_data:
  redis_data:
```

### Docker Commands

```bash
# Basic operations
docker-compose run --rm lrcget scan /music
docker-compose run --rm lrcget download --missing-lyrics
docker-compose run --rm lrcget config show

# Continuous monitoring
docker-compose run --rm lrcget watch /music --initial-scan

# With custom settings
docker-compose run --rm lrcget watch /music \
  --debounce-seconds 5 \
  --batch-size 100 \
  --extensions mp3,flac
```

### Production Deployment

For production environments, consider:

```yaml
version: '3.8'

services:
  lrcget-watcher:
    image: lrcget-cli:latest
    volumes:
      - lrcget_data:/data
      - /mnt/music:/music:ro
    environment:
      - LRCGET_DATABASE_PATH=/data/lrcget.db
      - LRCGET_REDIS_URL=redis://redis:6379
      - LRCGET_WATCH_DEBOUNCE_SECONDS=5
      - LRCGET_WATCH_BATCH_SIZE=25
      - RUST_LOG=info
    command: watch /music --initial-scan
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '1.0'

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes
    restart: unless-stopped

volumes:
  lrcget_data:
  redis_data:
```

### Environment Variables for Docker

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `LRCGET_DATABASE_PATH` | Main application database path | `/data/lrcget.db` | `/data/lrcget.db` |
| `LRCGET_LRCLIB_DATABASE_PATH` | Local LRCLIB database (optional) | None | `/data/lrclib.db` |
| `LRCGET_REDIS_URL` | Redis cache URL (optional) | None | `redis://redis:6379` |
| `LRCGET_LRCLIB_INSTANCE` | LRCLIB API instance | `https://lrclib.net` | `https://lrclib.net` |
| `RUST_LOG` | Logging level | `info` | `debug` |
| `DOCKER` | Docker environment detection | None | `1` |

## 🔧 Advanced Features

### 🚀 Hybrid Cache System

LRCGET CLI features a sophisticated three-tier caching system:

- **Redis Cache**: Ultra-fast shared cache for multiple instances (7-day TTL)
- **File Cache**: Local JSON-based cache with LRU eviction (1-week retention)
- **Intelligent Fallback**: Automatically switches between Redis and file cache

```bash
# Enable Redis cache
export LRCGET_REDIS_URL="redis://localhost:6379"
lrcget download --artist "Queen"  # Uses Redis + file cache

# Without Redis (file cache only)
unset LRCGET_REDIS_URL
lrcget download --artist "Queen"  # Uses file cache only
```

**Performance Impact**: Cache hits reduce API calls by 80%+ and improve response times significantly.

### 🎯 Intelligent Search Engine

Advanced search capabilities with multiple matching strategies:

- **Exact Matching**: Direct title/artist/album matches with duration tolerance
- **Fuzzy Search**: Handles typos and variations using skim algorithm
- **Multiple Variations**: Generates search permutations for better results
- **Smart Ranking**: Prioritizes title > artist > album matches

```bash
# Fuzzy search finds matches even with typos
lrcget search "Bohemain Rhapody" --artist "Quen"  # Finds "Bohemian Rhapsody" by "Queen"

# Multiple search strategies
lrcget search "Yesterday" --artist "Beatles" --duration 125  # Duration improves accuracy
```

### 📊 Local Database Integration

Support for local LRCLIB database dumps for enhanced performance:

- **Offline Operation**: Search millions of lyrics without internet connectivity
- **Lightning Speed**: Local searches are 100x faster than API calls
- **Automatic Fallback**: Local DB → API → Cache update workflow
- **Sync Updates**: API results automatically update local database

```bash
# Using local database
export LRCGET_LRCLIB_DATABASE_PATH="/path/to/lrclib-db-dump.sqlite3"
lrcget search "Title" --artist "Artist"  # Searches local DB first, then API
```

### 📁 Real-time File Monitoring

Intelligent file system watching with debounced processing:

- **Event-driven**: Responds to file creation/modification events
- **Debounced Processing**: Configurable delay to batch multiple operations
- **Batch Optimization**: Processes multiple files efficiently
- **Session Statistics**: Tracks activity and success rates

```bash
# Start watching with initial scan
lrcget watch ~/Music --initial-scan --debounce-seconds 5

# Monitor specific extensions
lrcget watch /mnt/music --extensions mp3,flac,m4a --batch-size 25
```

### 🔌 Extensibility Features

- **Hook System**: Pre/post download script execution (planned)
- **Template Engine**: Customizable output formatting
- **Export/Import**: Library backup and migration tools
- **Batch Operations**: File-based mass operations

## 💡 Examples

### Basic Workflow

```bash
# 1. Setup and initialization
lrcget init ~/Music
lrcget config set skip_tracks_with_synced_lyrics false
lrcget config set redis_url "redis://localhost:6379"

# 2. Scan and analyze library
lrcget scan --force  # Full rescan
lrcget export --format json --output library-backup.json

# 3. Download lyrics with different strategies
lrcget download --missing-lyrics --parallel 8
lrcget download --artist "Pink Floyd" --dry-run
lrcget download --album "The Dark Side of the Moon"

# 4. Monitor for new files
lrcget watch ~/Downloads/Music --initial-scan
```

### Advanced Use Cases

```bash
# High-performance batch processing
lrcget download --missing-lyrics --parallel 16 --format json

# Targeted artist/album processing
lrcget download --artist "Led Zeppelin" --album "IV" --force

# Testing and validation
lrcget download --track-id 123 --dry-run --format detailed
lrcget search "Stairway to Heaven" --artist "Led Zeppelin" --limit 1

# Configuration management
lrcget config keys  # List all available options
lrcget config set try_embed_lyrics true  # Enable metadata embedding
lrcget config reset  # Reset to defaults

# Cache management
lrcget cache stats  # View cache performance
lrcget cache clear  # Clear all caches for fresh start
```

### Integration Examples

```bash
# Plex/Jellyfin integration (with metadata embedding)
lrcget config set try_embed_lyrics true
lrcget watch /mnt/plex/music --initial-scan

# Automated daily downloads
0 2 * * * /usr/local/bin/lrcget download --missing-lyrics --parallel 4

# Bulk library migration
lrcget export --format json --output old-library.json
lrcget batch old-library.json --dry-run  # Validate
lrcget batch old-library.json  # Execute
```

## 🐛 Troubleshooting

### Common Issues

#### Permission Errors
```bash
# Check file permissions
ls -la ~/.local/share/lrcget-cli/

# Fix database permissions
chmod 664 ~/.local/share/lrcget-cli/lrcget.db

# Docker permission issues
docker run --rm -u root -v lrcget_data:/data alpine chown -R 1000:1000 /data
```

#### Network Issues
```bash
# Test API connectivity
curl -s "https://lrclib.net/api/search?q=test" | jq .

# Check cache configuration
lrcget cache stats
lrcget config get redis_url

# Bypass cache for testing
lrcget search "test" --format json  # Fresh API call
```

#### Performance Issues
```bash
# Enable debug logging
export RUST_LOG=debug
lrcget download --missing-lyrics --parallel 2

# Check database size and performance
du -h ~/.local/share/lrcget-cli/lrcget.db
lrcget export --format json | wc -l  # Count tracks
```

### Docker Troubleshooting

```bash
# Check container logs
docker-compose logs lrcget

# Verify environment variables
docker-compose run --rm lrcget env | grep LRCGET

# Test configuration
docker-compose run --rm lrcget config show

# Check volume mounts
docker run --rm -v lrcget_data:/data alpine ls -la /data
```

### Debug Commands

```bash
# Verbose logging
RUST_LOG=debug lrcget scan ~/Music

# Test specific functionality
lrcget search "test" --format detailed --limit 1
lrcget fetch /path/to/test.mp3 --dry-run

# Validate configuration
lrcget config show
lrcget config keys | grep redis
```

## 📚 Additional Resources

### Configuration Reference

For a complete list of configuration options:
```bash
lrcget config keys  # Shows all available configuration keys with descriptions
```

### Supported Audio Formats

- **Lossless**: FLAC, WAV
- **Lossy Compressed**: MP3, M4A (AAC), OGG Vorbis, Opus
- **Metadata Support**: ID3v1/v2 (MP3), MP4 tags (M4A), Vorbis Comments (OGG/FLAC)

### Performance Characteristics

- **Concurrency**: 1-100 parallel downloads (default: 4)
- **Memory Usage**: ~10-50MB typical usage
- **Database**: Handles 100k+ tracks efficiently
- **Cache**: 80%+ hit rate reduces API load significantly

### Contributing

This project welcomes contributions! Areas for improvement:
- Additional audio format support
- Enhanced fuzzy matching algorithms
- Web interface development
- Plugin system expansion

## 📄 License

This project is licensed under the MIT License. See the LICENSE file for details.

## 🙏 Acknowledgments

- [LRCLIB](https://lrclib.net) - The excellent lyrics database API
- [LRCGET](https://github.com/tranxuanthang/lrcget) - Original GUI project 
- [Lofty](https://github.com/Serial-ATA/lofty) - Audio metadata extraction
- [Ratatui](https://ratatui.rs) - Terminal user interface
