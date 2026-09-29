//! Memory and resource limiter enforcement via Wasmtime's `ResourceLimiter`.
//!
//! Prevents misbehaving or rogue extensions from starving host memory
//! by clamping dynamic memory growth to `max_memory_mb`.

use wasmtime::ResourceLimiter;

/// Resource limiter applied per extension instance.
#[derive(Debug, Clone)]
pub struct ExtensionMemoryLimiter {
    /// Maximum allowed memory size in bytes.
    pub max_bytes: usize,
    /// Maximum number of table elements allowed.
    pub max_table_elements: usize,
}

impl ExtensionMemoryLimiter {
    /// Constructs a memory limiter from a megabyte allocation limit.
    pub fn from_mb(max_memory_mb: u32) -> Self {
        Self {
            max_bytes: (max_memory_mb as usize) * 1024 * 1024,
            max_table_elements: 100_000,
        }
    }
}

impl ResourceLimiter for ExtensionMemoryLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        if desired > self.max_bytes {
            log::warn!(
                "Extension attempted to exceed declared RAM limit: desired {} bytes > limit {} bytes",
                desired,
                self.max_bytes
            );
            return Ok(false); // Memory growth rejected cleanly by Wasmtime
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        if desired > self.max_table_elements {
            log::warn!(
                "Extension attempted to exceed table limit: desired {} elements > limit {}",
                desired,
                self.max_table_elements
            );
            return Ok(false);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_limiter_clamps_growth() {
        let mut limiter = ExtensionMemoryLimiter::from_mb(64); // 64 MB
        let within_limit = 32 * 1024 * 1024;
        let exceeds_limit = 128 * 1024 * 1024;

        assert_eq!(
            limiter.memory_growing(0, within_limit, None).unwrap(),
            true
        );
        assert_eq!(
            limiter.memory_growing(within_limit, exceeds_limit, None).unwrap(),
            false
        );
    }
}
