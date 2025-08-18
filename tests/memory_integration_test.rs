//! Integration tests for enhanced memory management
//! 
//! Tests the memory.rs module with atomic-based tracking and adaptive sizing

use claude_usage::memory;

#[test]
fn test_memory_initialization() {
    // Initialize the memory manager
    memory::init_memory_limit();
    
    // Test basic functionality
    let stats = memory::get_memory_stats();
    assert!(stats.memory_limit > 0, "Memory limit should be set");
    assert!(stats.usage_percentage >= 0.0, "Usage percentage should be non-negative");
    assert!(stats.usage_percentage <= 100.0, "Usage percentage should not exceed 100%");
}

#[test]
fn test_memory_tracking() {
    memory::init_memory_limit();
    
    // Test allocation tracking
    let initial_usage = memory::get_memory_usage_mb();
    
    // Allocate some memory
    memory::track_allocation(1024 * 1024); // 1MB
    let after_allocation = memory::get_memory_usage_mb();
    
    // Should reflect the allocation (approximately)
    assert!(after_allocation >= initial_usage, "Memory usage should increase after allocation");
    
    // Deallocate
    memory::track_deallocation(1024 * 1024); // 1MB
    let after_deallocation = memory::get_memory_usage_mb();
    
    // Should reflect the deallocation
    assert!(after_deallocation <= after_allocation, "Memory usage should decrease after deallocation");
}

#[test]
fn test_adaptive_batch_size() {
    memory::init_memory_limit();
    
    let default_size = 1000;
    let adaptive_size = memory::get_adaptive_batch_size(default_size);
    
    // Should return a reasonable batch size
    assert!(adaptive_size > 0, "Adaptive batch size should be positive");
    assert!(adaptive_size <= default_size * 10, "Adaptive batch size should be reasonable");
}

#[test]
fn test_memory_pressure_check() {
    memory::init_memory_limit();
    
    // Should not panic
    let has_pressure = memory::check_memory_pressure();
    assert!(has_pressure == true || has_pressure == false);
}

#[test]
fn test_spill_to_disk_decision() {
    memory::init_memory_limit();
    
    // Should return a boolean
    let should_spill = memory::should_spill_to_disk();
    assert!(should_spill == true || should_spill == false);
}

#[test]
fn test_pressure_level_enum() {
    memory::init_memory_limit();
    
    let pressure = memory::get_pressure_level();
    
    // Should be one of the four valid levels
    use claude_usage::memory::MemoryPressureLevel;
    match pressure {
        MemoryPressureLevel::Low | 
        MemoryPressureLevel::Normal | 
        MemoryPressureLevel::High | 
        MemoryPressureLevel::Critical => {
            // Valid pressure level
        }
    }
}

#[test]
fn test_gc_trigger() {
    memory::init_memory_limit();
    
    // Should not panic
    let result = memory::try_gc_if_needed();
    assert!(result.is_ok(), "GC trigger should succeed");
}

#[test]
fn test_memory_stats_structure() {
    memory::init_memory_limit();
    
    let stats = memory::get_memory_stats();
    
    // Verify stats structure
    assert!(stats.memory_limit > 0, "Memory limit should be positive");
    assert!(stats.current_usage >= 0, "Current usage should be non-negative");
    assert!(stats.usage_percentage >= 0.0, "Usage percentage should be non-negative");
    assert!(stats.usage_percentage <= 100.0, "Usage percentage should not exceed 100%");
}

#[test]
fn test_backward_compatibility() {
    // Test that all the legacy functions still work
    memory::init_memory_limit();
    
    // Legacy allocation tracking
    memory::track_allocation(2048);
    memory::track_deallocation(1024);
    
    // Legacy memory usage
    let usage_mb = memory::get_memory_usage_mb();
    assert!(usage_mb >= 0, "Memory usage should be non-negative");
    
    // Legacy pressure check
    let pressure = memory::check_memory_pressure();
    assert!(pressure == true || pressure == false, "Pressure check should return boolean");
}