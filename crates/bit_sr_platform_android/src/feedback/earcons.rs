//! Low-latency auditory earcons for Android.
//! Auditory cues indicate screen boundaries, touch clicks, page loads, and focus shifts.

/// Types of auditory cues / earcons supported on Android.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EarconType {
    /// Subtle tick when a new element is touched or focused.
    Tick,
    /// Thud/bump when reaching the top or bottom of a list or screen boundary.
    Boundary,
    /// Click sound when double-tap or click action is performed.
    Click,
    /// Swoosh sound when scrolling a view.
    Scroll,
    /// Sound indicating a page or window loaded.
    PageLoad,
}

impl EarconType {
    /// Integer ID for fast JNI / sound pool routing.
    pub fn id(&self) -> i32 {
        match self {
            Self::Tick => 1,
            Self::Boundary => 2,
            Self::Click => 3,
            Self::Scroll => 4,
            Self::PageLoad => 5,
        }
    }
}
