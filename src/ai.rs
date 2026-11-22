//! Optional AI pictures. Google Gemini and OpenAI need the person's own
//! key, which is kept in PrintCraft's settings on their computer and sent
//! only to the service they chose.

use std::sync::{Arc, Mutex};

use base64::Engine;

pub const GEMINI_KEY_PAGE: &str = "https://aistudio.google.com/apikey";
pub const OPENAI_KEY_PAGE: &str = "https://platform.openai.com/api-keys";
/// Gemini picture models, newest first; the next one is tried when a key
/// can't use the first.
const GEMINI_MODELS: [&str; 2] = ["gemini-2.5-flash-image", "gemini-2.0-flash-preview-image-generation"];

pub type Slot = Arc<Mutex<Option<Result<Vec<u8>, String>>>>;

pub fn open_in_browser(url: &str) {
    crate::platform::open_url(url);
}

fn post(url: &str, auth: Option<&str>, body: serde_json::Value) -> Result<serde_json::Value, String> {
    use crate::platform::PostError;
    if crate::platform::WEB {
        return Err(crate::platform::DESKTOP_ONLY.into());
    }
    match crate::platform::http_post_json(url, auth, body, 120) {
        Ok(v) => Ok(v),
        Err(PostError::Status(code, body)) => {
            let msg = body.and_then(|v| v["error"]["message"].as_str().map(str::to_string)).unwrap_or_default();
            Err(match code {
                400 | 401 | 403 => format!("The service didn't accept the key or request. {msg}"),
                429 => "The free limit for now has been reached. Please try again later.".into(),
                _ => format!("The AI service had a problem (code {code}). {msg}"),
            })
        }
        Err(PostError::Unreachable) => Err("PrintCraft couldn't reach the AI service. Check the internet connection.".into()),
    }
}

fn gemini(key: &str, prompt: &str) -> Result<Vec<u8>, String> {
    let mut last = Err("No picture came back.".to_string());
    for model in GEMINI_MODELS {
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}");
        let body = serde_json::json!({"contents": [{"parts": [{"text": format!("Create a picture: {prompt}")}]}], "generationConfig": {"responseModalities": ["TEXT", "IMAGE"]}});
        last = post(&url, None, body).and_then(|v| {
            let parts = v["candidates"][0]["content"]["parts"].as_array().cloned().unwrap_or_default();
            let data = parts.iter().find_map(|p| p["inlineData"]["data"].as_str().or(p["inline_data"]["data"].as_str())).ok_or("No picture came back. Try describing it differently.")?;
            base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| e.to_string())
        });
        if last.is_ok() {
            break;
        }
    }
    last.map_err(|e| {
        if e.contains("free limit") {
            "Google's free plan doesn't include pictures for this key right now. Turn on billing in Google AI Studio, or try OpenAI.".into()
        } else {
            e
        }
    })
}

/// Ask for a picture in the background; the result lands in the slot.
pub fn generate(provider: &str, key: &str, prompt: &str, ctx: &eframe::egui::Context) -> Slot {
    let slot: Slot = Arc::new(Mutex::new(None));
    let (provider, key, prompt, out, ctx) = (provider.to_string(), key.trim().to_string(), prompt.to_string(), slot.clone(), ctx.clone());
    crate::platform::spawn(move || {
        let r = match provider.as_str() {
            "openai" => post("https://api.openai.com/v1/images/generations", Some(&key), serde_json::json!({"model": "gpt-image-1", "prompt": prompt, "size": "1024x1024"})).and_then(|v| {
                let b = v["data"][0]["b64_json"].as_str().ok_or("No picture came back.")?;
                base64::engine::general_purpose::STANDARD.decode(b).map_err(|e| e.to_string())
            }),
            _ => gemini(&key, &prompt),
        };
        *out.lock().unwrap() = Some(r);
        ctx.request_repaint();
    });
    slot
}
