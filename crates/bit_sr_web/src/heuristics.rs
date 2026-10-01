//! Universal Web heuristics, string sanitizers, and anti-chatter cleanup.

use bit_sr_core::roles::Role;

pub struct WebHeuristics;

impl WebHeuristics {
    /// Object Replacement Character used in embedded text runs across Blink and Gecko.
    pub const OBJECT_REPLACEMENT_CHAR: char = '\u{FFFC}';

    /// Sanitizes web strings: replaces non-breaking spaces and removes embedded replacement chars.
    pub fn sanitize_text(input: &str) -> String {
        input
            .replace(Self::OBJECT_REPLACEMENT_CHAR, " ")
            .replace('\u{00A0}', " ")
            .replace('\u{200B}', "") // Zero-width space
            .trim()
            .to_string()
    }

    /// Cleans up redundant author prefixes like "button submit" or "link google.com".
    pub fn strip_redundant_role_prefix(text: &str, role: Role) -> &str {
        let trimmed = text.trim();
        let prefix = match role {
            Role::Button => "button",
            Role::Link => "link",
            Role::Heading => "heading",
            Role::CheckBox => "checkbox",
            _ => return trimmed,
        };

        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let candidate = rest.trim_start_matches([':', '-', ' ']);
            if !candidate.is_empty() {
                return candidate;
            }
        }
        trimmed
    }

    /// Returns true if an image is purely decorative and should be omitted from Browse Mode.
    pub fn is_decorative_image(alt: Option<&str>, role: Role) -> bool {
        if role != Role::Graphic {
            return false;
        }
        match alt {
            None => true,
            Some(t) => {
                let s = t.trim();
                s.is_empty() || s == "\"\"" || s.eq_ignore_ascii_case("null")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_text_sanitizer() {
        let dirty = "Hello\u{00A0}world\u{FFFC}!";
        assert_eq!(WebHeuristics::sanitize_text(dirty), "Hello world !");
    }

    #[test]
    fn test_strip_redundant_prefixes() {
        assert_eq!(
            WebHeuristics::strip_redundant_role_prefix("button: Submit Form", Role::Button),
            "Submit Form"
        );
        assert_eq!(
            WebHeuristics::strip_redundant_role_prefix("link - Terms of Service", Role::Link),
            "Terms of Service"
        );
    }

    #[test]
    fn test_decorative_image_detection() {
        assert!(WebHeuristics::is_decorative_image(None, Role::Graphic));
        assert!(WebHeuristics::is_decorative_image(Some(""), Role::Graphic));
        assert!(WebHeuristics::is_decorative_image(Some("  "), Role::Graphic));
        assert!(!WebHeuristics::is_decorative_image(Some("Company Logo"), Role::Graphic));
    }
}
