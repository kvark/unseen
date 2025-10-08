# Unseen Vulkan Layer

A Vulkan layer that intercepts swapchain operations and saves rendered frames to disk. This layer is designed to work in headless environments without X11/Wayland, making it perfect for automated testing, CI/CD pipelines, and server-side rendering scenarios.

## Features

- **Headless Operation**: Works without a display server (X11/Wayland)
- **Frame Capture**: Automatically saves every presented frame to disk
- **PPM Format**: Saves frames in simple PPM format for easy viewing/processing
- **Configurable Output**: Customizable output directory via environment variables
- **Minimal Overhead**: Lightweight layer with minimal performance impact
- **Standard Compliance**: Fully compliant with Vulkan layer specification

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

The layer implements the Vulkan layer interface and intercepts key swapchain functions:

- `vkCreateSwapchainKHR`: Creates fake swapchain for headless operation
- `vkGetSwapchainImagesKHR`: Returns fake image handles
- `vkAcquireNextImageKHR`: Always returns image index 0
- `vkQueuePresentKHR`: Triggers frame capture and file writing

### Current Implementation

The current implementation creates placeholder gradient images to demonstrate the capture mechanism. In a production version, you would:

1. Copy the actual swapchain image data from GPU memory
2. Handle proper format conversion (BGRA to RGB, etc.)
3. Implement memory mapping and data extraction
4. Add support for different image formats and layouts

## Limitations & TODO

- **Placeholder Images**: Currently generates gradient patterns instead of actual frame data
- **Format Support**: Only outputs PPM format
- **Memory Handling**: Needs actual GPU memory readback implementation
- **Performance**: Could be optimized for high frame rates
- **Error Handling**: Basic error handling, could be more robust

## Development

### Adding Real Frame Capture

To implement actual frame capture, you would need to:

1. Create staging images with `HOST_VISIBLE` memory
2. Copy swapchain images to staging images using `vkCmdCopyImage`
3. Map staging memory and read pixel data
4. Handle format conversion and stride alignment
5. Write actual pixel data to output files

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

- Check Vulkan driver installation
- Verify application actually uses swapchain
- Look for error messages in logs