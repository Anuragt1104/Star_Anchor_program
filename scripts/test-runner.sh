#!/bin/bash

# Honorary Quote Fee Program - End-to-End Test Runner
# This script sets up a local Solana validator and runs comprehensive tests

set -e

echo "🚀 Starting Honorary Quote Fee End-to-End Test Suite"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VALIDATOR_PORT=8899
PROGRAM_ID="11111111111111111111111111111111"

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

# Function to check if port is in use
check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null ; then
        return 0
    else
        return 1
    fi
}

# Function to wait for validator to be ready
wait_for_validator() {
    print_status "Waiting for validator to start..."
    local retries=30
    local count=0

    while ! check_port $VALIDATOR_PORT; do
        if [ $count -ge $retries ]; then
            print_error "Validator failed to start after $retries attempts"
            exit 1
        fi

        sleep 2
        count=$((count + 1))
        echo -n "."
    done

    echo ""
    print_success "Validator is ready on port $VALIDATOR_PORT"
}

# Function to setup test environment
setup_test_env() {
    print_status "Setting up test environment..."

    # Install Node.js dependencies if needed
    if [ ! -d "node_modules" ]; then
        print_status "Installing Node.js dependencies..."
        npm install
    fi

    # Build the program
    print_status "Building Anchor program..."
    anchor build

    print_success "Test environment setup complete"
}

# Function to start local validator
start_validator() {
    print_status "Starting local Solana validator..."

    # Kill any existing validator
    pkill -f solana-test-validator || true
    sleep 2

    # Start validator in background
    solana-test-validator \
        --reset \
        --quiet \
        --bpf-program $PROGRAM_ID target/deploy/honorary_quote_fee.so \
        > validator.log 2>&1 &

    VALIDATOR_PID=$!
    echo $VALIDATOR_PID > validator.pid

    # Wait for validator to be ready
    wait_for_validator

    print_success "Local validator started (PID: $VALIDATOR_PID)"
}

# Function to run tests
run_tests() {
    print_status "Running end-to-end tests..."

    # Set environment variables for tests
    export ANCHOR_PROVIDER_URL="http://localhost:$VALIDATOR_PORT"
    export ANCHOR_WALLET="~/.config/solana/id.json"

    # Run the test suite
    npm test

    local test_exit_code=$?
    return $test_exit_code
}

# Function to cleanup
cleanup() {
    print_status "Cleaning up test environment..."

    # Kill validator if running
    if [ -f "validator.pid" ]; then
        VALIDATOR_PID=$(cat validator.pid)
        if kill -0 $VALIDATOR_PID 2>/dev/null; then
            print_status "Stopping validator (PID: $VALIDATOR_PID)"
            kill $VALIDATOR_PID
            wait $VALIDATOR_PID 2>/dev/null || true
        fi
        rm -f validator.pid
    fi

    # Kill any remaining validators
    pkill -f solana-test-validator || true

    # Clean up log files
    rm -f validator.log

    print_success "Cleanup complete"
}

# Function to show usage
show_usage() {
    echo "Honorary Quote Fee Program - Test Runner"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --setup-only     Only setup the environment, don't run tests"
    echo "  --no-cleanup     Don't cleanup after tests"
    echo "  --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                    # Run full test suite with cleanup"
    echo "  $0 --setup-only      # Only setup environment"
    echo "  $0 --no-cleanup      # Run tests without cleanup"
}

# Parse command line arguments
SETUP_ONLY=false
NO_CLEANUP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --setup-only)
            SETUP_ONLY=true
            shift
            ;;
        --no-cleanup)
            NO_CLEANUP=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    trap cleanup EXIT

    print_status "Honorary Quote Fee Program - End-to-End Test Runner"
    print_status "=================================================="

    # Setup test environment
    setup_test_env

    if [ "$SETUP_ONLY" = true ]; then
        print_success "Setup complete. Run with --no-cleanup to start testing."
        exit 0
    fi

    # Start validator
    start_validator

    # Run tests
    if run_tests; then
        print_success "All tests passed! 🎉"
        exit 0
    else
        print_error "Tests failed! ❌"
        exit 1
    fi
}

# Run main function
main "$@"
