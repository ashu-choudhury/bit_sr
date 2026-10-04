//! Application-specific filters, heuristics, and OEM quirks for Android.

pub mod chrome;
pub mod keyboard;
pub mod system_ui;

pub use chrome::AndroidChromeFilter;
pub use keyboard::SoftKeyboardFilter;
pub use system_ui::SystemUiFilter;

/// Identifies special Android applications and system packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidAppType {
    Chrome,
    SystemUi,
    SoftKeyboard,
    Settings,
    Generic,
}

pub fn classify_package_name(pkg: &str) -> AndroidAppType {
    match pkg {
        "com.android.chrome" | "org.chromium.chrome" => AndroidAppType::Chrome,
        "com.android.systemui" => AndroidAppType::SystemUi,
        pkg if pkg.contains("inputmethod") || pkg.contains("latin") || pkg.contains("gboard") => {
            AndroidAppType::SoftKeyboard
        }
        "com.android.settings" => AndroidAppType::Settings,
        _ => AndroidAppType::Generic,
    }
}
