//! Universal Localization (i18n) and Translation Subsystem.
//!
//! Provides compile-time bundled language catalogs, thread-safe runtime locale
//! switching, fallback chains (e.g. `es-MX` -> `es` -> `en` -> key),
//! template placeholder formatting, and zero-allocation static string lookups.

pub mod catalogs;

use std::collections::HashMap;
use std::sync::RwLock;

/// Metadata describing an available language locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleInfo {
    /// BCP-47 / ISO language code (e.g., "en", "es", "hi").
    pub code: &'static str,
    /// Canonical English name of the language (e.g., "Spanish").
    pub english_name: &'static str,
    /// Native endonym of the language (e.g., "Español", "हिन्दी").
    pub native_name: &'static str,
}

impl LocaleInfo {
    /// List of all first-party language catalogs bundled with bit_sr.
    pub fn available() -> &'static [LocaleInfo] {
        &[
            LocaleInfo {
                code: "en",
                english_name: "English",
                native_name: "English",
            },
            LocaleInfo {
                code: "es",
                english_name: "Spanish",
                native_name: "Español",
            },
            LocaleInfo {
                code: "hi",
                english_name: "Hindi",
                native_name: "हिन्दी",
            },
            LocaleInfo {
                code: "fr",
                english_name: "French",
                native_name: "Français",
            },
            LocaleInfo {
                code: "de",
                english_name: "German",
                native_name: "Deutsch",
            },
        ]
    }
}

/// Normalizes a language tag (e.g., "en-US", "en_GB", "es-CO", "hi-IN", "de_DE")
/// into a supported canonical base locale code.
pub fn normalize_locale(code: &str) -> &'static str {
    let clean = code.trim().to_lowercase().replace('_', "-");
    let primary = clean.split('-').next().unwrap_or("en");

    match primary {
        "es" => "es",
        "hi" => "hi",
        "fr" => "fr",
        "de" => "de",
        _ => "en",
    }
}

/// Thread-safe localization and translation manager.
pub struct LocalizationManager {
    active_locale: RwLock<&'static str>,
    catalogs: HashMap<&'static str, HashMap<&'static str, &'static str>>,
}

impl LocalizationManager {
    /// Creates a new `LocalizationManager` initialized with the given locale code.
    pub fn new(locale: &str) -> Self {
        let norm = normalize_locale(locale);

        let mut catalogs = HashMap::new();
        // Load all bundled catalogs
        for loc in LocaleInfo::available() {
            if let Some(cat) = catalogs::get_catalog(loc.code) {
                catalogs.insert(loc.code, cat);
            }
        }

        Self {
            active_locale: RwLock::new(norm),
            catalogs,
        }
    }

    /// Sets the active locale at runtime.
    pub fn set_locale(&self, locale: &str) {
        let norm = normalize_locale(locale);
        if let Ok(mut lock) = self.active_locale.write() {
            *lock = norm;
        }
    }

    /// Returns the currently active canonical locale code (e.g. "en", "es", "hi").
    pub fn locale(&self) -> &'static str {
        if let Ok(lock) = self.active_locale.read() {
            *lock
        } else {
            "en"
        }
    }

    /// Returns the `LocaleInfo` metadata for the currently active locale.
    pub fn current_locale_info(&self) -> LocaleInfo {
        let active = self.locale();
        LocaleInfo::available()
            .iter()
            .find(|info| info.code == active)
            .cloned()
            .unwrap_or(LocaleInfo {
                code: "en",
                english_name: "English",
                native_name: "English",
            })
    }

    /// Looks up a localized translation for `key`.
    ///
    /// Fallback chain:
    /// 1. Active locale catalog (e.g. Spanish)
    /// 2. English baseline catalog
    /// 3. Returns `None` if key does not exist.
    pub fn lookup(&self, key: &str) -> Option<&'static str> {
        let active = self.locale();

        // 1. Try active catalog
        if let Some(cat) = self.catalogs.get(active) {
            if let Some(&val) = cat.get(key) {
                return Some(val);
            }
        }

        // 2. Fallback to English baseline if active is not English
        if active != "en" {
            if let Some(cat) = self.catalogs.get("en") {
                if let Some(&val) = cat.get(key) {
                    return Some(val);
                }
            }
        }

        None
    }

    /// Translates `key` into the current language, falling back to English,
    /// or returning `key` itself if missing. Infallible and zero-allocation.
    pub fn t(&self, key: &'static str) -> &'static str {
        self.lookup(key).unwrap_or(key)
    }

    /// Translates `key` with dynamic placeholder substitution (e.g. `{pos}`, `{count}`).
    ///
    /// # Example
    /// ```rust
    /// use bit_sr_core::i18n::LocalizationManager;
    /// let loc = LocalizationManager::new("en");
    /// let formatted = loc.t_args("format.pos_of_total", &[("pos", "1"), ("count", "5")]);
    /// assert_eq!(formatted, "1 of 5");
    /// ```
    pub fn t_args(&self, key: &'static str, args: &[(&str, &str)]) -> String {
        let template = self.t(key);
        let mut result = template.to_string();
        for &(param, val) in args {
            let placeholder = format!("{{{}}}", param);
            result = result.replace(&placeholder, val);
        }
        result
    }

    /// Returns all available first-party locales supported by the engine.
    pub fn available_locales(&self) -> &'static [LocaleInfo] {
        LocaleInfo::available()
    }
}

impl Default for LocalizationManager {
    fn default() -> Self {
        Self::new("en")
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_normalization() {
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("en_GB"), "en");
        assert_eq!(normalize_locale("es-ES"), "es");
        assert_eq!(normalize_locale("es_MX"), "es");
        assert_eq!(normalize_locale("hi-IN"), "hi");
        assert_eq!(normalize_locale("fr-FR"), "fr");
        assert_eq!(normalize_locale("de-DE"), "de");
        assert_eq!(normalize_locale("ja-JP"), "en"); // unsupported falls back to en
    }

    #[test]
    fn test_english_translation() {
        let loc = LocalizationManager::new("en");
        assert_eq!(loc.t("role.button"), "button");
        assert_eq!(loc.t("role.checkbox"), "check box");
        assert_eq!(loc.t("state.checked"), "checked");
        assert_eq!(loc.t("system.speech_talk"), "Speech on");
    }

    #[test]
    fn test_spanish_translation() {
        let loc = LocalizationManager::new("es");
        assert_eq!(loc.t("role.button"), "botón");
        assert_eq!(loc.t("role.checkbox"), "casilla de verificación");
        assert_eq!(loc.t("state.checked"), "marcado");
        assert_eq!(loc.t("system.speech_talk"), "Voz activada");
    }

    #[test]
    fn test_hindi_translation() {
        let loc = LocalizationManager::new("hi");
        assert_eq!(loc.t("role.button"), "बटन");
        assert_eq!(loc.t("role.checkbox"), "चेक बॉक्स");
        assert_eq!(loc.t("state.checked"), "चेक किया गया");
        assert_eq!(loc.t("system.speech_talk"), "आवाज चालू");
    }

    #[test]
    fn test_french_translation() {
        let loc = LocalizationManager::new("fr");
        assert_eq!(loc.t("role.button"), "bouton");
        assert_eq!(loc.t("role.checkbox"), "case à cocher");
        assert_eq!(loc.t("state.checked"), "coché");
        assert_eq!(loc.t("system.speech_talk"), "Voix activée");
    }

    #[test]
    fn test_german_translation() {
        let loc = LocalizationManager::new("de");
        assert_eq!(loc.t("role.button"), "Schaltfläche");
        assert_eq!(loc.t("role.checkbox"), "Kontrollkästchen");
        assert_eq!(loc.t("state.checked"), "aktiviert");
        assert_eq!(loc.t("system.speech_talk"), "Sprache ein");
    }

    #[test]
    fn test_fallback_to_english() {
        let loc = LocalizationManager::new("es");
        // Unknown key in Spanish should fall back to English if it existed, or return key itself
        assert_eq!(loc.t("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn test_template_formatting() {
        let loc = LocalizationManager::new("en");
        let formatted = loc.t_args("format.pos_of_total", &[("pos", "3"), ("count", "10")]);
        assert_eq!(formatted, "3 of 10");

        loc.set_locale("es");
        let formatted_es = loc.t_args("format.pos_of_total", &[("pos", "3"), ("count", "10")]);
        assert_eq!(formatted_es, "3 de 10");

        loc.set_locale("hi");
        let formatted_hi = loc.t_args("format.pos_of_total", &[("pos", "3"), ("count", "10")]);
        assert_eq!(formatted_hi, "10 में से 3");
    }

    #[test]
    fn test_runtime_locale_switching() {
        let loc = LocalizationManager::new("en");
        assert_eq!(loc.t("system.speech_mute"), "Speech muted");

        loc.set_locale("hi");
        assert_eq!(loc.t("system.speech_mute"), "आवाज म्यूट");

        loc.set_locale("de");
        assert_eq!(loc.t("system.speech_mute"), "Sprache stumm");
    }
}
