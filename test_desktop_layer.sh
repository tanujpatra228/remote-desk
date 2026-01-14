#!/bin/bash
# Test script for desktop layer functionality
# Run this after installing: sudo apt-get install -y libxcb-randr0-dev

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  RemoteDesk - Desktop Layer Test Suite                    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if dependencies are installed
echo "Checking system dependencies..."
if dpkg -l | grep -q "libxcb-randr0-dev"; then
    echo "✓ libxcb-randr0-dev is installed"
else
    echo "✗ libxcb-randr0-dev is NOT installed"
    echo ""
    echo "Please install it with:"
    echo "  sudo apt-get install -y libxcb-randr0-dev"
    echo ""
    exit 1
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo " Phase 1: Build Project"
echo "════════════════════════════════════════════════════════════"
echo ""

cargo build --lib
echo "✓ Library built successfully"

echo ""
echo "════════════════════════════════════════════════════════════"
echo " Phase 2: Run Unit Tests"
echo "════════════════════════════════════════════════════════════"
echo ""

echo "Running desktop type tests..."
cargo test --lib desktop::types -- --nocapture

echo ""
echo "Running encoder tests..."
cargo test --lib desktop::encoder -- --nocapture

echo ""
echo "Running capture tests..."
cargo test --lib desktop::capture -- --nocapture

echo ""
echo "════════════════════════════════════════════════════════════"
echo " Phase 3: Run All Library Tests"
echo "════════════════════════════════════════════════════════════"
echo ""

cargo test --lib -- --test-threads=1

echo ""
echo "════════════════════════════════════════════════════════════"
echo " Phase 4: Build Binary"
echo "════════════════════════════════════════════════════════════"
echo ""

cargo build
echo "✓ Binary built successfully"

echo ""
echo "════════════════════════════════════════════════════════════"
echo " Phase 5: Test Summary"
echo "════════════════════════════════════════════════════════════"
echo ""

TEST_COUNT=$(cargo test --lib 2>&1 | grep -E "test result:" | head -1 | grep -oP '\d+(?= passed)')
echo "✓ All $TEST_COUNT tests passed"
echo "✓ Desktop layer is working correctly"
echo ""
echo "You can now run the application with:"
echo "  cargo run"
echo ""
