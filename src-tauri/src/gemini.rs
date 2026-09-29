use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ── Gemini Request Structures ──────────────────────────────────────────────

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String, // base64
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    Audio { inline_data: InlineData },
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct GeminiAudioRequest {
    system_instruction: Content,
    contents: Vec<Content>,
}

// ── Gemini Response Structures ─────────────────────────────────────────────

#[derive(Deserialize)]
struct GeminiCandidatePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Option<Vec<GeminiCandidatePart>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

// ── Gemini Formatter ───────────────────────────────────────────────────────

pub struct GeminiFormatter {
    api_key: String,
    model: String,
    client: Client,
}

fn smart_dictation_instruction() -> String {
    String::from(
        "You are Hams' smart dictation formatter. Convert the supplied raw voice transcript into clean, paste-ready writing while preserving the speaker's exact meaning and language. \
        Preserve Malayalam script, English, and mixed Malayalam-English dictation; do not translate. \
        Correct only obvious grammar, capitalization, punctuation, spacing, and paragraph breaks. \
        If two or more clearly distinct enumerated items are dictated (for example 'first', 'second', 'third'), render an ordered list. \
        Render an explicitly dictated bullet or item list as a bullet list. \
        Add question marks only for actual questions. Create a heading only when the speaker explicitly dictates a title or heading. \
        For clear tasks or next steps, use a concise list. \
        Preserve names, jargon, acronyms, commands, numbers, URLs, and custom vocabulary exactly. \
        Never invent facts, add explanations, greetings, advice, summaries, or conversational replies. \
        Never expand a short dictation into a long document. \
        Return ONLY the final formatted text.",
    )
}

impl GeminiFormatter {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: Client::new(),
        }
    }

    /// Transcribe raw WAV audio bytes using Gemini's multimodal audio input,
    /// then semantically format the result. Returns clean formatted text.
    pub async fn transcribe_and_format(
        &self,
        wav_bytes: &[u8],
        custom_dictionary: &str,
    ) -> Result<String, String> {
        if self.api_key.trim().is_empty() {
            return Err("Gemini API key is missing. Please set your API key in Settings.".into());
        }

        let audio_b64 = BASE64.encode(wav_bytes);

        let mut system_prompt = smart_dictation_instruction();
        system_prompt.push_str(
            " Accurately transcribe the supplied audio before applying these formatting rules.",
        );

        if !custom_dictionary.trim().is_empty() {
            system_prompt.push_str(&format!(
                "\n\nCustom vocabulary (use exact spelling): {}",
                custom_dictionary
            ));
        }

        let payload = GeminiAudioRequest {
            system_instruction: Content {
                parts: vec![Part::Text {
                    text: system_prompt,
                }],
            },
            contents: vec![Content {
                parts: vec![
                    Part::Audio {
                        inline_data: InlineData {
                            mime_type: "audio/wav".into(),
                            data: audio_b64,
                        },
                    },
                    Part::Text {
                        text: "Transcribe and format this dictation audio.".into(),
                    },
                ],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP request to Gemini API failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("Gemini API error {}: {}", status, body));
        }

        let data: GeminiResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

        if let Some(candidates) = data.candidates {
            if let Some(first) = candidates.first() {
                if let Some(ref content) = first.content {
                    if let Some(ref parts) = content.parts {
                        let text = parts
                            .iter()
                            .filter_map(|p| p.text.as_ref())
                            .cloned()
                            .collect::<Vec<String>>()
                            .join("");
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            return Ok(trimmed);
                        }
                    }
                }
            }
        }

        Err("Gemini returned empty transcription response.".into())
    }

    /// Format already-transcribed text (used when local Whisper provides raw transcript)
    pub async fn format_text(
        &self,
        raw_text: &str,
        custom_dictionary: &str,
    ) -> Result<String, String> {
        if self.api_key.trim().is_empty() {
            return Err("Gemini API key is missing.".into());
        }

        let mut system_prompt = smart_dictation_instruction();

        if !custom_dictionary.trim().is_empty() {
            system_prompt.push_str(&format!(
                "\n\nCustom Dictionary & Vocabulary Terms (use exact spelling when applicable): {}",
                custom_dictionary
            ));
        }

        let payload = GeminiAudioRequest {
            system_instruction: Content {
                parts: vec![Part::Text {
                    text: system_prompt,
                }],
            },
            contents: vec![Content {
                parts: vec![Part::Text {
                    text: raw_text.to_string(),
                }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP request error to Gemini API: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!(
                "Gemini API responded with error {}: {}",
                status, err_body
            ));
        }

        let response_data: GeminiResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini API JSON response: {}", e))?;

        if let Some(candidates) = response_data.candidates {
            if let Some(first_cand) = candidates.first() {
                if let Some(ref content) = first_cand.content {
                    if let Some(ref parts) = content.parts {
                        let text = parts
                            .iter()
                            .filter_map(|p| p.text.as_ref())
                            .cloned()
                            .collect::<Vec<String>>()
                            .join("");
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            return Ok(trimmed);
                        }
                    }
                }
            }
        }

        Err("Gemini API returned empty candidates array.".into())
    }
}
