# LiveCapTran

Live Korean speech caption/transcription overlay app.

## Project Structure

```
livecaptran/
├── assets/           # Icons and fonts
│   ├── NotoSansKR-Regular.ttf
│   ├── cog.png
│   └── close.png
├── src/              # Rust egui desktop app (frameless overlay window)
│   ├── main.rs
│   ├── app.rs
│   ├── audio.rs
│   └── settings.rs
├── Cargo.toml
└── .github/workflows/
    └── build.yml     # CI: builds for Linux, Windows, macOS
```

## Transcription API

- **Type**: OpenAI-compatible Whisper API (faster-whisper-server)
- Configured via `settings.yml` (defaults to OpenAI endpoints)

### Endpoints

#### GET /v1/models
Lists available Whisper models.

**Available models**: tiny, small, base, medium, large, large-v2, large-v3, distil-small.en, distil-medium.en, distil-large-v2, distil-large-v3

#### POST /v1/audio/transcriptions
Transcribes audio file to text.

**Request**: `multipart/form-data`
- `file`: audio file (WAV, etc.)
- `model`: model id (use `large-v3` for Korean)
- `language`: language code (`ko` for Korean)

**Response**: `{"text": "transcribed text"}`

## GUI Spec

- Frameless overlay window, screen-wide (left=0), height=500px
- Black semi-transparent background
- White text, 60pt, centered, auto-wrap
- Small X close button at top-right
- Draggable via mouse drag
- Captures live microphone audio, sends chunks to transcription API, displays results in real-time

## Build

```sh
cargo build --release
```

Binary name: `livecaptran`
