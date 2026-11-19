//! The languages Whisper's multilingual models (v1/v2: Base, Small, Medium)
//! can transcribe, and which speech model suits a set of them. Whisper has no
//! per-language downloads: the model decides how well each language works, so
//! "downloading a language" means getting the right model for it.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Lang {
    /// Whisper's code, e.g. "en", "ml", "haw".
    pub code: &'static str,
    /// The English name.
    pub name: &'static str,
    /// The name in the language itself.
    pub native: &'static str,
}

const fn l(code: &'static str, name: &'static str, native: &'static str) -> Lang {
    Lang { code, name, native }
}

/// In Whisper's own order (roughly by how much training audio it had).
/// Cantonese ("yue") is left out: only large-v3 knows it.
pub const LANGUAGES: &[Lang] = &[
    l("en", "English", "English"),
    l("zh", "Chinese", "中文"),
    l("de", "German", "Deutsch"),
    l("es", "Spanish", "Español"),
    l("ru", "Russian", "Русский"),
    l("ko", "Korean", "한국어"),
    l("fr", "French", "Français"),
    l("ja", "Japanese", "日本語"),
    l("pt", "Portuguese", "Português"),
    l("tr", "Turkish", "Türkçe"),
    l("pl", "Polish", "Polski"),
    l("ca", "Catalan", "Català"),
    l("nl", "Dutch", "Nederlands"),
    l("ar", "Arabic", "العربية"),
    l("sv", "Swedish", "Svenska"),
    l("it", "Italian", "Italiano"),
    l("id", "Indonesian", "Bahasa Indonesia"),
    l("hi", "Hindi", "हिन्दी"),
    l("fi", "Finnish", "Suomi"),
    l("vi", "Vietnamese", "Tiếng Việt"),
    l("he", "Hebrew", "עברית"),
    l("uk", "Ukrainian", "Українська"),
    l("el", "Greek", "Ελληνικά"),
    l("ms", "Malay", "Bahasa Melayu"),
    l("cs", "Czech", "Čeština"),
    l("ro", "Romanian", "Română"),
    l("da", "Danish", "Dansk"),
    l("hu", "Hungarian", "Magyar"),
    l("ta", "Tamil", "தமிழ்"),
    l("no", "Norwegian", "Norsk"),
    l("th", "Thai", "ไทย"),
    l("ur", "Urdu", "اردو"),
    l("hr", "Croatian", "Hrvatski"),
    l("bg", "Bulgarian", "Български"),
    l("lt", "Lithuanian", "Lietuvių"),
    l("la", "Latin", "Latina"),
    l("mi", "Māori", "Te Reo Māori"),
    l("ml", "Malayalam", "മലയാളം"),
    l("cy", "Welsh", "Cymraeg"),
    l("sk", "Slovak", "Slovenčina"),
    l("te", "Telugu", "తెలుగు"),
    l("fa", "Persian", "فارسی"),
    l("lv", "Latvian", "Latviešu"),
    l("bn", "Bengali", "বাংলা"),
    l("sr", "Serbian", "Српски"),
    l("az", "Azerbaijani", "Azərbaycan"),
    l("sl", "Slovenian", "Slovenščina"),
    l("kn", "Kannada", "ಕನ್ನಡ"),
    l("et", "Estonian", "Eesti"),
    l("mk", "Macedonian", "Македонски"),
    l("br", "Breton", "Brezhoneg"),
    l("eu", "Basque", "Euskara"),
    l("is", "Icelandic", "Íslenska"),
    l("hy", "Armenian", "Հայերեն"),
    l("ne", "Nepali", "नेपाली"),
    l("mn", "Mongolian", "Монгол"),
    l("bs", "Bosnian", "Bosanski"),
    l("kk", "Kazakh", "Қазақ"),
    l("sq", "Albanian", "Shqip"),
    l("sw", "Swahili", "Kiswahili"),
    l("gl", "Galician", "Galego"),
    l("mr", "Marathi", "मराठी"),
    l("pa", "Punjabi", "ਪੰਜਾਬੀ"),
    l("si", "Sinhala", "සිංහල"),
    l("km", "Khmer", "ខ្មែរ"),
    l("sn", "Shona", "chiShona"),
    l("yo", "Yoruba", "Yorùbá"),
    l("so", "Somali", "Soomaali"),
    l("af", "Afrikaans", "Afrikaans"),
    l("oc", "Occitan", "Occitan"),
    l("ka", "Georgian", "ქართული"),
    l("be", "Belarusian", "Беларуская"),
    l("tg", "Tajik", "Тоҷикӣ"),
    l("sd", "Sindhi", "سنڌي"),
    l("gu", "Gujarati", "ગુજરાતી"),
    l("am", "Amharic", "አማርኛ"),
    l("yi", "Yiddish", "ייִדיש"),
    l("lo", "Lao", "ລາວ"),
    l("uz", "Uzbek", "Oʻzbek"),
    l("fo", "Faroese", "Føroyskt"),
    l("ht", "Haitian Creole", "Kreyòl ayisyen"),
    l("ps", "Pashto", "پښتو"),
    l("tk", "Turkmen", "Türkmen"),
    l("nn", "Norwegian Nynorsk", "Nynorsk"),
    l("mt", "Maltese", "Malti"),
    l("sa", "Sanskrit", "संस्कृतम्"),
    l("lb", "Luxembourgish", "Lëtzebuergesch"),
    l("my", "Burmese", "မြန်မာ"),
    l("bo", "Tibetan", "བོད་ཡིག"),
    l("tl", "Tagalog", "Tagalog"),
    l("mg", "Malagasy", "Malagasy"),
    l("as", "Assamese", "অসমীয়া"),
    l("tt", "Tatar", "Татар"),
    l("haw", "Hawaiian", "ʻŌlelo Hawaiʻi"),
    l("ln", "Lingala", "Lingála"),
    l("ha", "Hausa", "Hausa"),
    l("ba", "Bashkir", "Башҡорт"),
    l("jw", "Javanese", "Basa Jawa"),
    l("su", "Sundanese", "Basa Sunda"),
];

/// Languages the small Base model already handles well. Everything else gets
/// noticeably better with Small.
const BASE_GOOD: &[&str] = &["en", "es", "fr", "de", "it", "pt", "nl"];

pub fn find(code: &str) -> Option<&'static Lang> {
    LANGUAGES.iter().find(|l| l.code == code)
}

/// The phone speech model that suits these languages (main first):
/// English only → the English model (same size, sharper English);
/// languages Base handles well → Base; anything else → Small.
pub fn recommend(languages: &[String]) -> &'static str {
    let codes: Vec<&str> = languages.iter().map(|c| c.trim()).filter(|c| !c.is_empty() && *c != "auto").collect();
    if codes.iter().all(|c| *c == "en") {
        "base.en-q5_1"
    } else if codes.iter().all(|c| BASE_GOOD.contains(c)) {
        "base-q5_1"
    } else {
        "small-q5_1"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn langs(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn recommends_a_model_for_the_languages() {
        assert_eq!(recommend(&[]), "base.en-q5_1");
        assert_eq!(recommend(&langs(&["en"])), "base.en-q5_1");
        assert_eq!(recommend(&langs(&["en", "es"])), "base-q5_1");
        assert_eq!(recommend(&langs(&["en", "ml"])), "small-q5_1");
        assert_eq!(recommend(&langs(&["hi"])), "small-q5_1");
    }

    #[test]
    fn recommended_models_are_phone_models() {
        for set in [langs(&[]), langs(&["fr"]), langs(&["ml"])] {
            assert!(crate::stt::models::info(recommend(&set)).is_some_and(|m| m.mobile));
        }
    }

    #[test]
    fn codes_are_unique_and_named() {
        let mut seen = std::collections::HashSet::new();
        for lang in LANGUAGES {
            assert!(seen.insert(lang.code), "duplicate {}", lang.code);
            assert!(!lang.name.is_empty() && !lang.native.is_empty());
        }
        assert_eq!(LANGUAGES.len(), 99);
    }

    /// Every code is one whisper.cpp knows (guards against typos).
    #[cfg(feature = "local-stt")]
    #[test]
    fn whisper_knows_every_code() {
        for lang in LANGUAGES {
            assert!(whisper_rs::get_lang_id(lang.code).is_some(), "whisper doesn't know {}", lang.code);
        }
    }
}
