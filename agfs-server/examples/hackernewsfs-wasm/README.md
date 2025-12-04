# HackerNewsFS - WASM Plugin

A filesystem plugin that provides access to Hacker News stories as markdown files.

## Features

- Fetches top stories from Hacker News API
- Displays stories as readable markdown files
- Refresh capability to get latest stories
- Read-only filesystem

## Building

```bash
# Install WASM target (first time only)
make install

# Build the plugin
make build
```

This will generate `hackernewsfs-wasm.wasm` in the current directory.

## Usage

### Load the plugin with agfs-server

```bash
../../build/agfs-server --plugin ./hackernewsfs-wasm.wasm
```

### Available paths

- `cat /hackernews/refresh` - Refresh the story list from Hacker News
- `echo 1 > /hackernews/refresh` - Alternative way to refresh (any write triggers refresh)
- `ls /hackernews/frontpage/` - List all fetched stories (30 by default)
- `cat /hackernews/frontpage/1.md` - Read the top story
- `cat /hackernews/frontpage/2.md` - Read the 2nd story
- etc.

### Example session

```bash
# List stories
ls /hackernews/frontpage/

# Read the top story
cat /hackernews/frontpage/1.md

# Refresh stories
cat /hackernews/refresh

# Read updated list
ls /hackernews/frontpage/
```

## How it works

1. On initialization, the plugin fetches the top 30 story IDs from HN API
2. For each ID, it fetches the full story details
3. Stories are cached in memory
4. Reading `/hackernews/refresh` triggers a new fetch
5. Each story is formatted as a markdown file with:
   - Title
   - Author
   - Score
   - Number of comments
   - URL (if available)
   - Story text (if available)
   - Link to HN discussion

## Implementation

This plugin demonstrates:
- Using the HTTP client API from WASM
- Making multiple HTTP requests to external APIs
- JSON parsing with serde
- Caching data in plugin state
- Dynamic file generation

## API Used

Hacker News API: https://github.com/HackerNews/API

- `GET /v0/topstories.json` - Get list of top story IDs
- `GET /v0/item/{id}.json` - Get individual story details
