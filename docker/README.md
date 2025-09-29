# LRCGET CLI - Docker Deployment Guide

This directory contains Docker configurations for deploying LRCGET CLI in various environments. All configurations use the official pre-built image `diegoninja/lrcget-cli:latest` with Redis caching support.

## 📁 Available Configurations

### `docker-compose.yml`
**Basic development/testing setup**
- Single LRCGET CLI service with Redis cache
- Manual command execution
- Suitable for testing and development
- Resource limits appropriate for development machines

### `docker-compose.production.yml`
**Production deployment with continuous monitoring**
- Automatic continuous lyrics monitoring service
- Separate CLI service for manual operations
- Production-optimized Redis configuration
- Enhanced health checks and resource limits
- Structured logging and monitoring


## 🚀 Quick Start

### Option 1: Docker Compose (Basic)
```bash
# Clone or download the docker-compose.yml
cd docker/

# Update music path in docker-compose.yml
# Edit the volume mount: /path/to/your/music:/music:ro

# Start services
docker-compose up -d

# Initialize library
docker-compose exec lrcget lrcget init /music

# Scan for music
docker-compose exec lrcget lrcget scan

# Download missing lyrics
docker-compose exec lrcget lrcget download --missing-lyrics --parallel 4
```

### Option 2: Portainer (Recommended)
1. Open Portainer web interface
2. Navigate to **Stacks** → **Add stack**
3. Choose **Web editor**
4. Copy contents of `docker-compose.yml` (works perfectly in Portainer)
5. Update the music path in the configuration
6. Deploy the stack
7. Access container console to run initial setup commands

### Option 3: Production Deployment
```bash
# For continuous monitoring
docker-compose -f docker-compose.production.yml up -d

# Or deploy docker-compose.production.yml in Portainer
```

## ⚙️ Configuration

### Required: Music Library Path
Update the music volume mount in your chosen configuration:

```yaml
volumes:
  # IMPORTANT: Write access needed to save .lrc/.txt lyrics files alongside music
  - /your/music/path:/music  # ← Update this path (NO :ro suffix)
```

**Example paths:**
- Linux: `/home/user/Music:/music`
- NAS: `/mnt/nas/music:/music`
- External drive: `/media/music:/music`
- Windows: `C:\Music:/music`

**⚠️ Important**: Do NOT use `:ro` (read-only) suffix as LRCGET needs to write lyrics files alongside your music files.

### Optional: LRCLIB Database Dump
For faster offline searches, mount a local LRCLIB database:

```yaml
volumes:
  - /path/to/lrclib-db-dump.sqlite3:/data/lrclib.db:ro
```

Download from: https://lrclib.net/db-dumps

### Environment Variables
Key configuration options available in all setups:

| Variable | Default | Description |
|----------|---------|-------------|
| `LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS` | `true` | Skip tracks with existing synced lyrics |
| `LRCGET_TRY_EMBED_LYRICS` | `false` | Embed lyrics in audio metadata |
| `LRCGET_WATCH_DEBOUNCE_SECONDS` | `10` | Watch mode delay before processing |
| `LRCGET_WATCH_BATCH_SIZE` | `50` | Max files per batch in watch mode |
| `RUST_LOG` | `info` | Logging level (error/warn/info/debug) |

## 📁 How Lyrics Are Stored

LRCGET CLI saves lyrics files directly alongside your music files:

```
/your/music/
├── Artist/
│   ├── Album/
│   │   ├── track.mp3
│   │   ├── track.lrc          ← Synchronized lyrics
│   │   └── track.txt          ← Plain text lyrics (if no synced available)
```

**File Types:**
- **`.lrc`**: Synchronized lyrics with timestamps `[00:12.34]`
- **`.txt`**: Plain text lyrics without timestamps
- **Instrumental tracks**: Creates `.lrc` with `[au: instrumental]`

**Behavior:**
- Synced lyrics (`.lrc`) take priority over plain text (`.txt`)
- When synced lyrics are found, any existing `.txt` file is removed
- Media servers like Plex/Jellyfin can automatically detect these files

## 🔧 Common Operations

### Initial Setup
```bash
# Initialize library (required first step)
docker-compose exec lrcget lrcget init /music

# Scan for all music files
docker-compose exec lrcget lrcget scan
```

### Download Lyrics
```bash
# Download missing lyrics
docker-compose exec lrcget lrcget download --missing-lyrics --parallel 4

# Download for specific artist
docker-compose exec lrcget lrcget download --artist "Queen"

# Download for specific album
docker-compose exec lrcget lrcget download --artist "Queen" --album "A Night at the Opera"
```

### Search and Testing
```bash
# Search for lyrics
docker-compose exec lrcget lrcget search "Bohemian Rhapsody" --artist "Queen"

# Test specific file
docker-compose exec lrcget lrcget fetch /music/path/to/song.mp3

# Dry run (preview operations)
docker-compose exec lrcget lrcget download --missing-lyrics --dry-run
```

### Monitoring
```bash
# Continuous monitoring (basic setup)
docker-compose exec lrcget lrcget watch /music --initial-scan

# View configuration
docker-compose exec lrcget lrcget config show

# Cache statistics
docker-compose exec lrcget lrcget cache stats
```

### Maintenance
```bash
# Clear cache
docker-compose exec lrcget lrcget cache clear

# Export library data
docker-compose exec lrcget lrcget export --format json --output /data/backup.json

# View logs
docker-compose logs lrcget
docker-compose logs redis
```

## 📊 Production Considerations

### Resource Requirements
- **Basic setup**: 2 CPU cores, 1GB RAM
- **Production**: 4 CPU cores, 2GB RAM
- **Storage**: 100MB for application data, 500MB for Redis cache

### Performance Optimization
1. **Enable Redis caching** (included in all configurations)
2. **Use LRCLIB database dump** for offline searches
3. **Adjust parallel downloads** based on system capacity
4. **Enable metadata embedding** for media servers

### Security
- Music directories mounted as read-only
- Redis not exposed to host network
- Non-root container execution
- Resource limits prevent exhaustion

### Monitoring
- Health checks for all services
- Structured logging with rotation
- Resource usage monitoring via Portainer/Docker stats
- Cache performance metrics via `lrcget cache stats`

## 🐛 Troubleshooting

### Permission Issues
```bash
# Check volume permissions
docker-compose exec lrcget ls -la /music
docker-compose exec lrcget ls -la /data

# Fix data directory permissions
docker run --rm -v lrcget_data:/data alpine chown -R 1000:1000 /data
```

### Network Issues
```bash
# Test Redis connection
docker-compose exec lrcget redis-cli -h redis ping

# Test LRCLIB API
docker-compose exec lrcget curl -s "https://lrclib.net/api/search?q=test"
```

### Performance Issues
```bash
# Enable debug logging
# Update docker-compose.yml: RUST_LOG=debug

# Monitor resource usage
docker stats

# Check Redis memory
docker-compose exec redis redis-cli info memory
```

### Configuration Validation
```bash
# Verify environment variables
docker-compose exec lrcget env | grep LRCGET

# Test configuration
docker-compose exec lrcget lrcget config show

# Validate music mount
docker-compose exec lrcget find /music -name "*.mp3" | head -5
```

## 🔄 Updates

To update to the latest LRCGET CLI version:

```bash
# Pull latest image
docker-compose pull

# Restart services
docker-compose down && docker-compose up -d

# Or in Portainer: Stack actions → Pull and redeploy
```

## 📚 Additional Resources

- [LRCGET CLI Documentation](../README.md)
- [Docker Hub Repository](https://hub.docker.com/r/diegoninja/lrcget-cli)
- [LRCLIB API Documentation](https://lrclib.net/docs)
- [Issue Reporting](https://github.com/musicdock/lrcget/issues)