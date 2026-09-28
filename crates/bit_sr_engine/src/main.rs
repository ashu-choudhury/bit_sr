//! bit_sr Screen Reader - Main CLI Binary
//! High-performance native screen reader for Windows.

use bit_sr_core::events::AccessibilityEvent;
use bit_sr_engine::EngineCoordinator;
use bit_sr_speech::SpeechHub;
use crossbeam_channel::bounded;

#[cfg(windows)]
use bit_sr_platform_windows::WindowsPlatform;
#[cfg(windows)]
use bit_sr_speech::drivers::Sapi5Synthesizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!(
        r#"
╔═══════════════════════════════════════════════════════════════════╗
║                     bit_sr Screen Reader v0.1.0                   ║
║         High-Performance Native Screen Reader for Windows         ║
╚═══════════════════════════════════════════════════════════════════╝
  Hotkeys:
    • CapsLock (default SR key) or Insert
    • SR + 1                    : Toggle Input Help mode (test any key safely!)
    • Double-tap CapsLock       : Toggle Caps Lock hardware state on/off
    • SR + Tab                  : Repeat current focus
    • SR + T                    : Speak active window title
    • SR + S                    : Toggle speech mode (Talk / Mute)
    • SR + Ctrl + Up/Down       : Adjust volume
    • SR + Ctrl + Left/Right    : Adjust speech rate
    • Ctrl + Alt + Q or SR + Q  : Exit cleanly
"#
    );

    // 1. Initialize Speech Hub
    let mut speech_hub = SpeechHub::new();

    #[cfg(windows)]
    {
        println!("[1/3] Initializing Windows SAPI 5 speech synthesizer...");
        match Sapi5Synthesizer::new() {
            Ok(sapi) => {
                speech_hub.register_driver(Box::new(sapi));
                println!("  SAPI 5 driver active.");
            }
            Err(e) => {
                eprintln!(
                    "  WARNING: Could not initialize SAPI 5: {:?}. Using fallback mock driver.",
                    e
                );
                speech_hub.register_driver(Box::new(bit_sr_speech::drivers::MockSynthesizer::new()));
            }
        }
    }

    #[cfg(not(windows))]
    {
        speech_hub.register_driver(Box::new(bit_sr_speech::drivers::MockSynthesizer::new()));
    }

    // 2. Initialize Event Channels
    let (event_tx, event_rx) = bounded::<AccessibilityEvent>(256);
    let (shutdown_tx, shutdown_rx) = bounded::<()>(1);

    // Set up Ctrl+C handler for graceful console exit
    let shutdown_tx_ctrlc = shutdown_tx.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        println!("\nReceived shutdown signal (Ctrl+C). Terminating bit_sr...");
        let _ = shutdown_tx_ctrlc.try_send(());
    }) {
        eprintln!("Warning: Failed to set Ctrl+C handler: {:?}", e);
    }

    // 3. Start Windows Accessibility & Input Platform
    #[cfg(windows)]
    println!("[2/3] Hooking Windows UI Automation and low-level keyboard...");
    #[cfg(windows)]
    let mut platform = match WindowsPlatform::start(event_tx) {
        Ok(p) => {
            println!("  Windows platform hooks registered successfully!");
            p
        }
        Err(e) => {
            eprintln!("Failed to initialize Windows platform: {:?}", e);
            return Err(Box::new(e));
        }
    };

    // Detect system UI locale on Windows or fallback to English
    #[cfg(windows)]
    let detected_locale = bit_sr_platform_windows::get_user_default_locale_name();
    #[cfg(not(windows))]
    let detected_locale: Option<String> = None;

    let initial_locale = detected_locale.as_deref().unwrap_or("en");

    // 4. Run Engine Coordinator with localized catalogs
    let mut coordinator = EngineCoordinator::with_locale(speech_hub, initial_locale);
    let active_info = coordinator.loc.current_locale_info();
    println!(
        "[3/3] Starting screen reader engine (Locale: {} / {} [{}], detected: {:?})...",
        active_info.code, active_info.english_name, active_info.native_name, detected_locale
    );

    let ready_msg = coordinator.loc.t("system.app_ready");
    let _ = coordinator.speech_hub.speak(ready_msg, bit_sr_speech::SpeechPriority::Now);

    #[cfg(windows)]
    if let Some(tp) = platform.text_provider() {
        coordinator.set_text_provider(tp);
    }

    #[cfg(windows)]
    if let Some(title) = bit_sr_platform_windows::get_foreground_window_title() {
        coordinator.focus_tracker.set_window_title(title);
    }

    coordinator.run(event_rx, shutdown_rx);

    // 5. Clean Shutdown
    #[cfg(windows)]
    {
        println!("Cleaning up Windows platform hooks...");
        platform.stop();
    }

    println!("bit_sr screen reader terminated cleanly. Goodbye!");
    Ok(())
}
