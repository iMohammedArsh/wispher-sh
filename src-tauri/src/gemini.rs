use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

#[derive(Deserialize)]
struct UploadedFileResponse {
    file: UploadedFile,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadedFile {
    name: String,
    uri: String,
    mime_type: String,
    state: Option<String>,
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

fn class_notes_instruction() -> String {
    String::from(
        "You are Hams' class note maker. Turn the supplied class transcript into clear, study-ready notes without inventing information. \
        Start with a short overview, then use descriptive headings and concise bullet points for the main ideas. \
        Include a section named Key takeaways and, only when present in the transcript, sections for Definitions, Examples, Assignments, and Questions to review. \
        Preserve names, equations, terminology, dates, and the language used by the speaker. \
        If part of the transcript is uncertain or incomplete, say so instead of guessing. \
        Return only the notes in plain Markdown.",
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

    /// Transcribe WAV audio with Google's dedicated Gemini 3.5 speech model.
    /// The temporary cloud file is deleted after the interaction completes.
    pub async fn transcribe_audio(
        &self,
        wav_bytes: &[u8],
        language: &str,
        custom_dictionary: &str,
    ) -> Result<String, String> {
        if self.api_key.trim().is_empty() {
            return Err("Gemini API key is missing.".into());
        }

        let uploaded = self.upload_audio(wav_bytes).await?;
        let uploaded_name = uploaded.name.clone();
        let uploaded = match self.wait_for_file(uploaded).await {
            Ok(file) => file,
            Err(error) => {
                self.delete_file(&uploaded_name).await;
                return Err(error);
            }
        };
        let language_codes: Vec<&str> = match language {
            "en" => vec!["en-US"],
            "ml" => vec!["ml-IN"],
            _ => Vec::new(),
        };
        let vocabulary = custom_dictionary
            .split([',', '\n'])
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .take(1000)
            .collect::<Vec<_>>();
        let mut transcription_config = json!({
            "mode": "verbatim",
            "language_codes": language_codes,
        });
        if !vocabulary.is_empty() {
            transcription_config["custom_vocabulary"] = json!(vocabulary);
        }
        let payload = json!({
            "model": "gemini-3.5-transcribe",
            "input": [{
                "type": "audio",
                "uri": uploaded.uri,
                "mime_type": uploaded.mime_type,
            }],
            "generation_config": {
                "transcription_config": transcription_config,
            },
        });

        let response_result = self
            .client
            .post("https://generativelanguage.googleapis.com/v1beta/interactions")
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        self.delete_file(&uploaded.name).await;
        let response = response_result
            .map_err(|error| format!("Gemini 3.5 Transcribe request failed: {error}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Gemini 3.5 Transcribe error {status}: {body}"));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|error| format!("Could not read Gemini transcription: {error}"))?;
        interaction_output_text(&value)
            .ok_or_else(|| "Gemini 3.5 Transcribe returned no spoken words.".into())
    }

    async fn upload_audio(&self, wav_bytes: &[u8]) -> Result<UploadedFile, String> {
        let byte_count = wav_bytes.len().to_string();
        let start = self
            .client
            .post("https://generativelanguage.googleapis.com/upload/v1beta/files")
            .header("x-goog-api-key", &self.api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", &byte_count)
            .header("X-Goog-Upload-Header-Content-Type", "audio/wav")
            .header("Content-Type", "application/json")
            .json(&json!({ "file": { "display_name": "Hams dictation" } }))
            .send()
            .await
            .map_err(|error| format!("Could not start Gemini audio upload: {error}"))?;
        if !start.status().is_success() {
            let status = start.status();
            let body = start.text().await.unwrap_or_default();
            return Err(format!("Gemini upload error {status}: {body}"));
        }
        let upload_url = start
            .headers()
            .get("x-goog-upload-url")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| "Gemini did not return an audio upload URL.".to_string())?
            .to_string();
        let uploaded = self
            .client
            .post(upload_url)
            .header("Content-Length", &byte_count)
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("Content-Type", "audio/wav")
            .body(wav_bytes.to_vec())
            .send()
            .await
            .map_err(|error| format!("Could not upload audio to Gemini: {error}"))?;
        if !uploaded.status().is_success() {
            let status = uploaded.status();
            let body = uploaded.text().await.unwrap_or_default();
            return Err(format!("Gemini audio upload error {status}: {body}"));
        }
        uploaded
            .json::<UploadedFileResponse>()
            .await
            .map(|response| response.file)
            .map_err(|error| format!("Could not read Gemini upload response: {error}"))
    }

    async fn wait_for_file(&self, mut file: UploadedFile) -> Result<UploadedFile, String> {
        for _ in 0..30 {
            match file.state.as_deref() {
                None | Some("ACTIVE") => return Ok(file),
                Some("FAILED") => return Err("Gemini could not process the uploaded audio.".into()),
                _ => {}
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let response = self
                .client
                .get(format!(
                    "https://generativelanguage.googleapis.com/v1beta/{}",
                    file.name
                ))
                .header("x-goog-api-key", &self.api_key)
                .send()
                .await
                .map_err(|error| format!("Could not check Gemini audio status: {error}"))?;
            if !response.status().is_success() {
                return Err(format!(
                    "Gemini audio status check returned {}.",
                    response.status()
                ));
            }
            file = response
                .json::<UploadedFile>()
                .await
                .map_err(|error| format!("Could not read Gemini audio status: {error}"))?;
        }
        Err("Gemini took too long to prepare the uploaded audio.".into())
    }

    async fn delete_file(&self, name: &str) {
        let _ = self
            .client
            .delete(format!(
                "https://generativelanguage.googleapis.com/v1beta/{name}"
            ))
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await;
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

    /// Convert a completed long-form class transcript into study-ready notes.
    pub async fn create_class_notes(
        &self,
        transcript: &str,
        title: &str,
    ) -> Result<String, String> {
        if self.api_key.trim().is_empty() {
            return Err("Gemini API key is missing.".into());
        }

        let payload = GeminiAudioRequest {
            system_instruction: Content {
                parts: vec![Part::Text {
                    text: class_notes_instruction(),
                }],
            },
            contents: vec![Content {
                parts: vec![Part::Text {
                    text: format!("Class title: {title}\n\nTranscript:\n{transcript}"),
                }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| format!("HTTP request to Gemini API failed: {error}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API error {status}: {body}"));
        }
        let data: GeminiResponse = response
            .json()
            .await
            .map_err(|error| format!("Failed to parse Gemini response: {error}"))?;
        let text = data
            .candidates
            .and_then(|candidates| candidates.into_iter().next())
            .and_then(|candidate| candidate.content)
            .and_then(|content| content.parts)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|part| part.text)
            .collect::<Vec<_>>()
            .join("");
        let text = text.trim().to_string();
        if text.is_empty() {
            Err("Gemini returned empty class notes.".into())
        } else {
            Ok(text)
        }
    }
}

fn interaction_output_text(value: &Value) -> Option<String> {
    let direct = value
        .get("output_text")
        .or_else(|| value.get("outputText"))
        .and_then(Value::as_str);
    if let Some(text) = direct.filter(|text| !text.trim().is_empty()) {
        return Some(text.trim().to_string());
    }
    if let Some(outputs) = value.get("outputs").and_then(Value::as_array) {
        let text = outputs
            .iter()
            .filter_map(|output| output.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
        if !text.trim().is_empty() {
            return Some(text.trim().to_string());
        }
    }
    value
        .get("steps")
        .and_then(Value::as_array)
        .and_then(|steps| {
            steps.iter().rev().find_map(|step| {
                let text = step
                    .get("content")
                    .and_then(Value::as_array)?
                    .iter()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("");
                (!text.trim().is_empty()).then(|| text.trim().to_string())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::interaction_output_text;
    use serde_json::json;

    #[test]
    fn reads_interactions_text_output() {
        let response = json!({
            "steps": [{ "content": [{ "type": "text", "text": "  hello world  " }] }]
        });
        assert_eq!(
            interaction_output_text(&response).as_deref(),
            Some("hello world")
        );
    }
}
