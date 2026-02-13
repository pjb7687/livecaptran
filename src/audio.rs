use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::settings::{DisplayMode, Settings, MAX_PHRASE_SECS, SILENCE_CHUNKS_TO_END};

fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;
    let byte_rate = sample_rate * 2;

    let mut buf = Vec::with_capacity(44 + data_size as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let i = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        buf.extend_from_slice(&i.to_le_bytes());
    }
    buf
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

fn translate_text(
    client: &reqwest::blocking::Client,
    text: &str,
    chat_api_url: &str,
    chat_api_key: &str,
    chat_model: &str,
    target_language: &str,
    history: &VecDeque<(String, String)>,
) -> Option<String> {
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": format!(
            "You are a real-time translator for a scientific presentation. Translate the following spoken text into {}. Preserve technical and scientific terminology accurately. Output only a single, most probable translation. Print only the translated text and absolutely nothing else—no alternatives, no explanations, no notes, no quotation marks.",
            target_language
        )
    })];

    // Include previous transcription/translation pairs as context
    for (orig, translated) in history {
        messages.push(serde_json::json!({"role": "user", "content": orig}));
        messages.push(serde_json::json!({"role": "assistant", "content": translated}));
    }

    messages.push(serde_json::json!({"role": "user", "content": text}));

    let body = serde_json::json!({
        "model": chat_model,
        "messages": messages
    });

    let mut req = client
        .post(chat_api_url)
        .header("Content-Type", "application/json");
    if !chat_api_key.is_empty() {
        req = req.bearer_auth(chat_api_key);
    }
    match req
        .body(body.to_string())
        .send()
    {
        Ok(resp) => {
            if let Ok(body) = resp.text() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        let translated = content.trim().to_string();
                        if !translated.is_empty() {
                            return Some(translated);
                        }
                    }
                }
            }
            None
        }
        Err(e) => {
            eprintln!("Translation error: {e}");
            None
        }
    }
}

fn send_transcription(
    client: &reqwest::blocking::Client,
    samples: &[f32],
    rate: u32,
    transcript: &Arc<Mutex<String>>,
    api_url: &str,
    api_key: &str,
    language: &str,
    chat_api_url: &str,
    chat_api_key: &str,
    chat_model: &str,
    target_language: &str,
    display_mode: &DisplayMode,
    history: &mut VecDeque<(String, String)>,
    log_file: &mut Option<std::fs::File>,
) {
    let wav = encode_wav(samples, rate);
    let form = reqwest::blocking::multipart::Form::new()
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(wav)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .unwrap(),
        )
        .text("model", "large-v3")
        .text("language", language.to_string());

    let mut req = client.post(api_url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    match req.multipart(form).send() {
        Ok(resp) => {
            if let Ok(body) = resp.text() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(text) = json["text"].as_str() {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            let maybe_translated = if !target_language.is_empty() {
                                translate_text(
                                    client,
                                    &text,
                                    chat_api_url,
                                    chat_api_key,
                                    chat_model,
                                    target_language,
                                    history,
                                )
                            } else {
                                None
                            };

                            // Log to session file
                            if let Some(file) = log_file {
                                use std::io::Write;
                                let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                                let _ = writeln!(file, "[{}] {}", now, &text);
                                if let Some(ref tr) = maybe_translated {
                                    let _ = writeln!(file, "[{}] {}", now, tr);
                                }
                                let _ = writeln!(file, "---");
                                let _ = file.flush();
                            }

                            // Build display string
                            let display = if let Some(translated) = maybe_translated {
                                history.push_back((text.clone(), translated.clone()));
                                if history.len() > 3 {
                                    history.pop_front();
                                }
                                match display_mode {
                                    DisplayMode::TranslationOnly => translated,
                                    DisplayMode::Both => {
                                        format!("{text}\n{translated}")
                                    }
                                }
                            } else {
                                text
                            };
                            *transcript.lock().unwrap() = display;
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("Transcription error: {e}"),
    }
}

pub fn start_audio_and_transcription(
    transcript: Arc<Mutex<String>>,
    running: Arc<AtomicBool>,
    settings: Arc<Mutex<Settings>>,
    session_active: Arc<AtomicBool>,
) {
    let audio_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let sample_rate: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    // Audio capture thread
    {
        let buf = audio_buffer.clone();
        let sr = sample_rate.clone();
        let run = running.clone();
        let input_device_name = settings.lock().unwrap().input_device.clone();
        thread::spawn(move || {
            let host = cpal::default_host();
            let device = if input_device_name.is_empty() {
                host.default_input_device()
            } else {
                use cpal::traits::HostTrait;
                host.input_devices()
                    .ok()
                    .and_then(|mut devs| {
                        devs.find(|d| {
                            d.name().map(|n| n == input_device_name).unwrap_or(false)
                        })
                    })
                    .or_else(|| {
                        eprintln!("Device '{}' not found, using default", input_device_name);
                        host.default_input_device()
                    })
            };
            let device = match device {
                Some(d) => d,
                None => {
                    eprintln!("No audio input device found");
                    return;
                }
            };
            let supported = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("No input config: {e}");
                    return;
                }
            };

            *sr.lock().unwrap() = supported.sample_rate().0;
            let channels = supported.channels() as usize;
            let fmt = supported.sample_format();
            let config: cpal::StreamConfig = supported.into();

            let _stream = match fmt {
                cpal::SampleFormat::F32 => {
                    let buf = buf.clone();
                    device
                        .build_input_stream(
                            &config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                let mut b = buf.lock().unwrap();
                                if channels > 1 {
                                    for chunk in data.chunks(channels) {
                                        b.push(chunk.iter().sum::<f32>() / channels as f32);
                                    }
                                } else {
                                    b.extend_from_slice(data);
                                }
                            },
                            |e| eprintln!("Audio error: {e}"),
                            None,
                        )
                        .expect("Failed to build input stream")
                }
                cpal::SampleFormat::I16 => {
                    let buf = buf.clone();
                    device
                        .build_input_stream(
                            &config,
                            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                let mut b = buf.lock().unwrap();
                                if channels > 1 {
                                    for chunk in data.chunks(channels) {
                                        let sum: f32 =
                                            chunk.iter().map(|&s| s as f32 / 32768.0).sum();
                                        b.push(sum / channels as f32);
                                    }
                                } else {
                                    for &s in data {
                                        b.push(s as f32 / 32768.0);
                                    }
                                }
                            },
                            |e| eprintln!("Audio error: {e}"),
                            None,
                        )
                        .expect("Failed to build input stream")
                }
                fmt => panic!("Unsupported sample format: {fmt:?}"),
            };

            _stream.play().expect("Failed to start audio stream");

            while run.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    // VAD + transcription thread
    {
        let buf = audio_buffer;
        let sr = sample_rate;
        let run = running;
        thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client");

            thread::sleep(Duration::from_secs(1));

            let mut speaking = false;
            let mut phrase: Vec<f32> = Vec::new();
            let mut silence_count: usize = 0;
            let mut translation_history: VecDeque<(String, String)> = VecDeque::new();
            let mut log_file: Option<std::fs::File> = None;
            let mut was_session_active = false;

            while run.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));

                let rate = *sr.lock().unwrap();
                if rate == 0 {
                    continue;
                }

                let new_samples: Vec<f32> = {
                    let mut b = buf.lock().unwrap();
                    std::mem::take(&mut *b)
                };

                if new_samples.is_empty() {
                    continue;
                }

                // Session state transitions
                let is_active = session_active.load(Ordering::Relaxed);
                if is_active && !was_session_active {
                    let dir = crate::settings::sessions_dir();
                    let filename = format!(
                        "session_{}.txt",
                        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
                    );
                    match std::fs::File::create(dir.join(&filename)) {
                        Ok(f) => log_file = Some(f),
                        Err(e) => eprintln!("Failed to create session log: {e}"),
                    }
                    was_session_active = true;
                } else if !is_active && was_session_active {
                    log_file = None;
                    *transcript.lock().unwrap() = String::new();
                    phrase.clear();
                    speaking = false;
                    silence_count = 0;
                    was_session_active = false;
                }

                if !is_active {
                    continue;
                }

                let (threshold, api_url, api_key, language, chat_api_url, chat_api_key, chat_model, target_language, display_mode) = {
                    let s = settings.lock().unwrap();
                    (
                        s.silence_threshold,
                        s.api_url.clone(),
                        s.api_key.clone(),
                        s.language.clone(),
                        s.chat_api_url.clone(),
                        s.chat_api_key.clone(),
                        s.chat_model.clone(),
                        s.target_language.clone(),
                        s.display_mode.clone(),
                    )
                };

                let energy = rms(&new_samples);
                let is_voice = energy > threshold;

                if speaking {
                    phrase.extend_from_slice(&new_samples);

                    if is_voice {
                        silence_count = 0;
                    } else {
                        silence_count += 1;
                    }

                    // End of phrase: sustained silence after speech
                    let phrase_too_long = phrase.len() > rate as usize * MAX_PHRASE_SECS;
                    if silence_count >= SILENCE_CHUNKS_TO_END || phrase_too_long {
                        // Trim trailing silence
                        let trim_samples = silence_count * new_samples.len();
                        let end = phrase.len().saturating_sub(trim_samples);
                        if end > rate as usize / 2 {
                            send_transcription(
                                &client,
                                &phrase[..end],
                                rate,
                                &transcript,
                                &api_url,
                                &api_key,
                                &language,
                                &chat_api_url,
                                &chat_api_key,
                                &chat_model,
                                &target_language,
                                &display_mode,
                                &mut translation_history,
                                &mut log_file,
                            );
                        }
                        phrase.clear();
                        speaking = false;
                        silence_count = 0;
                    }
                } else if is_voice {
                    // Speech started
                    speaking = true;
                    silence_count = 0;
                    phrase.clear();
                    phrase.extend_from_slice(&new_samples);
                }
                // If silent and not speaking, discard samples
            }
        });
    }
}
