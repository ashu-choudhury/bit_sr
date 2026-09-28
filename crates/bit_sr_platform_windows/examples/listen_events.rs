//! Example: Listen for Windows accessibility events and Explorer navigation in real-time.

use bit_sr_platform_windows::WindowsPlatform;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== bit_sr Windows Platform Accessibility Monitor ===");
    println!("Initializing UI Automation, WinEvents/MSAA, and WH_KEYBOARD_LL hook...");

    let (tx, event_rx) = crossbeam_channel::unbounded();
    let mut platform = WindowsPlatform::start(tx)?;

    println!("Listening for system focus, selection, and keyboard events.");
    println!("Switch between File Explorer folders or press any key to test.");
    println!("Press Ctrl+C to exit.\n");

    while let Ok(event) = event_rx.recv() {
        match event {
            bit_sr_core::events::AccessibilityEvent::Focus(node) => {
                println!(
                    "[FOCUS] role={:?} name={:?} class={:?}",
                    node.role, node.name, node.class_name
                );
            }
            bit_sr_core::events::AccessibilityEvent::Selection(node) => {
                println!(
                    "[SELECTION] role={:?} name={:?}",
                    node.role, node.name
                );
            }
            bit_sr_core::events::AccessibilityEvent::SpeechInterrupt => {
                println!("[SPEECH INTERRUPT] (Key pressed)");
            }
            bit_sr_core::events::AccessibilityEvent::Input(key) => {
                println!("[KEY] vk=0x{:X} action={:?} mods={:?}", key.vk_code, key.action, key.modifiers);
            }
            other => {
                println!("[EVENT] {:?}", other);
            }
        }
    }

    platform.stop();
    Ok(())
}
