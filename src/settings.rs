use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SILENCE_CHUNKS_TO_END: usize = 10; // ~500ms at 50ms polling
pub const MAX_PHRASE_SECS: usize = 30;

// Whisper transcription source languages
pub const SOURCE_LANGUAGES: &[(&str, &str)] = &[
    ("ko", "Korean"),
    ("en", "English"),
    ("ja", "Japanese"),
    ("zh", "Chinese"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("ru", "Russian"),
    ("pt", "Portuguese"),
    ("vi", "Vietnamese"),
];

// Translation target languages (LLM-based, supports more)
pub const TARGET_LANGUAGES: &[(&str, &str)] = &[
    ("ko", "Korean"),
    ("en", "English"),
    ("ja", "Japanese"),
    ("zh", "Chinese"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("ru", "Russian"),
    ("pt", "Portuguese"),
    ("vi", "Vietnamese"),
    ("ar", "Arabic"),
    ("hi", "Hindi"),
    ("th", "Thai"),
    ("id", "Indonesian"),
    ("ms", "Malay"),
    ("it", "Italian"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("sv", "Swedish"),
    ("da", "Danish"),
    ("no", "Norwegian"),
    ("fi", "Finnish"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("cs", "Czech"),
    ("el", "Greek"),
    ("he", "Hebrew"),
    ("hu", "Hungarian"),
    ("ro", "Romanian"),
    ("bg", "Bulgarian"),
    ("hr", "Croatian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("sr", "Serbian"),
    ("lt", "Lithuanian"),
    ("lv", "Latvian"),
    ("et", "Estonian"),
    ("tl", "Filipino"),
    ("sw", "Swahili"),
    ("bn", "Bengali"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("ml", "Malayalam"),
    ("ur", "Urdu"),
    ("fa", "Persian"),
    ("mn", "Mongolian"),
    ("ka", "Georgian"),
    ("az", "Azerbaijani"),
    ("kk", "Kazakh"),
    ("uz", "Uzbek"),
];

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DisplayMode {
    TranslationOnly,
    Both,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub api_url: String,
    pub api_key: String, // empty = no auth
    pub silence_threshold: f32,
    pub language: String,
    pub font_size: f32,
    pub chat_api_url: String,
    pub chat_api_key: String, // empty = no auth
    pub chat_model: String,
    pub target_language: String, // empty = no translation
    pub display_mode: DisplayMode,
    pub opacity: u8,             // 0=transparent, 255=opaque
    pub input_device: String,    // empty = system default
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_url: "https://api.openai.com/v1/audio/transcriptions".to_string(),
            api_key: String::new(),
            silence_threshold: 0.003,
            language: "ko".to_string(),
            font_size: 60.0,
            chat_api_url: "https://api.openai.com/v1/chat/completions".to_string(),
            chat_api_key: String::new(),
            chat_model: "gpt-4o".to_string(),
            target_language: "en".to_string(),
            display_mode: DisplayMode::TranslationOnly,
            opacity: 200,
            input_device: String::new(),
        }
    }
}

fn config_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    exe.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("settings.yml")
}

pub fn sessions_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("sessions");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

impl Settings {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_yaml::from_str::<Settings>(&contents) {
                    return settings;
                }
                eprintln!("Failed to parse {}, using defaults", path.display());
            }
        } else {
            // Create default config file
            let settings = Settings::default();
            settings.save();
            return settings;
        }
        Settings::default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Ok(yaml) = serde_yaml::to_string(self) {
            if let Err(e) = std::fs::write(&path, yaml) {
                eprintln!("Failed to save settings: {e}");
            }
        }
    }
}
