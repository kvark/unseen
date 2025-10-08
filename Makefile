# Makefile for Unseen Vulkan Layer
# Builds both the Rust library and C test programs

.PHONY: all clean debug release test install help c-programs rust-library

# Default target
all: release

# Help target
help:
	@echo "Unseen Vulkan Layer Build System"
	@echo "================================="
	@echo "Available targets:"
	@echo "  all        - Build everything (release mode)"
	@echo "  debug      - Build in debug mode"
	@echo "  release    - Build in release mode"
	@echo "  rust-lib   - Build only the Rust library"
	@echo "  c-programs - Build only the C programs"
	@echo "  test       - Run tests"
	@echo "  clean      - Clean build artifacts"
	@echo "  install    - Install to system (requires sudo)"
	@echo "  help       - Show this help"
	@echo ""
	@echo "Build outputs:"
	@echo "  Rust library: target/{debug,release}/libVkLayer_PRIVATE_unseen.so"
	@echo "  C programs:   target/{debug,release}/bin/"

# Build everything in release mode
release: rust-library c-programs-release
	@echo "✅ Release build complete"

# Build everything in debug mode
debug: rust-library-debug c-programs-debug
	@echo "✅ Debug build complete"

# Build only the Rust library (release)
rust-library:
	@echo "🔨 Building Rust library (release)..."
	cargo build --release

# Build only the Rust library (debug)
rust-library-debug:
	@echo "🔨 Building Rust library (debug)..."
	cargo build

# Build C programs (release)
c-programs-release: rust-library
	@echo "🔨 Building C programs (release)..."
	@chmod +x scripts/build_c_programs.sh
	@scripts/build_c_programs.sh release

# Build C programs (debug)
c-programs-debug: rust-library-debug
	@echo "🔨 Building C programs (debug)..."
	@chmod +x scripts/build_c_programs.sh
	@scripts/build_c_programs.sh debug

# Alias for c-programs-release
c-programs: c-programs-release

# Run tests
test: release
	@echo "🧪 Running tests..."
	@chmod +x scripts/test_layer.sh
	@scripts/test_layer.sh

# Run frame capture demo
demo: release
	@echo "🎬 Running frame capture demo..."
	@chmod +x examples/frame_capture_demo.sh
	@examples/frame_capture_demo.sh

# Run final demo
final-demo: release
	@echo "🎭 Running final demo..."
	@chmod +x scripts/final_demo.sh
	@scripts/final_demo.sh

# Clean all build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@rm -rf target/debug/bin target/release/bin
	@echo "✅ Clean complete"

# Install to system (requires appropriate permissions)
install: release
	@echo "📦 Installing Unseen Vulkan Layer..."
	@if [ -z "$(DESTDIR)" ]; then \
		echo "Installing to system directories..."; \
		LAYER_DIR=/usr/local/share/vulkan/explicit_layer.d; \
		LIB_DIR=/usr/local/lib; \
	else \
		echo "Installing to $(DESTDIR)..."; \
		LAYER_DIR=$(DESTDIR)/usr/local/share/vulkan/explicit_layer.d; \
		LIB_DIR=$(DESTDIR)/usr/local/lib; \
	fi; \
	mkdir -p "$$LAYER_DIR" "$$LIB_DIR"; \
	cp target/release/libVkLayer_PRIVATE_unseen.so "$$LIB_DIR/"; \
	sed 's|./|'"$$LIB_DIR"'/|g' VkLayer_PRIVATE_unseen.json > "$$LAYER_DIR/VkLayer_PRIVATE_unseen.json"; \
	echo "✅ Installed to $$LAYER_DIR and $$LIB_DIR"

# Uninstall from system
uninstall:
	@echo "🗑️ Uninstalling Unseen Vulkan Layer..."
	@rm -f /usr/local/lib/libVkLayer_PRIVATE_unseen.so
	@rm -f /usr/local/share/vulkan/explicit_layer.d/VkLayer_PRIVATE_unseen.json
	@echo "✅ Uninstalled"

# Check build environment
check-env:
	@echo "🔍 Checking build environment..."
	@command -v rustc >/dev/null 2>&1 || { echo "❌ Rust compiler not found"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "❌ Cargo not found"; exit 1; }
	@command -v gcc >/dev/null 2>&1 || { echo "❌ GCC not found"; exit 1; }
	@command -v pkg-config >/dev/null 2>&1 || { echo "⚠️ pkg-config not found (may affect Vulkan detection)"; }
	@echo "✅ Build environment OK"

# Show build info
info:
	@echo "Unseen Vulkan Layer Build Information"
	@echo "===================================="
	@echo "Rust version: $$(rustc --version 2>/dev/null || echo 'Not found')"
	@echo "Cargo version: $$(cargo --version 2>/dev/null || echo 'Not found')"
	@echo "GCC version: $$(gcc --version 2>/dev/null | head -1 || echo 'Not found')"
	@echo ""
	@echo "Project structure:"
	@echo "  Source: src/"
	@echo "  Examples: examples/"
	@echo "  Tests: tests/"
	@echo "  Scripts: scripts/"
	@echo "  Build output: target/"
	@echo ""
	@if [ -f "target/release/libVkLayer_PRIVATE_unseen.so" ]; then \
		echo "✅ Release library exists"; \
	else \
		echo "❌ Release library not built"; \
	fi
	@if [ -d "target/release/bin" ] && [ "$$(ls -A target/release/bin 2>/dev/null)" ]; then \
		echo "✅ C programs built: $$(ls target/release/bin 2>/dev/null | tr '\n' ' ')"; \
	else \
		echo "❌ C programs not built"; \
	fi
