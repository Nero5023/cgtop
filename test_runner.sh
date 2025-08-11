#!/bin/bash

# Comprehensive test runner for cgtop
# Usage: ./test_runner.sh [test_type]
# test_type: unit, integration, property, bench, all (default)

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

# Default test type
TEST_TYPE=${1:-all}

print_status "Running tests for cgtop application"
print_status "Test type: $TEST_TYPE"
echo

# Unit Tests
run_unit_tests() {
    print_status "Running unit tests..."
    if cargo test --lib; then
        print_success "Unit tests passed!"
    else
        print_error "Unit tests failed!"
        return 1
    fi
    echo
}

# Integration Tests
run_integration_tests() {
    print_status "Running integration tests..."
    if cargo test --test integration_tests; then
        print_success "Integration tests passed!"
    else
        print_error "Integration tests failed!"
        return 1
    fi
    echo
}

# Property-based Tests
run_property_tests() {
    print_status "Running property-based tests..."
    if cargo test --test property_tests; then
        print_success "Property-based tests passed!"
    else
        print_error "Property-based tests failed!"
        return 1
    fi
    echo
}

# Tree State Tests
run_tree_tests() {
    print_status "Running tree state tests..."
    if cargo test --test tree_state_tests; then
        print_success "Tree state tests passed!"
    else
        print_error "Tree state tests failed!"
        return 1
    fi
    echo
}

# Collection Tests
run_collection_tests() {
    print_status "Running collection tests..."
    if cargo test --test collection_tests; then
        print_success "Collection tests passed!"
    else
        print_error "Collection tests failed!"
        return 1
    fi
    echo
}

# Benchmarks
run_benchmarks() {
    print_status "Running benchmarks..."
    if cargo bench --bench tree_benchmarks; then
        print_success "Benchmarks completed!"
        print_status "Benchmark results saved to target/criterion/"
    else
        print_error "Benchmarks failed!"
        return 1
    fi
    echo
}

# Code coverage (if available)
run_coverage() {
    print_status "Checking for code coverage tools..."
    if command -v cargo-tarpaulin &> /dev/null; then
        print_status "Running code coverage with tarpaulin..."
        cargo tarpaulin --out Html --output-dir target/coverage
        print_success "Coverage report generated in target/coverage/"
    else
        print_warning "cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
    fi
    echo
}

# Lint and format check
run_lint() {
    print_status "Running linting and format checks..."
    
    if command -v cargo-fmt &> /dev/null || cargo fmt --version &> /dev/null; then
        if cargo fmt -- --check; then
            print_success "Code formatting is correct!"
        else
            print_warning "Code needs formatting. Run: cargo fmt"
        fi
    else
        print_warning "cargo fmt not available"
    fi
    
    if command -v cargo-clippy &> /dev/null || cargo clippy --version &> /dev/null; then
        if cargo clippy -- -D warnings; then
            print_success "No clippy warnings found!"
        else
            print_warning "Clippy found issues. Please address them."
        fi
    else
        print_warning "cargo clippy not available"
    fi
    echo
}

# Main execution
case $TEST_TYPE in
    "unit")
        run_unit_tests
        ;;
    "integration")
        run_integration_tests
        ;;
    "property")
        run_property_tests
        ;;
    "tree")
        run_tree_tests
        ;;
    "collection")
        run_collection_tests
        ;;
    "bench")
        run_benchmarks
        ;;
    "coverage")
        run_coverage
        ;;
    "lint")
        run_lint
        ;;
    "all")
        print_status "Running comprehensive test suite..."
        echo
        
        # Track failures
        FAILED_TESTS=""
        
        if ! run_unit_tests; then
            FAILED_TESTS="$FAILED_TESTS unit"
        fi
        
        if ! run_tree_tests; then
            FAILED_TESTS="$FAILED_TESTS tree"
        fi
        
        if ! run_collection_tests; then
            FAILED_TESTS="$FAILED_TESTS collection"
        fi
        
        if ! run_integration_tests; then
            FAILED_TESTS="$FAILED_TESTS integration"
        fi
        
        if ! run_property_tests; then
            FAILED_TESTS="$FAILED_TESTS property"
        fi
        
        run_lint
        
        # Optional benchmarks and coverage
        print_status "Running optional performance and coverage analysis..."
        run_benchmarks || print_warning "Benchmarks failed but continuing..."
        run_coverage || print_warning "Coverage analysis failed but continuing..."
        
        # Final summary
        echo
        print_status "=== TEST SUMMARY ==="
        if [ -z "$FAILED_TESTS" ]; then
            print_success "All tests passed! ✨"
        else
            print_error "Failed test suites:$FAILED_TESTS"
            exit 1
        fi
        ;;
    *)
        print_error "Unknown test type: $TEST_TYPE"
        print_status "Available options: unit, integration, property, tree, collection, bench, coverage, lint, all"
        exit 1
        ;;
esac

print_success "Test execution completed!"