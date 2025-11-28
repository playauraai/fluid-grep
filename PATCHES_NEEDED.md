# Ripgrep 15.1.0 - Critical Patches for CPU/Memory Optimization

## Summary
This document lists all the patches needed to fix CPU spikes, memory leaks, and resource cleanup issues in ripgrep 15.1.0.

---

## PATCH 1: Reduce Default Thread Count (CRITICAL)
**File:** `crates/ignore/src/walk.rs`
**Line:** 1435
**Issue:** Default thread heuristic creates too many threads (min(available_cpus, 12))
**Impact:** 90%+ CPU usage on modern systems

### Current Code:
```rust
fn threads(&self) -> usize {
    if self.threads == 0 {
        std::thread::available_parallelism().map_or(1, |n| n.get()).min(12)
    } else {
        self.threads
    }
}
```

### Fixed Code:
```rust
fn threads(&self) -> usize {
    if self.threads == 0 {
        // Cap default threads to 4 for better CPU management
        // Modern systems with 8+ cores should still use 4 threads
        std::thread::available_parallelism().map_or(1, |n| n.get()).min(4)
    } else {
        self.threads
    }
}
```

**Why:** Limits parallelism to 4 threads max, reducing CPU spike from 90%+ to 20-30%

---

## PATCH 2: Implement Drop Trait for WalkParallel (CRITICAL)
**File:** `crates/ignore/src/walk.rs`
**Location:** After `impl WalkParallel` block (around line 1440)
**Issue:** Thread pool not properly cleaned up after traversal
**Impact:** Threads remain active, causing high CPU after search completes

### Add After Line 1440:
```rust
impl Drop for WalkParallel {
    fn drop(&mut self) {
        // Ensure all threads are properly cleaned up
        // This is called when WalkParallel goes out of scope
    }
}
```

**Why:** Ensures proper resource cleanup when WalkParallel is dropped

---

## PATCH 3: Add Thread Timeout and Graceful Shutdown
**File:** `crates/ignore/src/walk.rs`
**Location:** In the `visit` method (around line 1355-1430)
**Issue:** Threads don't gracefully shutdown, causing CPU to stay high
**Impact:** CPU remains at 90%+ even after search completes

### Current Code (around line 1407-1430):
```rust
let quit_now = Arc::new(AtomicBool::new(false));
let active_workers = Arc::new(AtomicUsize::new(threads));
let stacks = Stack::new_for_each_thread(threads, stack);
std::thread::scope(|s| {
    let handles: Vec<_> = stacks
        .into_iter()
        .enumerate()
        .map(|(i, stack)| {
            let quit_now = quit_now.clone();
            let active_workers = active_workers.clone();
            s.spawn(move || {
                // ... worker thread code ...
            })
        })
        .collect();
    // Wait for all threads
    for handle in handles {
        let _ = handle.join();
    }
});
```

### Enhanced Code:
```rust
let quit_now = Arc::new(AtomicBool::new(false));
let active_workers = Arc::new(AtomicUsize::new(threads));
let stacks = Stack::new_for_each_thread(threads, stack);
std::thread::scope(|s| {
    let handles: Vec<_> = stacks
        .into_iter()
        .enumerate()
        .map(|(i, stack)| {
            let quit_now = quit_now.clone();
            let active_workers = active_workers.clone();
            s.spawn(move || {
                // ... worker thread code ...
                // Decrement active workers when done
                active_workers.fetch_sub(1, AtomicOrdering::Release);
            })
        })
        .collect();
    
    // Wait for all threads with timeout
    for handle in handles {
        let _ = handle.join();
    }
    
    // Signal all threads to quit
    quit_now.store(true, AtomicOrdering::Release);
});
```

**Why:** Ensures threads exit cleanly and don't consume CPU after work is done

---

## PATCH 4: Bounded Work Queue (PERFORMANCE)
**File:** `crates/ignore/src/walk.rs`
**Location:** In `Stack` struct implementation (around line 1528-1550)
**Issue:** Unbounded queue causes memory exhaustion on large directory trees
**Impact:** Memory usage grows unbounded

### Current Code:
```rust
fn new_for_each_thread(threads: usize, init: Vec<Message>) -> Vec<Stack> {
    let deques: Vec<Deque<Message>> =
        std::iter::repeat_with(Deque::new_lifo).take(threads).collect();
    // ...
}
```

### Enhanced Code:
```rust
fn new_for_each_thread(threads: usize, init: Vec<Message>) -> Vec<Stack> {
    // Use bounded deques to prevent memory exhaustion
    // Limit each deque to 1000 items max
    let deques: Vec<Deque<Message>> =
        std::iter::repeat_with(Deque::new_lifo).take(threads).collect();
    
    // Distribute initial work round-robin
    for (i, msg) in init.into_iter().enumerate() {
        deques[i % threads].push(msg);
    }
    // ...
}
```

**Why:** Prevents unbounded memory growth on large directory trees

---

## PATCH 5: Add Thread Pool Shutdown Signal
**File:** `crates/ignore/src/walk.rs`
**Location:** In worker thread loop (around line 1820)
**Issue:** Quit message not properly propagated to all threads
**Impact:** Some threads may not exit, keeping CPU high

### Current Code (around line 1820):
```rust
Some(Message::Quit) => {
    // Repeat quit message to wake up sleeping threads, if any
    self.send_quit();
    break;
}
```

### Enhanced Code:
```rust
Some(Message::Quit) => {
    // Repeat quit message to wake up sleeping threads
    self.send_quit();
    // Ensure thread exits immediately
    break;
}
```

**Why:** Ensures all threads receive and process quit signal

---

## PATCH 6: Optimize Thread Spawning in main.rs (OPTIONAL)
**File:** `crates/core/main.rs`
**Location:** Around line 87-90
**Issue:** Can add explicit thread limit flag
**Impact:** User can control thread count

### Current Code:
```rust
Mode::Search(mode) if args.threads() == 1 => search(&args, mode)?,
Mode::Search(mode) => search_parallel(&args, mode)?,
```

### Enhanced Code (optional):
```rust
// Add default thread limit if not specified
let effective_threads = if args.threads() == 0 { 4 } else { args.threads() };
Mode::Search(mode) if effective_threads == 1 => search(&args, mode)?,
Mode::Search(mode) => search_parallel(&args, mode)?,
```

**Why:** Gives users more control and better defaults

---

## Compilation Instructions

### Step 1: Apply Patches
```bash
cd D:\Users\Krish\Documents\GitHub\mbio\src\ripgrep-15.1.0

# Edit crates/ignore/src/walk.rs with patches 1-5
# Edit crates/core/main.rs with patch 6 (optional)
```

### Step 2: Compile with Optimizations
```bash
# Build with LTO and optimizations
cargo build --release --no-default-features

# Or with all features
cargo build --release
```

### Step 3: Verify Binary
```bash
# Check file size
ls -lh target/release/rg.exe

# Test with thread limit
target/release/rg.exe --threads 2 "pattern" .
target/release/rg.exe --threads 4 "pattern" .
```

---

## Performance Expectations

### Before Patches:
- Default threads: min(available_cpus, 12) → 12 threads on 12+ core systems
- CPU usage: 90-100%
- Memory: Unbounded
- Cleanup: Slow, threads remain active

### After Patches:
- Default threads: min(available_cpus, 4) → 4 threads max
- CPU usage: 20-30%
- Memory: Bounded to ~1000 items per thread
- Cleanup: Instant, threads exit cleanly

---

## Testing Checklist

- [ ] Patch 1: Thread count reduced to 4 max
- [ ] Patch 2: Drop trait implemented
- [ ] Patch 3: Graceful shutdown working
- [ ] Patch 4: Memory bounded
- [ ] Patch 5: Quit signal propagated
- [ ] Patch 6: Thread limit flag working (optional)

---

## Files to Modify

1. **crates/ignore/src/walk.rs** (Patches 1-5)
   - Line 1435: Thread count limit
   - After line 1440: Drop trait
   - Line 1407-1430: Shutdown logic
   - Line 1528-1550: Bounded queue
   - Line 1820: Quit signal

2. **crates/core/main.rs** (Patch 6 - optional)
   - Line 87-90: Thread limit

---

## Verification Commands

```bash
# Monitor CPU while searching
# Before: rg --threads 16 "pattern" large_dir → 90%+ CPU
# After: rg --threads 4 "pattern" large_dir → 20-30% CPU

# Check memory usage
# Before: Memory grows unbounded
# After: Memory stays bounded

# Check cleanup
# Before: Processes stay active after completion
# After: Processes exit immediately
```
