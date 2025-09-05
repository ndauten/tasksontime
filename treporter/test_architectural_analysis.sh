#!/bin/bash

# Test script for architectural analysis feature
# This demonstrates how to generate comprehensive architectural analysis reports

set -e

echo "🏗️  Testing Architectural Analysis Feature"
echo "=========================================="

# Colors for output
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
    print_error "Please run this script from the chronopulse directory"
    exit 1
fi

# Build the project
print_status "Building chronopulse..."
cargo build --release

# Check if Q2 data files exist
Q2_DATA_FILES=(
    "../spear-hq/spear-hq.wiki/reports/2025-Q2/2025-04/2025-04-git-events.json"
    "../spear-hq/spear-hq.wiki/reports/2025-Q2/2025-05/2025-05-git-events.json"
    "../spear-hq/spear-hq.wiki/reports/2025-Q2/2025-06/2025-06-git-events.json"
)

echo ""
print_status "Checking for Q2 data files..."
AVAILABLE_FILES=()
for file in "${Q2_DATA_FILES[@]}"; do
    if [ -f "$file" ]; then
        print_success "Found: $file"
        AVAILABLE_FILES+=("$file")
    else
        print_warning "Missing: $file"
    fi
done

if [ ${#AVAILABLE_FILES[@]} -eq 0 ]; then
    print_error "No Q2 data files found. Please collect data first."
    exit 1
fi

# Generate architectural analysis using available data
echo ""
print_status "Generating architectural analysis report..."
echo "Using data files: ${AVAILABLE_FILES[*]}"

# Run the architectural analysis command
if ./target/release/chronopulse architectural-analysis --data-files "${AVAILABLE_FILES[@]}"; then
    print_success "Architectural analysis completed successfully!"
else
    print_error "Failed to generate architectural analysis"
    exit 1
fi

# Check for output files
echo ""
print_status "Checking generated output files..."
OUTPUT_DIR="output"
if [ -d "$OUTPUT_DIR" ]; then
    ANALYSIS_FILES=$(find "$OUTPUT_DIR" -name "*architectural_analysis*" -type f)
    if [ -n "$ANALYSIS_FILES" ]; then
        print_success "Generated architectural analysis files:"
        echo "$ANALYSIS_FILES" | while read -r file; do
            echo "  📄 $file"
            echo "     Size: $(wc -c < "$file") bytes"
            echo "     Lines: $(wc -l < "$file") lines"
        done
    else
        print_warning "No architectural analysis files found in output directory"
    fi
else
    print_warning "Output directory not found"
fi

# Show usage examples
echo ""
print_status "Usage Examples:"
echo "1. Generate from existing data files:"
echo "   ./target/release/chronopulse architectural-analysis --data-files file1.json file2.json file3.json"
echo ""
echo "2. Generate from single data file:"
echo "   ./target/release/chronopulse architectural-analysis --data-file collected_data.json"
echo ""
echo "3. Generate with fresh data collection:"
echo "   ./target/release/chronopulse architectural-analysis --from 2025-04-01 --to 2025-06-30"
echo ""

# Show template information
echo ""
print_status "Available Templates:"
echo "1. Basic template: templates/architectural-analysis.md"
echo "2. Advanced template: templates/architectural-analysis-advanced.md"
echo "3. Custom prompts: prompts/architectural-analysis.toml"
echo ""

print_success "Architectural analysis test completed!"
print_status "Review the generated report for comprehensive technical insights."
