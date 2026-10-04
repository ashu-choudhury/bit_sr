//! Android System UI heuristics (Status bar, Navigation bar, Notification shade, Quick settings).

use bit_sr_core::roles::Role;

/// System UI filter for navigation and notifications.
pub struct SystemUiFilter;

impl SystemUiFilter {
    /// Checks if a package name is Android System UI.
    pub fn is_system_ui(package_name: &str) -> bool {
        package_name == "com.android.systemui"
    }

    /// Checks if a view is a status bar or navigation bar container.
    pub fn is_system_bar(resource_id: &str) -> bool {
        resource_id.contains("status_bar")
            || resource_id.contains("navigation_bar")
            || resource_id.contains("quick_settings")
    }

    /// Normalizes system bar elements.
    pub fn normalize_role(resource_id: &str, current_role: Role) -> Role {
        if resource_id.contains("battery") || resource_id.contains("clock") {
            Role::StaticText
        } else if resource_id.contains("back") || resource_id.contains("home") || resource_id.contains("recent") {
            Role::Button
        } else {
            current_role
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_ui_detection() {
        assert!(SystemUiFilter::is_system_ui("com.android.systemui"));
        assert!(SystemUiFilter::is_system_bar("com.android.systemui:id/status_bar_container"));
    }
}
