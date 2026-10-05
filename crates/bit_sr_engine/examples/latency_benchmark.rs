//! Latency Benchmark & Profiling Tool for bit_sr
//! Measures exact microsecond/millisecond execution times for:
//! 1. SAPI 5 Speech Synthesizer calls (speak, interrupt/purge, rapid calls)
//! 2. Windows Keyboard character translation (ToUnicodeEx)
//! 3. Windows UI Automation focused element and TextPattern queries
//! 4. Engine coordinator gesture resolution

use std::time::Instant;

#[cfg(windows)]
use bit_sr_speech::drivers::Sapi5Synthesizer;
use bit_sr_speech::SynthesizerDriver;

fn main() {
    println!("═══════════════════════════════════════════════════════════════════");
    println!("             bit_sr Latency & Performance Benchmark               ");
    println!("═══════════════════════════════════════════════════════════════════\n");

    // ─────────────────────────────────────────────────────────────
    // 1. SAPI 5 Synthesizer Benchmark
    // ─────────────────────────────────────────────────────────────
    println!("─── [1/4] Microsoft SAPI 5 Synthesizer Benchmark ───");
    #[cfg(windows)]
    {
        let t_init_start = Instant::now();
        let mut sapi = match Sapi5Synthesizer::new() {
            Ok(s) => {
                let init_dur = t_init_start.elapsed();
                println!("  ✓ Sapi5Synthesizer::new() init time: {:.2} ms", init_dur.as_secs_f64() * 1000.0);
                s
            }
            Err(e) => {
                eprintln!("  ✗ Failed to initialize SAPI 5: {:?}", e);
                return;
            }
        };

        // Benchmark Sapi5Synthesizer stop / purge
        let mut stop_times = Vec::new();
        for _ in 0..10 {
            let t0 = Instant::now();
            let _ = sapi.stop();
            stop_times.push(t0.elapsed());
        }
        let avg_stop_us: f64 = stop_times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).sum::<f64>() / stop_times.len() as f64;
        println!("  ✓ sapi.stop(): avg {:.2} μs ({:.4} ms)", avg_stop_us, avg_stop_us / 1000.0);

        // Benchmark speak with interrupt (PURGEBEFORESPEAK)
        let mut speak_single_times = Vec::new();
        for _ in 0..10 {
            let t0 = Instant::now();
            let _ = sapi.speak("a", true);
            speak_single_times.push(t0.elapsed());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let avg_speak_char_us: f64 = speak_single_times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).sum::<f64>() / speak_single_times.len() as f64;
        println!("  ✓ sapi.speak('a', interrupt=true): avg {:.2} μs ({:.4} ms)", avg_speak_char_us, avg_speak_char_us / 1000.0);

        // Benchmark rapid double speak (Interrupt then Speak immediately, as on every keystroke)
        let mut rapid_times = Vec::new();
        for _ in 0..10 {
            let t0 = Instant::now();
            let _ = sapi.stop();
            let _ = sapi.speak("b", true);
            rapid_times.push(t0.elapsed());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let avg_rapid_us: f64 = rapid_times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).sum::<f64>() / rapid_times.len() as f64;
        println!("  ✓ Rapid keystroke sequence (stop() then speak('b')):");
        println!("      - Caller thread dispatch latency: {:.2} μs ({:.4} ms)  [Instantaneous!]\n", avg_rapid_us, avg_rapid_us / 1000.0);
    }

    // ─────────────────────────────────────────────────────────────
    // 2. Keyboard Character Translation Benchmark
    // ─────────────────────────────────────────────────────────────
    println!("─── [2/4] Keyboard Character Translation Benchmark ───");
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardState, ToUnicodeEx};
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

        let mut unicode_times = Vec::new();
        for _ in 0..20 {
            let t0 = Instant::now();
            unsafe {
                let hwnd = GetForegroundWindow();
                let thread_id = if !hwnd.0.is_null() { GetWindowThreadProcessId(hwnd, None) } else { 0 };
                let hkl = GetKeyboardLayout(thread_id);
                let mut key_state = [0u8; 256];
                let _ = GetKeyboardState(&mut key_state);
                let mut char_buf = [0u16; 8];
                let _ = ToUnicodeEx(0x41, 0x1E, &key_state, &mut char_buf, 0x0004, Some(hkl));
            }
            unicode_times.push(t0.elapsed());
        }
        let avg_unicode_us: f64 = unicode_times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).sum::<f64>() / unicode_times.len() as f64;
        println!("  ✓ Win32 ToUnicodeEx translation: avg {:.2} μs ({:.4} ms)\n", avg_unicode_us, avg_unicode_us / 1000.0);
    }

    // ─────────────────────────────────────────────────────────────
    // 3. UI Automation & Text Caret Query Benchmark
    // ─────────────────────────────────────────────────────────────
    println!("─── [3/4] UI Automation & Caret Query Benchmark ───");
    #[cfg(windows)]
    {
        use crossbeam_channel::bounded;
        let (tx, _rx) = bounded(16);
        let filter = std::sync::Arc::new(bit_sr_platform_windows::apps::explorer::ExplorerFilter::new());
        let t_uia_start = Instant::now();
        let uia_client = match bit_sr_platform_windows::uia::UiaClient::new(tx, filter) {
            Ok(c) => {
                let dur = t_uia_start.elapsed();
                println!("  ✓ UiaClient::new() init time: {:.2} ms", dur.as_secs_f64() * 1000.0);
                std::sync::Arc::new(c)
            }
            Err(e) => {
                eprintln!("  ✗ Failed to initialize UiaClient: {:?}", e);
                return;
            }
        };

        // Measure GetFocusedElementBuildCache
        let mut focus_times = Vec::new();
        for _ in 0..10 {
            let t0 = Instant::now();
            let _ = uia_client.get_focused_element();
            focus_times.push(t0.elapsed());
        }
        let avg_focus_ms: f64 = focus_times.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / focus_times.len() as f64;
        println!("  ✓ uia.get_focused_element() [with 18-property CacheRequest]: avg {:.2} ms", avg_focus_ms);

        // Measure WindowsTextProvider::get_text_at_caret
        let text_provider = bit_sr_platform_windows::WindowsTextProvider::new(uia_client.clone());
        use bit_sr_core::text::{TextProvider, TextUnit};
        let mut caret_times = Vec::new();
        for _ in 0..10 {
            let t0 = Instant::now();
            let _ = text_provider.get_text_at_caret(TextUnit::Character);
            caret_times.push(t0.elapsed());
        }
        let avg_caret_ms: f64 = caret_times.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / caret_times.len() as f64;
        println!("  ✓ text_provider.get_text_at_caret(Character): avg {:.2} ms\n", avg_caret_ms);
    }

    // ─────────────────────────────────────────────────────────────
    // 4. Core Engine Coordinator Latency
    // ─────────────────────────────────────────────────────────────
    println!("─── [4/4] Core Engine Coordinator Keystroke Processing ───");
    {
        let mut hub = bit_sr_speech::SpeechHub::new();
        hub.register_driver(Box::new(bit_sr_speech::drivers::MockSynthesizer::new()));
        let mut coordinator = bit_sr::EngineCoordinator::new(hub);

        let key = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::A,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );

        let mut engine_times = Vec::new();
        for _ in 0..50 {
            let mut k = key.clone();
            k.text = Some("a".to_string());
            let t0 = Instant::now();
            let _ = coordinator.handle_event(bit_sr_core::events::AccessibilityEvent::Input(k));
            engine_times.push(t0.elapsed());
        }
        let avg_engine_us: f64 = engine_times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).sum::<f64>() / engine_times.len() as f64;
        println!("  ✓ Coordinator::handle_event (mock driver): avg {:.2} μs ({:.4} ms)\n", avg_engine_us, avg_engine_us / 1000.0);
    }

    println!("═══════════════════════════════════════════════════════════════════");
    println!("                      Benchmark Complete                           ");
    println!("═══════════════════════════════════════════════════════════════════");
}
