export interface AppSettings {
  gemini_api_key: string;
  gemini_model?: string;
  custom_dictionary: string;
  whisper_model_path: string;
  whisper_binary_path: string;
  audio_gain_factor: number;
  hotkey_combo: string;
  transcription_language: "auto" | "en" | "ml";
  delivery_mode: string;
  app_usage_tracking: boolean;
}

export interface AppStatus {
  state_name: string;
  recording_mode: string | null;
  last_dictation: string;
  error_message: string | null;
  delivery_message: string | null;
}

export interface AudioDiagnostics {
  device_name: string;
  native_sample_rate: number;
  channels: number;
  recorded_duration_secs: number;
  peak_amplitude: number;
  rms_amplitude: number;
  is_clipping: boolean;
  is_recording: boolean;
  message: string | null;
}

export interface DictationRecord {
  id: string;
  text: string;
  created_at_ms: number;
  mode?: string;
  is_pinned?: boolean;
}

export interface ClassSession {
  id: string;
  title: string;
  created_at_ms: number;
  duration_secs: number;
  audio_path: string;
  transcript: string;
  notes: string;
  status: "processing" | "ready" | "error";
  error_message: string | null;
}

export interface ClassRecordingStatus {
  is_recording: boolean;
  started_at_ms: number | null;
  title: string | null;
}

export interface CorrectionRule {
  source: string;
  replacement: string;
  uses: number;
  enabled: boolean;
  created_at_ms: number;
  last_used_at_ms: number;
}

export interface InsightsSummary {
  tracked_words: number;
  completed_dictations: number;
  average_wpm: number;
  current_streak_days: number;
  longest_streak_days: number;
  profile_unlocked: boolean;
  category_words: Record<string, number>;
  activity_days: Record<string, number>;
  correction_rules: CorrectionRule[];
}

export type ActiveTab = "studio" | "classes" | "history" | "insights" | "vocabulary" | "overlay-lab" | "settings" | "guide";

export type TransformMode = "smart" | "raw" | "code" | "email" | "bullets";

export interface PresetVocabulary {
  id: string;
  name: string;
  description: string;
  words: string[];
}
