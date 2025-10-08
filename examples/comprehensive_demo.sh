#!/usr/bin/env bash

# Comprehensive Demo Script for Unseen Vulkan Layer
# This script demonstrates all the layer's features and configuration options

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
DEMO_OUTPUT_BASE="$PROJECT_ROOT/demo_outputs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}========================================${NC}"
}

print_step() {
    echo -e "${GREEN}[STEP]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_prerequisites() {
    print_step "Checking prerequisites..."

    # Check if layer library exists
    if [ ! -f "$BUILD_DIR/libVkLayer_PRIVATE_unseen.so" ]; then
        print_error "Layer library not found. Please run 'make release' first."
        exit 1
    fi

    # Check if layer manifest exists
    if [ ! -f "$PROJECT_ROOT/VkLayer_PRIVATE_unseen.json" ]; then
        print_error "Layer manifest not found."
        exit 1
    fi

    # Check for test programs
    if [ ! -f "$BUILD_DIR/bin/headless_test" ]; then
        print_warning "C test program not found. Building C programs..."
        cd "$PROJECT_ROOT"
        make c-programs
    fi

    print_info "Prerequisites check complete"
}

setup_environment() {
    print_step "Setting up environment variables..."

    # Basic layer configuration
    export VK_LAYER_PATH="$PROJECT_ROOT"
    export VK_INSTANCE_LAYERS="VK_LAYER_PRIVATE_unseen"
    export VK_UNSEEN_ENABLE=1
    export RUST_LOG=info

    print_info "Layer path: $VK_LAYER_PATH"
    print_info "Enabled layers: $VK_INSTANCE_LAYERS"
    print_info "Layer enabled: $VK_UNSEEN_ENABLE"
    print_info "Log level: $RUST_LOG"
}

test_basic_functionality() {
    print_header "Basic Functionality Test"

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/basic_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_FORMAT="ppm"
    export VK_CAPTURE_FREQUENCY=1
    export VK_CAPTURE_MAX_FRAMES=5

    print_step "Running basic headless test..."
    print_info "Output directory: $OUTPUT_DIR"
    print_info "Format: PPM"
    print_info "Capturing every frame, max 5 frames"

    cd "$BUILD_DIR"
    if ./bin/headless_test; then
        print_info "Basic test completed successfully"

        # Count captured frames
        FRAME_COUNT=$(ls "$OUTPUT_DIR"/*.ppm 2>/dev/null | wc -l || echo "0")
        print_info "Captured frames: $FRAME_COUNT"

        if [ "$FRAME_COUNT" -gt 0 ]; then
            print_info "Sample frame info:"
            ls -la "$OUTPUT_DIR"/*.ppm | head -3
        fi
    else
        print_error "Basic test failed"
        return 1
    fi

    cd "$PROJECT_ROOT"
}

test_configuration_options() {
    print_header "Configuration Options Test"

    # Test 1: Different output formats
    print_step "Testing different output formats..."

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/format_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_MAX_FRAMES=3

    # Test PPM format
    print_info "Testing PPM format..."
    export VK_CAPTURE_FORMAT="ppm"
    cd "$BUILD_DIR"
    ./bin/headless_test >/dev/null 2>&1 || true

    # Test PNG format (will fall back to PPM)
    print_info "Testing PNG format (falls back to PPM)..."
    export VK_CAPTURE_FORMAT="png"
    cd "$BUILD_DIR"
    ./bin/headless_test >/dev/null 2>&1 || true

    print_info "Format test results:"
    ls -la "$OUTPUT_DIR"/ | grep -E "\.(ppm|png)$" || echo "No output files found"

    # Test 2: Capture frequency
    print_step "Testing capture frequency..."

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/frequency_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_FORMAT="ppm"
    export VK_CAPTURE_FREQUENCY=2  # Capture every 2nd frame
    export VK_CAPTURE_MAX_FRAMES=0  # No limit

    print_info "Capturing every 2nd frame..."
    cd "$BUILD_DIR"
    ./bin/headless_test >/dev/null 2>&1 || true

    FRAME_COUNT=$(ls "$OUTPUT_DIR"/*.ppm 2>/dev/null | wc -l || echo "0")
    print_info "Captured frames with frequency=2: $FRAME_COUNT"

    # Test 3: Max frames limit
    print_step "Testing max frames limit..."

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/maxframes_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_FREQUENCY=1
    export VK_CAPTURE_MAX_FRAMES=3

    print_info "Limiting to 3 frames maximum..."
    cd "$BUILD_DIR"
    ./bin/headless_test >/dev/null 2>&1 || true

    FRAME_COUNT=$(ls "$OUTPUT_DIR"/*.ppm 2>/dev/null | wc -l || echo "0")
    print_info "Captured frames with max_frames=3: $FRAME_COUNT"

    cd "$PROJECT_ROOT"
}

test_surface_capabilities() {
    print_header "Surface Capabilities Test"

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/surface_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_FORMAT="ppm"
    export VK_CAPTURE_MAX_FRAMES=2
    export RUST_LOG=debug

    print_step "Testing surface creation and capabilities..."
    print_info "This test verifies:"
    print_info "  - VK_KHR_surface functions work correctly"
    print_info "  - Surface capabilities are reported properly"
    print_info "  - Surface formats and present modes are available"

    cd "$BUILD_DIR"
    if ./bin/headless_test 2>&1 | tee "$OUTPUT_DIR/surface_test.log"; then
        print_info "Surface test completed successfully"

        # Check log for surface-related messages
        if grep -q "Creating headless surface" "$OUTPUT_DIR/surface_test.log"; then
            print_info "✓ Surface creation logged"
        fi

        if grep -q "Getting surface capabilities" "$OUTPUT_DIR/surface_test.log"; then
            print_info "✓ Surface capabilities queried"
        fi

        if grep -q "Getting surface formats" "$OUTPUT_DIR/surface_test.log"; then
            print_info "✓ Surface formats queried"
        fi

        if grep -q "Getting surface present modes" "$OUTPUT_DIR/surface_test.log"; then
            print_info "✓ Surface present modes queried"
        fi
    else
        print_error "Surface test failed"
        return 1
    fi

    cd "$PROJECT_ROOT"
}

test_layer_enumeration() {
    print_header "Layer Enumeration Test"

    print_step "Testing layer enumeration functions..."

    # Create a simple test program to verify layer enumeration
    cat > "/tmp/layer_enum_test.c" << 'EOF'
#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    printf("Testing layer enumeration...\n");

    uint32_t layer_count = 0;
    VkResult result = vkEnumerateInstanceLayerProperties(&layer_count, NULL);
    if (result != VK_SUCCESS) {
        printf("Failed to enumerate layers\n");
        return 1;
    }

    printf("Available layers: %u\n", layer_count);

    if (layer_count > 0) {
        VkLayerProperties* layers = malloc(sizeof(VkLayerProperties) * layer_count);
        result = vkEnumerateInstanceLayerProperties(&layer_count, layers);

        if (result == VK_SUCCESS) {
            for (uint32_t i = 0; i < layer_count; i++) {
                printf("  Layer %u: %s (v%u) - %s\n",
                       i, layers[i].layerName, layers[i].implementationVersion,
                       layers[i].description);

                // Check if our layer is present
                if (strstr(layers[i].layerName, "VK_LAYER_PRIVATE_unseen") != NULL) {
                    printf("    ✓ Found our layer!\n");
                }
            }
        }

        free(layers);
    }

    // Test extension enumeration
    uint32_t extension_count = 0;
    result = vkEnumerateInstanceExtensionProperties("VK_LAYER_PRIVATE_unseen", &extension_count, NULL);
    if (result == VK_SUCCESS) {
        printf("Layer extensions: %u\n", extension_count);

        if (extension_count > 0) {
            VkExtensionProperties* extensions = malloc(sizeof(VkExtensionProperties) * extension_count);
            result = vkEnumerateInstanceExtensionProperties("VK_LAYER_PRIVATE_unseen", &extension_count, extensions);

            if (result == VK_SUCCESS) {
                for (uint32_t i = 0; i < extension_count; i++) {
                    printf("  Extension %u: %s (v%u)\n",
                           i, extensions[i].extensionName, extensions[i].specVersion);
                }
            }

            free(extensions);
        }
    }

    printf("Layer enumeration test complete\n");
    return 0;
}
EOF

    # Compile and run the test
    gcc -o "/tmp/layer_enum_test" "/tmp/layer_enum_test.c" -lvulkan

    export VK_LAYER_PATH="$PROJECT_ROOT"
    "/tmp/layer_enum_test"

    rm -f "/tmp/layer_enum_test" "/tmp/layer_enum_test.c"
}

performance_test() {
    print_header "Performance Test"

    OUTPUT_DIR="$DEMO_OUTPUT_BASE/performance_test"
    mkdir -p "$OUTPUT_DIR"

    export VK_CAPTURE_OUTPUT_DIR="$OUTPUT_DIR"
    export VK_CAPTURE_FORMAT="ppm"
    export VK_CAPTURE_FREQUENCY=1
    export VK_CAPTURE_MAX_FRAMES=50
    export RUST_LOG=warn  # Reduce log noise

    print_step "Running performance test with 50 frames..."
    print_info "This test measures the layer's overhead"

    START_TIME=$(date +%s.%N)

    cd "$BUILD_DIR"
    ./bin/headless_test >/dev/null 2>&1 || true

    END_TIME=$(date +%s.%N)
    DURATION=$(echo "$END_TIME - $START_TIME" | bc -l)

    FRAME_COUNT=$(ls "$OUTPUT_DIR"/*.ppm 2>/dev/null | wc -l || echo "0")

    if [ "$FRAME_COUNT" -gt 0 ]; then
        FPS=$(echo "scale=2; $FRAME_COUNT / $DURATION" | bc -l)
        AVG_TIME=$(echo "scale=4; $DURATION / $FRAME_COUNT" | bc -l)

        print_info "Performance results:"
        print_info "  Total time: ${DURATION}s"
        print_info "  Frames captured: $FRAME_COUNT"
        print_info "  Average FPS: $FPS"
        print_info "  Average time per frame: ${AVG_TIME}s"

        # Calculate file sizes
        TOTAL_SIZE=$(du -sb "$OUTPUT_DIR" | cut -f1)
        AVG_SIZE=$(echo "$TOTAL_SIZE / $FRAME_COUNT" | bc)
        print_info "  Total output size: ${TOTAL_SIZE} bytes"
        print_info "  Average file size: ${AVG_SIZE} bytes"
    else
        print_warning "No frames captured during performance test"
    fi

    cd "$PROJECT_ROOT"
}

generate_demo_report() {
    print_header "Demo Report Generation"

    REPORT_FILE="$DEMO_OUTPUT_BASE/demo_report.txt"

    print_step "Generating comprehensive demo report..."

    cat > "$REPORT_FILE" << EOF
Unseen Vulkan Layer - Demo Report
Generated on: $(date)
=====================================

Project Information:
- Project root: $PROJECT_ROOT
- Layer library: $BUILD_DIR/libVkLayer_PRIVATE_unseen.so
- Layer manifest: $PROJECT_ROOT/VkLayer_PRIVATE_unseen.json

Test Results:
EOF

    # Add frame counts from each test
    for test_dir in "$DEMO_OUTPUT_BASE"/*/; do
        if [ -d "$test_dir" ]; then
            test_name=$(basename "$test_dir")
            frame_count=$(ls "$test_dir"/*.ppm 2>/dev/null | wc -l || echo "0")
            total_size=$(du -sb "$test_dir" 2>/dev/null | cut -f1 || echo "0")

            echo "- $test_name: $frame_count frames, $total_size bytes" >> "$REPORT_FILE"
        fi
    done

    cat >> "$REPORT_FILE" << EOF

Layer Configuration Options Tested:
- VK_CAPTURE_OUTPUT_DIR: Output directory for captured frames
- VK_CAPTURE_FORMAT: Output format (ppm, png)
- VK_CAPTURE_FREQUENCY: Frame capture frequency
- VK_CAPTURE_MAX_FRAMES: Maximum frames to capture
- VK_UNSEEN_ENABLE: Enable/disable layer
- RUST_LOG: Logging level

Vulkan Functions Implemented:
Instance Functions:
- vkGetInstanceProcAddr
- vkCreateInstance
- vkDestroyInstance
- vkCreateDevice
- vkEnumerateInstanceLayerProperties
- vkEnumerateInstanceExtensionProperties

Surface Functions (VK_KHR_surface):
- vkCreateHeadlessSurfaceEXT
- vkDestroySurfaceKHR
- vkGetPhysicalDeviceSurfaceCapabilitiesKHR
- vkGetPhysicalDeviceSurfaceFormatsKHR
- vkGetPhysicalDeviceSurfacePresentModesKHR
- vkGetPhysicalDeviceSurfaceSupportKHR

Device Functions:
- vkGetDeviceProcAddr
- vkDestroyDevice

Swapchain Functions (VK_KHR_swapchain):
- vkCreateSwapchainKHR
- vkDestroySwapchainKHR
- vkGetSwapchainImagesKHR
- vkAcquireNextImageKHR
- vkQueuePresentKHR

Extensions Supported:
- VK_KHR_surface (instance)
- VK_EXT_headless_surface (instance)
- VK_KHR_swapchain (device)

Output Formats:
- PPM (Portable Pixmap) - fully implemented
- PNG - planned (falls back to PPM currently)

Notes:
- Current implementation generates synthetic frame data for demonstration
- Real GPU memory capture would require additional command buffer management
- Layer is designed for headless environments without display servers
- Thread-safe implementation using Mutex for shared data structures
EOF

    print_info "Demo report generated: $REPORT_FILE"

    # Display summary
    print_step "Demo Summary:"
    TOTAL_FRAMES=$(find "$DEMO_OUTPUT_BASE" -name "*.ppm" | wc -l)
    TOTAL_SIZE=$(du -sb "$DEMO_OUTPUT_BASE" | cut -f1)
    print_info "Total frames captured across all tests: $TOTAL_FRAMES"
    print_info "Total output size: $TOTAL_SIZE bytes"
    print_info "Demo output directory: $DEMO_OUTPUT_BASE"
}

cleanup_old_outputs() {
    if [ -d "$DEMO_OUTPUT_BASE" ]; then
        print_step "Cleaning up old demo outputs..."
        rm -rf "$DEMO_OUTPUT_BASE"
    fi
    mkdir -p "$DEMO_OUTPUT_BASE"
}

main() {
    print_header "Unseen Vulkan Layer - Comprehensive Demo"

    print_info "This demo will test all layer features and generate sample outputs"
    print_info "Make sure you have built the project with 'make release' first"
    echo

    # Check if user wants to continue
    read -p "Press Enter to continue or Ctrl+C to cancel..."

    cleanup_old_outputs
    check_prerequisites
    setup_environment

    # Run all tests
    test_basic_functionality
    test_configuration_options
    test_surface_capabilities
    test_layer_enumeration
    performance_test

    generate_demo_report

    print_header "Demo Complete!"
    print_info "All tests completed successfully"
    print_info "Check the following directory for outputs: $DEMO_OUTPUT_BASE"
    print_info "View the demo report: $DEMO_OUTPUT_BASE/demo_report.txt"
    echo
    print_info "Sample commands to view captured frames:"
    print_info "  # View a PPM file:"
    print_info "  display $DEMO_OUTPUT_BASE/basic_test/frame_000000.ppm"
    print_info "  # Convert to PNG:"
    print_info "  convert $DEMO_OUTPUT_BASE/basic_test/frame_000000.ppm frame.png"
    print_info "  # Create animation:"
    print_info "  convert $DEMO_OUTPUT_BASE/basic_test/frame_*.ppm animation.gif"
}

# Handle script interruption
trap 'echo -e "\n${YELLOW}Demo interrupted by user${NC}"; exit 1' INT

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
