//! Live test example for Windows SAPI 5 synthesizer driver in `bit_sr_speech`.
//!
//! Run with: `cargo run -p bit_sr_speech --example speak_sapi`

use bit_sr_speech::drivers::Sapi5Synthesizer;
use bit_sr_speech::{SpeechHub, SpeechPriority};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== bit_sr Speech Subsystem: Live SAPI 5 Test ===");

    let mut hub = SpeechHub::new();

    println!("[1/4] Initializing Windows SAPI 5 Driver...");
    let sapi = Sapi5Synthesizer::new()?;
    hub.register_driver(Box::new(sapi));

    println!("[2/4] Querying available voices...");
    let voices = hub.available_voices();
    for (i, v) in voices.iter().enumerate() {
        println!("  Voice #{}: {} [{}] (Gender: {:?})", i + 1, v.name, v.language, v.gender);
    }

    if voices.is_empty() {
        println!("  WARNING: No SAPI 5 voices detected on this system.");
    } else {
        println!("  Active Driver: {}", hub.active_driver().map(|d| d.name()).unwrap_or("None"));
    }

    println!("[3/4] Speaking test phrase (Asynchronous)...");
    let test_phrase = "Welcome to bit_sr. The high performance native screen reader engine is ready.";
    hub.speak(test_phrase, SpeechPriority::Now)?;
    println!("  Spoken request queued successfully. Waiting for speech audio output...");

    // Give SAPI 5 asynchronous audio thread time to play through the soundcard
    thread::sleep(Duration::from_millis(4500));

    println!("[4/4] Testing speech interruption...");
    hub.speak("This sentence should be cut off immediately when interrupted.", SpeechPriority::Now)?;
    thread::sleep(Duration::from_millis(600));
    hub.interrupt()?;
    println!("  Speech successfully interrupted!");

    hub.speak("Speech interruption verified.", SpeechPriority::Now)?;
    thread::sleep(Duration::from_millis(2500));

    println!("=== SAPI 5 Test Completed Successfully ===");
    Ok(())
}
