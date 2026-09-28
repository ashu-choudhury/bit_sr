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
    • Insert + Tab              : Repeat current focus
    • Insert + T                : Speak active window title
    • Insert + S                : Toggle speech mode (Talk / Mute)
    • Insert + Ctrl + Up/Down   : Adjust volume
    • Insert + Ctrl + Left/Right: Adjust speech rate
    • Ctrl + Alt + Q or Insert+Q: Exit cleanly
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

    println!("[3/3] Starting screen reader engine...");
    let _ = speech_hub.speak("bit_sr screen reader active.", bit_sr_speech::SpeechPriority::Now);

    // 4. Run Engine Coordinator
    let coordinator = EngineCoordinator::new(speech_hub);
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
