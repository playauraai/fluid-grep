#!/bin/bash
# Benchmark: Original vs Fluid Search Mode

RG="./target/release/rg"
TEST_DIR="."
PATTERN="fn"
ITERATIONS=5

echo "=========================================="
echo "Ripgrep Benchmark: Original vs Fluid"
echo "=========================================="
echo ""

# Create test data if needed
echo "Creating test data..."
mkdir -p benchmark_data
for i in {1..100}; do
    cat > "benchmark_data/test_$i.rs" << 'EOF'
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("Division by zero");
    }
    a / b
}

fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn factorial(n: u32) -> u32 {
    match n {
        0 => 1,
        _ => n * factorial(n - 1),
    }
}
EOF
done

echo "Test data created (100 files)"
echo ""

# Benchmark Original Mode
echo "=========================================="
echo "ORIGINAL MODE (Regex-based)"
echo "=========================================="
echo "Pattern: '$PATTERN'"
echo "Iterations: $ITERATIONS"
echo ""

total_time=0
for i in $(seq 1 $ITERATIONS); do
    start=$(date +%s%N)
    $RG "$PATTERN" benchmark_data > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    total_time=$(( total_time + elapsed ))
    echo "Run $i: ${elapsed}ms"
done

avg_original=$(( total_time / ITERATIONS ))
echo "Average: ${avg_original}ms"
echo ""

# Benchmark Fluid Mode
echo "=========================================="
echo "FLUID MODE (Heuristic-based)"
echo "=========================================="
echo "Pattern: '$PATTERN'"
echo "Iterations: $ITERATIONS"
echo ""

total_time=0
for i in $(seq 1 $ITERATIONS); do
    start=$(date +%s%N)
    $RG --fluid "$PATTERN" benchmark_data > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    total_time=$(( total_time + elapsed ))
    echo "Run $i: ${elapsed}ms"
done

avg_fluid=$(( total_time / ITERATIONS ))
echo "Average: ${avg_fluid}ms"
echo ""

# Calculate improvement
if [ $avg_original -gt 0 ]; then
    improvement=$(( (avg_original - avg_fluid) * 100 / avg_original ))
    echo "=========================================="
    echo "RESULTS"
    echo "=========================================="
    echo "Original Mode: ${avg_original}ms"
    echo "Fluid Mode:    ${avg_fluid}ms"
    if [ $improvement -gt 0 ]; then
        echo "Improvement:   ${improvement}% faster"
    else
        improvement=$(( (avg_fluid - avg_original) * 100 / avg_original ))
        echo "Difference:    ${improvement}% slower (expected for heuristic mode)"
    fi
fi

echo ""
echo "Cleanup..."
rm -rf benchmark_data
echo "Done!"
