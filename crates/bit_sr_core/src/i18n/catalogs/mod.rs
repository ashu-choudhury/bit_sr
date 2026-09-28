//! Bundled static translation catalogs for bit_sr.

pub mod de;
pub mod en;
pub mod es;
pub mod fr;
pub mod hi;

use std::collections::HashMap;

/// Returns a loaded in-memory catalog map for a given language code.
pub fn get_catalog(lang: &str) -> Option<HashMap<&'static str, &'static str>> {
    let entries = match lang.to_lowercase().as_str() {
        "en" | "en_us" | "en-us" | "en_gb" | "en-gb" => en::catalog(),
        "es" | "es_es" | "es-es" | "es_co" | "es_mx" => es::catalog(),
        "hi" | "hi_in" | "hi-in" => hi::catalog(),
        "fr" | "fr_fr" | "fr-fr" => fr::catalog(),
        "de" | "de_de" | "de-de" => de::catalog(),
        _ => return None,
    };

    let mut map = HashMap::with_capacity(entries.len());
    for &(k, v) in entries {
        map.insert(k, v);
    }
    Some(map)
}
