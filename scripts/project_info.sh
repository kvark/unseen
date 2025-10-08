#!/usr/bin/env bash

# Project information and overview script for Unseen Vulkan Layer

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}📋 Unseen Vulkan Layer - Project Overview${NC}"
echo "============================================="
echo

# Project information
echo -e "${YELLOW}📖 Project Information${NC}"
echo "Project Name: Unseen Vulkan Layer"
echo "Description:  A Vulkan layer for capturing frames in headless environments"
echo "Language:     Rust (library) + C (tests/examples)"
echo "License:      MIT OR Apache-2.0"
echo

# Directory structure
echo -e "${YELLOW}📁 Directory Structure${NC}"
echo "."
echo "├── src/                     # Rust library source"
echo "├── examples/                # Demonstration programs"
echo "│   ├── c/                  # C example programs"
echo "│   ├── demo.sh             # Main demo script"
echo "│   └── frame_capture_demo.sh # Detailed demo"
echo "├── tests/                   # Test programs"
echo "│   └── c/                  # C test programs"
echo "├── scripts/                 # Build and utility scripts"
echo "├── target/                  # Build output (auto-generated)"
echo "│   ├── debug/bin/          # Debug C programs"
echo "│   └── release/            # Release builds"
echo "├── Cargo.toml              # Rust configuration"
echo "├── Makefile                # Build system"
echo "└── VkLayer_PRIVATE_unseen.json # Layer manifest"
echo

# Build status
echo -e "${YELLOW}🔨 Build Status${NC}"
if [ -f "target/release/libVkLayer_PRIVATE_unseen.so" ]; then
    lib_size=$(stat -f%z "target/release/libVkLayer_PRIVATE_unseen.so" 2>/dev/null || stat -c%s "target/release/libVkLayer_PRIVATE_unseen.so" 2>/dev/null)
    lib_size_human=$(echo $lib_size | numfmt --to=iec-i --suffix=B --format="%.1f" 2>/dev/null || echo "$lib_size bytes")
    echo -e "✅ Rust library: ${GREEN}BUILT${NC} ($lib_size_human)"
else
    echo -e "❌ Rust library: ${RED}NOT BUILT${NC}"
fi

if [ -d "target/release/bin" ] && [ "$(ls -A target/release/bin 2>/dev/null)" ]; then
    c_programs=$(ls target/release/bin 2>/dev/null | wc -l)
    echo -e "✅ C programs: ${GREEN}BUILT${NC} ($c_programs programs)"
    ls target/release/bin 2>/dev/null | sed 's/^/   - /'
else
    echo -e "❌ C programs: ${RED}NOT BUILT${NC}"
fi
echo

# Available commands
echo -e "${YELLOW}🚀 Available Commands${NC}"
echo "Build Commands:"
echo "  make              # Build everything (release)"
echo "  make debug        # Build in debug mode"
echo "  make c-programs   # Build only C programs"
echo "  make rust-library # Build only Rust library"
echo
echo "Test Commands:"
echo "  make test         # Run layer tests"
echo "  make demo         # Run frame capture demo"
echo "  make final-demo   # Run complete demo"
echo
echo "Utility Commands:"
echo "  make clean        # Clean build artifacts"
echo "  make info         # Show build information"
echo "  make help         # Show help"
echo
echo "Manual Commands:"
echo "  scripts/build_c_programs.sh [debug|release]"
echo "  scripts/test_layer.sh"
echo "  examples/frame_capture_demo.sh"
echo

# Environment setup
echo -e "${YELLOW}🌍 Environment Setup${NC}"
echo "Required Environment Variables:"
echo "  VK_LAYER_PATH=\$(pwd)                   # Path to layer files"
echo "  VK_INSTANCE_LAYERS=VK_LAYER_PRIVATE_unseen  # Enable the layer"
echo "  VK_UNSEEN_ENABLE=1                     # Enable frame capture"
echo "  VK_CAPTURE_OUTPUT_DIR=./captured_frames # Output directory"
echo
echo "Optional:"
echo "  RUST_LOG=info                          # Logging level"
echo

# Quick start
echo -e "${YELLOW}⚡ Quick Start${NC}"
echo "1. Build: make"
echo "2. Test:  make test"
echo "3. Demo:  make demo"
echo
echo "For detailed usage, see README.md"
echo

# Dependencies check
echo -e "${YELLOW}🔍 Dependencies${NC}"
command -v rustc >/dev/null 2>&1 && echo -e "✅ Rust: ${GREEN}$(rustc --version)${NC}" || echo -e "❌ Rust: ${RED}NOT FOUND${NC}"
command -v cargo >/dev/null 2>&1 && echo -e "✅ Cargo: ${GREEN}$(cargo --version)${NC}" || echo -e "❌ Cargo: ${RED}NOT FOUND${NC}"
command -v gcc >/dev/null 2>&1 && echo -e "✅ GCC: ${GREEN}$(gcc --version | head -1)${NC}" || echo -e "❌ GCC: ${RED}NOT FOUND${NC}"
command -v make >/dev/null 2>&1 && echo -e "✅ Make: ${GREEN}$(make --version | head -1)${NC}" || echo -e "⚠️ Make: ${YELLOW}NOT FOUND (optional)${NC}"

# Vulkan check
if command -v vulkaninfo >/dev/null 2>&1; then
    echo -e "✅ Vulkan: ${GREEN}AVAILABLE${NC}"
else
    echo -e "⚠️ Vulkan: ${YELLOW}vulkaninfo not found${NC}"
fi
echo

# File sizes
echo -e "${YELLOW}📊 Project Statistics${NC}"
total_rust_lines=$(find src -name "*.rs" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
total_c_lines=$(find examples tests -name "*.c" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
total_script_lines=$(find scripts examples -name "*.sh" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")

echo "Source Code:"
echo "  Rust:   $total_rust_lines lines"
echo "  C:      $total_c_lines lines"
echo "  Shell:  $total_script_lines lines"
echo

# Git info (if available)
if [ -d ".git" ]; then
    echo -e "${YELLOW}📝 Git Information${NC}"
    echo "Current branch: $(git branch --show-current 2>/dev/null || echo 'unknown')"
    echo "Last commit:    $(git log -1 --format='%h %s' 2>/dev/null || echo 'unknown')"
    echo "Status:         $(git status --porcelain 2>/dev/null | wc -l) files changed"
    echo
fi

echo -e "${GREEN}🎉 Project overview complete!${NC}"
echo "Ready to build and test the Unseen Vulkan Layer!"
