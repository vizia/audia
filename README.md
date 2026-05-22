# Audia

Audia is a desktop Spotify client written in Rust, built with [Vizia](https://github.com/vizia/vizia) for the UI and [librespot](https://github.com/librespot-org/librespot) for local playback.



> Spotify Premium is required.

> This project is still in early development and is likely to contain bugs and rough edges.

![Audia screenshot](Screenshot2.png)

## Highlights

- Rust-native desktop app with a custom Vizia interface
- Spotify OAuth login flow with persisted credentials
- Search across tracks, artists, and albums
- Playlist browsing and track filtering
- Create, rename, and delete playlists
- Add and remove tracks in playlists
- Local playback controls (play, pause, seek, volume)
- Queue and recently played side panel
- Automatic token refresh for long-running sessions
- Persistent UI and playback preferences

## Quick Start

### 1. Prerequisites

- Rust toolchain (stable)
- Spotify Premium account

### 2. Build & Run

```bash
cargo run --release
```

On first launch, use the login dialog to authenticate with Spotify.

## Current Status

Audia is actively evolving. Core flows (login, browsing, search, playback, queue, and playlist operations) are in place, but the app is not yet production-hardened.

If you hit a bug, opening an issue with steps to reproduce is very helpful.

## License

See [LICENSE](LICENSE).

