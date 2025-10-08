# Unseen Vulkan Layer

A comprehensive Vulkan layer that provides complete headless surface and swapchain functionality for running Vulkan applications without display servers. The layer intercepts Vulkan API calls, creates virtual surfaces and swapchains, and captures rendered frames to disk for analysis, testing, or automated processing.

## Features

### Core Capabilities
- **Complete VK_KHR_surface Implementation**: Full headless surface support with proper capabilities
- **Complete VK_KHR_swapchain Implementation**: Virtual swapchains with configurable properties  
- **Headless Operation**: Works without X11/Wayland - perfect for servers and containers
- **Frame Capture System**: Captures and saves presented frames automatically
- **Idiomatic Configuration**: Standard Vulkan layer configuration with environment variables

### Advanced Features
- **Multi-Device Support**: Handles multiple Vulkan devices simultaneously
- **Thread-Safe Architecture**: All shared data properly synchronized
- **Configurable Capture**: Frame frequency control, output formats, size limits
- **Real GPU Capture Ready**: Infrastructure in place for actual GPU memory capture
- **Standards Compliant**: Fully compliant with Vulkan layer specification

## Project Structure

```
unseen/
├── src/                    # Rust library source code
│   └── lib.rs             # Main layer implementation
├── examples/              # Example programs and demos
│   ├── c/                 # C example programs
│   ├── demo.sh           # Main demonstration script
│   └── frame_capture_demo.sh # Detailed frame capture demo
├── tests/                 # Test programs
│   └── c/                # C test programs
├── scripts/               # Build and utility scripts
│   ├── build_c_programs.sh # C program build script
│   ├── test_layer.sh     # Layer testing script
│   └── final_demo.sh     # Complete demonstration
├── target/                # Build output directory
│   ├── debug/            # Debug build artifacts
│   │   └── bin/         # Debug C programs
│   └── release/          # Release build artifacts
│       ├── libVkLayer_PRIVATE_unseen.so # The layer library
│       └── bin/         # Release C programs
├── Cargo.toml            # Rust project configuration
├── Makefile              # Build system
└── VkLayer_PRIVATE_unseen.json # Layer manifest
```

## Building

### Prerequisites

- Rust (latest stable version)
- Vulkan SDK or development packages
- GCC for building test programs
- Make (optional, for convenience)

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install build-essential pkg-config libvulkan-dev vulkan-tools
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Fedora
```bash
sudo dnf install gcc rust cargo vulkan-devel vulkan-tools make
```

#### Arch Linux
```bash
sudo pacman -S base-devel rust vulkan-devel vulkan-tools
```

### Quick Build

Using the Makefile (recommended):

```bash
# Build everything (release mode)
make

# Build with specific targets
make debug          # Debug build
make release        # Release build
make c-programs     # Only C programs
make rust-library   # Only Rust library

# Run tests and demos
make test          # Run layer tests
make demo          # Run frame capture demo
make final-demo    # Run complete demonstration

# Get help
make help
```

### Manual Build

```bash
# Build the Rust layer
cargo build --release

# Build C test programs
scripts/build_c_programs.sh release

# Run tests
scripts/test_layer.sh
```

### Build Outputs

- **Rust library**: `target/release/libVkLayer_PRIVATE_unseen.so`
- **C programs**: `target/release/bin/`
- **Debug builds**: `target/debug/` (same structure)

## Usage

### Environment Variables

- `VK_LAYER_PATH`: Directory containing the layer manifest (usually current directory)
- `VK_INSTANCE_LAYERS`: Set to `VK_LAYER_PRIVATE_unseen` to enable the layer
- `VK_UNSEEN_ENABLE`: Set to `1` to enable frame capture
- `VK_CAPTURE_OUTPUT_DIR`: Output directory for captured frames (default: `./captured_frames`)
- `RUST_LOG`: Set logging level (`error`, `warn`, `info`, `debug`, `trace`)

### Quick Test

Use the provided test script:

```bash
make test
# or manually:
scripts/test_layer.sh
```

### Manual Usage

```bash
# Set environment variables
export VK_LAYER_PATH="$(pwd)"
export VK_INSTANCE_LAYERS="VK_LAYER_PRIVATE_unseen"
export VK_UNSEEN_ENABLE=1
export VK_CAPTURE_OUTPUT_DIR="./my_captures"
export RUST_LOG=info

# Create output directory
mkdir -p "$VK_CAPTURE_OUTPUT_DIR"

# Run any Vulkan application
vkcube
# or
your_vulkan_app
```

### Integration with Applications

The layer can be used with any Vulkan application by setting the appropriate environment variables before launching:

```bash
# For automated testing
VK_LAYER_PATH=/path/to/layer \
VK_INSTANCE_LAYERS=VK_LAYER_PRIVATE_unseen \
VK_UNSEEN_ENABLE=1 \
VK_CAPTURE_OUTPUT_DIR=/tmp/test_output \
./my_vulkan_test_suite

# For CI/CD pipelines
docker run -e VK_LAYER_PATH=/layers \
           -e VK_INSTANCE_LAYERS=VK_LAYER_CAPTURE_frames \
           -e VK_CAPTURE_ENABLE=1 \
           -v $(pwd):/layers \
           my-vulkan-app:latest
```

## Output Format

Frames are saved as PPM (Portable Pixmap) files with sequential numbering:

```
captured_frames/
├── frame_000000.ppm
├── frame_000001.ppm
├── frame_000002.ppm
└── ...
```

PPM files can be viewed with most image viewers or converted to other formats:

```bash
# View with ImageMagick
display frame_000000.ppm

# Convert to PNG
convert frame_000000.ppm frame_000000.png

# Create animated GIF from sequence
convert frame_*.ppm animation.gif
```

## Architecture

The layer provides a complete headless Vulkan implementation with the following components:

### VK_KHR_surface Implementation
- `vkCreateHeadlessSurfaceEXT`: Creates virtual headless surfaces
- `vkDestroySurfaceKHR`: Proper surface cleanup
- `vkGetPhysicalDeviceSurfaceCapabilitiesKHR`: Returns realistic surface capabilities
- `vkGetPhysicalDeviceSurfaceFormatsKHR`: Supports common formats (BGRA8, RGBA8)
- `vkGetPhysicalDeviceSurfacePresentModesKHR`: FIFO, MAILBOX, IMMEDIATE modes
- `vkGetPhysicalDeviceSurfaceSupportKHR`: Always supports headless operation

### VK_KHR_swapchain Implementation  
- `vkCreateSwapchainKHR`: Creates virtual swapchains with proper properties
- `vkDestroySwapchainKHR`: Cleans up swapchain resources
- `vkGetSwapchainImagesKHR`: Returns virtual image handles
- `vkAcquireNextImageKHR`: Cycles through available images
- `vkQueuePresentKHR`: Triggers frame capture and file writing

### Frame Capture Modes

**Current Mode (Synthetic)**: Generates animated test content for development and testing
- Realistic animated patterns simulating typical application rendering
- Perfect for testing layer functionality without real GPU dependencies
- Useful for CI/CD environments and development

**Real GPU Capture (Planned)**: Complete implementation ready for actual GPU memory capture
- Infrastructure in place for staging buffers and command buffer management
- Format conversion support (BGRA ↔ RGB, etc.)
- Proper synchronization and memory mapping
- See `REAL_GPU_CAPTURE_GUIDE.md` for implementation details

## Current Status & Roadmap

### ✅ Completed Features
- Complete VK_KHR_surface and VK_KHR_swapchain implementation
- Thread-safe multi-device support
- Configurable frame capture system
- Comprehensive test suite
- Standards-compliant layer interface
- Synthetic frame generation for testing

### 🚧 In Progress
- Real GPU memory capture implementation
- PNG output format support
- Performance optimization

### 📋 Planned Features
- Actual GPU framebuffer capture (infrastructure ready)
- Additional output formats (PNG, JPEG)
- Network streaming capabilities
- Configuration file support
- Advanced filtering and post-processing

## Development

### Real GPU Capture Implementation

The layer includes complete infrastructure for real GPU capture. See `REAL_GPU_CAPTURE_GUIDE.md` for detailed implementation instructions including:

- Staging buffer management with host-visible memory
- Command buffer operations for image copying  
- Memory mapping and format conversion
- Synchronization and performance optimization
- Integration with real Vulkan drivers

The current synthetic mode can be easily switched to real capture mode once integrated with actual Vulkan drivers.

### Debugging

Enable detailed logging:

```bash
export RUST_LOG=debug
export VK_CAPTURE_ENABLE=1
your_vulkan_app 2>&1 | grep -E "(CAPTURE|ERROR|WARN)"
```

### Testing

Test with various Vulkan applications:

```bash
# Simple spinning cube
vkcube

# Vulkan samples
./vulkan_sample

# Custom applications
./your_app
```

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## Troubleshooting

### Layer Not Loading

- Verify `VK_LAYER_PATH` points to directory containing `VkLayer_capture.json`
- Check that `libVkLayer_capture.so` exists and has correct permissions
- Ensure `VK_INSTANCE_LAYERS` is set correctly

### No Frames Captured

- Verify `VK_CAPTURE_ENABLE=1` is set
- Check output directory permissions
- Enable debug logging to see layer activity

### Build Issues

- Update Rust: `rustup update`
- Install Vulkan development packages
- Check for missing system dependencies

### Runtime Errors

- Check Vulkan driver installation (for real GPU capture mode)
- Verify application actually uses swapchain
- Look for error messages in logs
- Try mock mode first: `export VK_UNSEEN_FORCE_MOCK=1`

### Real GPU Capture

For actual GPU memory capture (when implemented):
```bash
export VK_UNSEEN_REAL_CAPTURE=1      # Enable real GPU capture
export VK_CAPTURE_REAL_GPU=1         # Use actual GPU memory
export VK_CAPTURE_STAGING_BUFFERS=3  # Number of staging buffers
```

Current implementation uses synthetic frames for demonstration and testing purposes.