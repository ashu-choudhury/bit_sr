//! Workarounds and heuristics for Google Chrome and Chromium WebView on Android.

use bit_sr_core::roles::Role;

/// Filter for Android Chrome and WebView content.
pub struct AndroidChromeFilter;

impl AndroidChromeFilter {
    /// Checks if a package name is Chrome or Chromium-based browser.
    pub fn is_chrome_package(package_name: &str) -> bool {
        package_name == "com.android.chrome"
            || package_name == "com.chrome.beta"
            || package_name == "com.chrome.dev"
            || package_name == "com.chrome.canary"
            || package_name == "org.chromium.chrome"
    }

    /// Normalizes role for Chrome web elements on Android.
    pub fn normalize_web_role(class_name: &str, current_role: Role) -> Role {
        if class_name.contains("WebView") || class_name.contains("RenderWidgetHost") {
            Role::Document
        } else {
            current_role
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrome_detection() {
        assert!(AndroidChromeFilter::is_chrome_package("com.android.chrome"));
        assert!(AndroidChromeFilter::is_chrome_package("org.chromium.chrome"));
        assert!(!AndroidChromeFilter::is_chrome_package("com.android.settings"));
    }
}
