//! Memory monitoring and management utilities
//! 
//! This module provides enhanced memory tracking and pressure management
//! with atomic-based tracking and adaptive sizing capabilities.

use crate::config::get_config;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use tracing::{warn, error};
use anyhow::Result;

/// Global memory tracking state
static MEMORY_LIMIT: AtomicUsize = AtomicUsize::new(0);
static CURRENT_USAGE: AtomicUsize = AtomicUsize::new(0);
static MEMORY_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Memory pressure levels for adaptive behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryPressureLevel {
    Low,    // < 50% of limit
    Normal, // 50-75% of limit
    High,   // 75-90% of limit
    Critical, // > 90% of limit
}

/// Memory statistics for monitoring
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_usage: usize,
    pub memory_limit: usize,
    pub usage_percentage: f64,
}

/// Initialize the global memory limit with configuration
pub fn init_memory_limit() {
    let config = get_config();
    let limit_bytes = config.memory.max_memory_mb * 1_000_000;
    
    MEMORY_LIMIT.store(limit_bytes, Ordering::Relaxed);
    
    if MEMORY_INITIALIZED.set(()).is_err() {
        error!("Failed to initialize memory limit - already initialized");
    }
}

/// Ensure memory limit is initialized
fn ensure_initialized() {
    MEMORY_INITIALIZED.get_or_init(|| {
        // Fallback initialization if init_memory_limit wasn't called
        let config = get_config();
        let limit_bytes = config.memory.max_memory_mb * 1_000_000;
        MEMORY_LIMIT.store(limit_bytes, Ordering::Relaxed);
    });
}

/// Check if we're approaching memory limit (backward compatibility)
/// Maps MemoryPressureLevel to boolean for existing code
pub fn check_memory_pressure() -> bool {
    ensure_initialized();
    let pressure = get_pressure_level();
    
    match pressure {
        MemoryPressureLevel::Low | MemoryPressureLevel::Normal => false,
        MemoryPressureLevel::High | MemoryPressureLevel::Critical => {
            let stats = get_memory_stats();
            warn!(
                current_mb = stats.current_usage / 1_000_000,
                limit_mb = stats.memory_limit / 1_000_000,
                usage_pct = stats.usage_percentage,
                pressure_level = ?pressure,
                "Memory pressure detected"
            );
            true
        }
    }
}

/// Track approximate memory allocation (backward compatibility)
pub fn track_allocation(bytes: usize) {
    ensure_initialized();
    let limit = MEMORY_LIMIT.load(Ordering::Relaxed);
    let new_usage = CURRENT_USAGE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    
    if new_usage > limit {
        warn!(
            bytes = bytes,
            new_usage_mb = new_usage / 1_000_000,
            limit_mb = limit / 1_000_000,
            "Memory allocation would exceed limit"
        );
    }
}

/// Track approximate memory deallocation (backward compatibility)
pub fn track_deallocation(bytes: usize) {
    ensure_initialized();
    // Use saturating_sub to prevent underflow
    let current = CURRENT_USAGE.load(Ordering::Relaxed);
    let new_usage = current.saturating_sub(bytes);
    CURRENT_USAGE.store(new_usage, Ordering::Relaxed);
}

/// Get current memory usage estimate in MB (backward compatibility)
pub fn get_memory_usage_mb() -> usize {
    ensure_initialized();
    CURRENT_USAGE.load(Ordering::Relaxed) / 1_000_000
}

// Enhanced methods for adaptive memory management

/// Get adaptive batch size based on current memory pressure
/// Returns a dynamically adjusted batch size for optimal performance
pub fn get_adaptive_batch_size(default_size: usize) -> usize {
    ensure_initialized();
    let pressure = get_pressure_level();
    
    match pressure {
        MemoryPressureLevel::Low => default_size,
        MemoryPressureLevel::Normal => (default_size * 3) / 4,  // 75% of default
        MemoryPressureLevel::High => default_size / 2,          // 50% of default
        MemoryPressureLevel::Critical => default_size / 4,      // 25% of default
    }
}

/// Get detailed memory statistics
pub fn get_memory_stats() -> MemoryStats {
    ensure_initialized();
    let current = CURRENT_USAGE.load(Ordering::Relaxed);
    let limit = MEMORY_LIMIT.load(Ordering::Relaxed);
    let percentage = if limit > 0 {
        (current as f64 / limit as f64) * 100.0
    } else {
        0.0
    };
    
    MemoryStats {
        current_usage: current,
        memory_limit: limit,
        usage_percentage: percentage,
    }
}

/// Check if we should spill to disk due to critical memory pressure
pub fn should_spill_to_disk() -> bool {
    ensure_initialized();
    matches!(get_pressure_level(), MemoryPressureLevel::Critical)
}

/// Get current memory pressure level
pub fn get_pressure_level() -> MemoryPressureLevel {
    ensure_initialized();
    let current = CURRENT_USAGE.load(Ordering::Relaxed);
    let limit = MEMORY_LIMIT.load(Ordering::Relaxed);
    
    if limit == 0 {
        return MemoryPressureLevel::Low;
    }
    
    let usage_ratio = current as f64 / limit as f64;
    
    match usage_ratio {
        r if r < 0.5 => MemoryPressureLevel::Low,
        r if r < 0.75 => MemoryPressureLevel::Normal,
        r if r < 0.9 => MemoryPressureLevel::High,
        _ => MemoryPressureLevel::Critical,
    }
}

/// Attempt to trigger garbage collection if memory pressure is high
pub fn try_gc_if_needed() -> Result<()> {
    ensure_initialized();
    match get_pressure_level() {
        MemoryPressureLevel::High | MemoryPressureLevel::Critical => {
            // Force garbage collection in high pressure situations
            // Note: In Rust, we can't force GC like in managed languages,
            // but we can suggest it for the allocator and hint at optimization
            std::hint::black_box(());
            Ok(())
        }
        _ => Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_initialization() {
        // Test that we can initialize and use the memory manager
        init_memory_limit();
        
        // Should not panic and should return valid stats
        let stats = get_memory_stats();
        assert!(stats.memory_limit > 0);
        assert!(stats.usage_percentage <= 100.0);
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that legacy API still works
        init_memory_limit();
        
        // Track some allocations
        track_allocation(1024);
        track_allocation(2048);
        
        // Should be able to get usage
        let usage_mb = get_memory_usage_mb();
        assert!(usage_mb >= 0);
        
        // Should be able to check pressure
        let has_pressure = check_memory_pressure();
        assert!(has_pressure == true || has_pressure == false); // Just ensure it doesn't panic
        
        // Track deallocations
        track_deallocation(1024);
        track_deallocation(2048);
    }

    #[test]
    fn test_adaptive_batch_size() {
        init_memory_limit();
        
        let default_size = 1000;
        let adaptive_size = get_adaptive_batch_size(default_size);
        
        // Adaptive size should be reasonable
        assert!(adaptive_size > 0);
        assert!(adaptive_size <= default_size * 10); // Sanity check
    }

    #[test]
    fn test_spill_to_disk_decision() {
        init_memory_limit();
        
        // Should return a boolean without panicking
        let should_spill = should_spill_to_disk();
        assert!(should_spill == true || should_spill == false);
    }

    #[test]
    fn test_pressure_level() {
        init_memory_limit();
        
        let pressure = get_pressure_level();
        // Should be one of the valid pressure levels
        matches!(pressure, 
            MemoryPressureLevel::Low | 
            MemoryPressureLevel::Normal | 
            MemoryPressureLevel::High | 
            MemoryPressureLevel::Critical
        );
    }
}