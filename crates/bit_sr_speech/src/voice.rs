//! Voice metadata and language descriptors.

/// Information describing an installed TTS voice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoiceInfo {
    /// Unique identifier for this voice within the driver (e.g. registry key, URI, or ID).
    pub id: String,

    /// Human-readable display name (e.g. "Microsoft David", "English (America) - Alex").
    pub name: String,

    /// BCP-47 language tag (e.g. "en-US", "en-GB", "hi-IN", "de-DE").
    pub language: String,

    /// Gender classification, if provided by the engine.
    pub gender: Option<VoiceGender>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

impl std::fmt::Display for VoiceGender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Male => write!(f, "Male"),
            Self::Female => write!(f, "Female"),
            Self::Neutral => write!(f, "Neutral"),
        }
    }
}
