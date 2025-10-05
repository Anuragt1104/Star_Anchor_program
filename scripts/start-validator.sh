#!/bin/bash

# Start local validator with required programs for testing
# This script sets up a local validator with CP-AMM and Streamflow programs

set -e

echo "🚀 Starting local Solana test validator with required programs..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Program IDs
DAMM_PROGRAM_ID="675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
STREAMFLOW_PROGRAM_ID="strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m"

# Check if program binaries exist
DAMM_SO="./programs/damm-v2/damm_v2.so"
STREAMFLOW_SO="./programs/streamflow/streamflow.so"

if [ ! -f "$DAMM_SO" ]; then
    echo -e "${YELLOW}⚠️  Warning: DAMM v2 program binary not found at $DAMM_SO${NC}"
    echo "   You need to download or build the Meteora DAMM v2 program"
    echo "   The validator will start without it, but integration tests will fail"
fi

if [ ! -f "$STREAMFLOW_SO" ]; then
    echo -e "${YELLOW}⚠️  Warning: Streamflow program binary not found at $STREAMFLOW_SO${NC}"
    echo "   You need to download or build the Streamflow program"
    echo "   The validator will start without it, but integration tests will fail"
fi

# Kill any existing test validator
pkill -f "solana-test-validator" || true
sleep 2

echo -e "${GREEN}✓${NC} Starting validator..."

# Build the command
CMD="solana-test-validator"

# Add DAMM program if binary exists
if [ -f "$DAMM_SO" ]; then
    CMD="$CMD --bpf-program $DAMM_PROGRAM_ID $DAMM_SO"
    echo -e "${GREEN}✓${NC} Loading DAMM v2 program: $DAMM_PROGRAM_ID"
fi

# Add Streamflow program if binary exists
if [ -f "$STREAMFLOW_SO" ]; then
    CMD="$CMD --bpf-program $STREAMFLOW_PROGRAM_ID $STREAMFLOW_SO"
    echo -e "${GREEN}✓${NC} Loading Streamflow program: $STREAMFLOW_PROGRAM_ID"
fi

# Add additional configuration
CMD="$CMD --reset --quiet"

# Start the validator in background
eval "$CMD" &
VALIDATOR_PID=$!

echo -e "${GREEN}✓${NC} Validator started (PID: $VALIDATOR_PID)"
echo ""
echo "Waiting for validator to be ready..."

# Wait for validator to be ready
for i in {1..30}; do
    if solana cluster-version &> /dev/null; then
        echo -e "${GREEN}✓${NC} Validator is ready!"
        echo ""
        echo "📍 RPC URL: http://localhost:8899"
        echo "🔑 Default accounts funded"
        echo ""
        
        if [ -f "$DAMM_SO" ] && [ -f "$STREAMFLOW_SO" ]; then
            echo -e "${GREEN}✅ All programs loaded successfully!${NC}"
            echo ""
            echo "You can now run: npm test"
        else
            echo -e "${YELLOW}⚠️  Some programs are missing. Tests may fail.${NC}"
            echo ""
            echo "To get program binaries:"
            echo "1. Meteora DAMM v2: Contact Meteora team or build from source"
            echo "2. Streamflow: Contact Streamflow team or build from source"
        fi
        echo ""
        echo "Press Ctrl+C to stop the validator"
        
        # Keep the validator running
        wait $VALIDATOR_PID
        exit 0
    fi
    sleep 1
done

echo -e "${RED}❌ Failed to start validator${NC}"
kill $VALIDATOR_PID 2>/dev/null || true
exit 1

