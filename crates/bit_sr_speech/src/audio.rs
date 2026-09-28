//! Unified Audio Buffers and Stream Abstractions.
//! Used by synthesizers that generate raw PCM audio frames (e.g. eSpeak NG, Wasm synthesizers).

/// Audio format specification for PCM audio buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    /// Sampling rate in Hz (e.g. 16000, 22050, 44100, 48000).
    pub sample_rate: u32,

    /// Number of audio channels (1 for Mono, 2 for Stereo).
    pub channels: u16,

    /// Bit depth per sample (typically 16 for i16, 32 for f32).
    pub bits_per_sample: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 22050,
            channels: 1,
            bits_per_sample: 16,
        }
    }
}

/// A chunk of linear PCM audio samples produced by a synthesizer.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    pub format: AudioFormat,
    /// Interleaved 16-bit signed PCM samples.
    pub samples: Vec<i16>,
}

/// Callback trait for receiving raw PCM audio frames from synthesizers.
pub trait AudioConsumer: Send + Sync {
    /// Consumes a chunk of audio for playback.
    fn consume_audio(&self, chunk: AudioChunk);

    /// Instantly pauses or cancels pending audio playback.
    fn flush(&self);
}
