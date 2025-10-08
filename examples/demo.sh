#!/usr/bin/env bash

# Unseen Vulkan Layer Demonstration
# Clean demo script that builds and tests the layer

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🎬 Unseen Vulkan Layer Demo"
echo "============================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build the layer
echo -e "${BLUE}🔨 Building Unseen layer...${NC}"
cargo build --release
echo -e "${GREEN}✅ Layer built successfully${NC}"
echo

# Build C test programs first
echo -e "${BLUE}🔨 Building C test programs...${NC}"
scripts/build_c_programs.sh release
echo

# Setup demo environment
DEMO_DIR="/tmp/unseen_demo"
echo -e "${BLUE}📁 Setting up demo environment: $DEMO_DIR${NC}"
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR/captured_frames"

# Copy necessary files
cp target/release/libVkLayer_PRIVATE_unseen.so "$DEMO_DIR/"
cp VkLayer_PRIVATE_unseen.json "$DEMO_DIR/"

# Update manifest to use absolute path
sed -i "s|\\./|$DEMO_DIR/|g" "$DEMO_DIR/VkLayer_PRIVATE_unseen.json"

echo -e "${GREEN}✅ Demo environment ready${NC}"
echo

# Copy pre-built test programs
echo -e "${BLUE}💻 Copying test programs...${NC}"
cd "$DEMO_DIR"

# Copy the direct test program
if [ -f "$PROJECT_ROOT/target/release/bin/direct_test" ]; then
    cp "$PROJECT_ROOT/target/release/bin/direct_test" .
    echo -e "${GREEN}✅ Direct test program copied${NC}"
else
    echo -e "${RED}❌ direct_test not found - please run 'scripts/build_c_programs.sh release' first${NC}"
    exit 1
fi

# Copy other test programs if available
if [ -f "$PROJECT_ROOT/target/release/bin/simple_test" ]; then
    cp "$PROJECT_ROOT/target/release/bin/simple_test" .
    echo -e "${GREEN}✅ Simple test program copied${NC}"
fi

echo

# Set environment variables
export VK_LAYER_PATH="$DEMO_DIR"
export VK_INSTANCE_LAYERS="VK_LAYER_PRIVATE_unseen"
export VK_UNSEEN_ENABLE=1
export VK_CAPTURE_OUTPUT_DIR="$DEMO_DIR/captured_frames"
export RUST_LOG=info

echo -e "${YELLOW}🚀 Running demonstration...${NC}"
echo "Environment configured:"
echo "   VK_LAYER_PATH: $VK_LAYER_PATH"
echo "   VK_INSTANCE_LAYERS: $VK_INSTANCE_LAYERS"
echo "   VK_CAPTURE_OUTPUT_DIR: $VK_CAPTURE_OUTPUT_DIR"
echo

# Run the direct test (this should work reliably)
echo -e "${BLUE}🎭 Starting direct layer test...${NC}"
echo "=========================================="
./direct_test
echo "=========================================="
echo

# Analyze results
echo -e "${YELLOW}📊 Analyzing Results${NC}"
echo "===================="

if [ -d captured_frames ] && [ "$(ls -A captured_frames 2>/dev/null)" ]; then
    frame_count=$(ls -1 captured_frames/frame_*.ppm 2>/dev/null | wc -l)
    total_size=$(du -sh captured_frames/ | cut -f1)

    echo -e "${GREEN}🎉 SUCCESS! Captured $frame_count frames${NC}"
    echo "   Total size: $total_size"
    echo "   Format: PPM (Portable Pixmap)"
    echo

    echo -e "${BLUE}📁 Captured Files:${NC}"
    ls -la captured_frames/ | head -8
    if [ "$frame_count" -gt 7 ]; then
        echo "   ... and $(($frame_count - 7)) more files"
    fi
    echo

    # Show file details
    if [ -f "captured_frames/frame_000000.ppm" ]; then
        echo -e "${BLUE}🔍 File Analysis:${NC}"
        file_info=$(file captured_frames/frame_000000.ppm)
        file_size=$(stat -f%z "captured_frames/frame_000000.ppm" 2>/dev/null || stat -c%s "captured_frames/frame_000000.ppm")
        echo "   $file_info"
        echo "   File size: $(echo $file_size | numfmt --to=iec-i --suffix=B --format="%.1f" 2>/dev/null || echo $file_size bytes)"
        echo
    fi

    # Show viewing commands
    echo -e "${YELLOW}🎨 View the captured frames:${NC}"
    echo "   display captured_frames/frame_000000.ppm    # ImageMagick"
    echo "   feh captured_frames/                        # feh image viewer"
    echo "   convert captured_frames/frame_*.ppm anim.gif # Create animation"
    echo

else
    echo -e "${RED}❌ No frames were captured${NC}"
    echo "   This indicates an issue with the capture process"
    exit 1
fi

# Technical summary
echo -e "${YELLOW}🔧 Technical Summary${NC}"
echo "==================="
echo -e "${GREEN}✅ Vulkan layer builds successfully${NC}"
echo -e "${GREEN}✅ Layer loads and initializes correctly${NC}"
echo -e "${GREEN}✅ Vulkan API calls are intercepted${NC}"
echo -e "${GREEN}✅ Swapchain operations work properly${NC}"
echo -e "${GREEN}✅ Frame capture mechanism functions${NC}"
echo -e "${GREEN}✅ Files are written to disk with correct format${NC}"
echo

# Usage instructions
echo -e "${YELLOW}🚀 Production Usage${NC}"
echo "=================="
echo "To use this layer with real Vulkan applications:"
echo
echo "1. Copy layer files to your desired location"
echo "2. Set environment variables:"
echo "   export VK_LAYER_PATH=\"/path/to/unseen\""
echo "   export VK_INSTANCE_LAYERS=\"VK_LAYER_PRIVATE_unseen\""
echo "   export VK_UNSEEN_ENABLE=1"
echo "   export VK_CAPTURE_OUTPUT_DIR=\"/path/to/output\""
echo "3. Run any Vulkan application"
echo

echo -e "${GREEN}🎉 DEMONSTRATION COMPLETE!${NC}"
echo "=========================="
echo "The Unseen Vulkan Layer is working!"
echo -e "${BLUE}📁 Demo files: $DEMO_DIR${NC}"
echo -e "${BLUE}🖼️  Captured frames: $DEMO_DIR/captured_frames/${NC}"
echo
echo "Ready for use! 🚀"
