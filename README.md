# RinRuDesktop

A desktop application for browsing and installing games from the [cs.rin.ru forum](https://cs.rin.ru/forum/viewforum.php?f=10).

## Overview

This Tauri-based desktop app provides a modern interface for discovering, browsing, and managing games from the Russian programming forum cs.rin.ru. The application:

- **Scrapes and parses** the cs.rin.ru game forum section
- **Matches games** with external databases for enhanced metadata and images
- **Navigates through forum threads** to extract detailed post information
- **Identifies and extracts** release posts with multiple download links

## Features

### Game Feed
- Browse games from the cs.rin.ru forum in a clean, organized interface
- Refresh the feed to get the latest game listings
- Pagination support for browsing through multiple pages

### Game Details
- Click on any game to expand and view detailed information
- See all forum posts associated with each game
- Identify release posts with badges highlighting important updates

### Release Management
- Extract release information including version numbers and release dates
- View all download links grouped by file type
- Access multiple mirror links for flexible downloading

### External Database Integration
- Automatic matching with IGDB, Steam, and GOG databases
- Image caching for offline viewing
- Rich metadata enhancement from external sources

## Project Structure

```
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Application entry point
│   │   ├── lib.rs           # Library root with tests
│   │   ├── models.rs        # Data models (Game, Release, etc.)
│   │   ├── scraper.rs       # Forum scraping logic
│   │   ├── parser.rs        # Content parsing and extraction
│   │   ├── database.rs      # External database integration
│   │   └── commands.rs      # Tauri commands for frontend bridge
│   ├── Cargo.toml           # Rust dependencies
│   ├── tauri.conf.json      # Tauri configuration
│   └── build.rs             # Build script
├── frontend/
│   ├── index.html           # Main HTML page
│   ├── css/
│   │   └── styles.css       # Application styles
│   └── js/
│       └── main.js          # Frontend application logic
└── README.md                # This file
```

## Technology Stack

### Backend
- **Rust** with **Tauri** framework for cross-platform desktop support
- **reqwest** for HTTP requests to forum and database APIs
- **scraper** for HTML parsing of forum pages
- **serde** for serialization/deserialization
- **regex** for content pattern matching

### Frontend
- **Vanilla JavaScript** (ES modules) for clean, performant UI
- **CSS** with CSS custom properties for theming
- **Google Fonts** (Inter) for typography
- Tauri's webview API for native integration

## Getting Started

### Prerequisites
- Rust 1.70+ installed
- Node.js 16+ (for frontend development, optional)
- Webkit2gtk-4.0 and libgtk-3-dev (Linux)

### Building

```bash
# Build for development
cargo tauri dev

# Build for production
cargo tauri build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Architecture

The application follows a modular architecture:

1. **Scraper Layer**: Handles HTTP communication with cs.rin.ru and extracts game data from forum HTML
2. **Parser Layer**: Processes forum content to identify posts, releases, and download links
3. **Database Layer**: Integrates with external game databases for metadata enhancement
4. **Commands Layer**: Provides Tauri commands bridging frontend and backend
5. **Frontend Layer**: Reacts to user interactions and displays game information

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

This project is part of the CsRinRuWrapper initiative to create desktop tools for the Russian programming community.
