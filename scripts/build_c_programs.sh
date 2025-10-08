#!/usr/bin/env bash

# Build script for C test and example programs
# Outputs binaries to target/debug or target/release directories

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Default to debug build
BUILD_TYPE="${1:-debug}"

if [ "$BUILD_TYPE" != "debug" ] && [ "$BUILD_TYPE" != "release" ]; then
    echo "Usage: $0 [debug|release]"
    echo "  debug   - Build with debug symbols (default)"
    echo "  release - Build with optimizations"
    exit 1
fi

TARGET_DIR="target/$BUILD_TYPE"
BIN_DIR="$TARGET_DIR/bin"

# Create target directories
mkdir -p "$BIN_DIR"

echo "Building C programs ($BUILD_TYPE mode)"
echo "======================================"

# Compiler flags
CFLAGS="-std=c99 -Wall -Wextra -D_DEFAULT_SOURCE -D_POSIX_C_SOURCE=200112L"
LDFLAGS=""

if [ "$BUILD_TYPE" = "debug" ]; then
    CFLAGS="$CFLAGS -g -O0 -DDEBUG"
else
    CFLAGS="$CFLAGS -O2 -DNDEBUG"
fi

# Build direct test
if [ -f "tests/c/direct_test.c" ]; then
    echo "🔨 Building direct_test..."
    gcc $CFLAGS -o "$BIN_DIR/direct_test" tests/c/direct_test.c -ldl
    echo "   ✅ $BIN_DIR/direct_test"
fi

# Build simple test
if [ -f "examples/c/simple_test.c" ]; then
    echo "🔨 Building simple_test..."
    gcc $CFLAGS -o "$BIN_DIR/simple_test" examples/c/simple_test.c -lvulkan
    echo "   ✅ $BIN_DIR/simple_test"
fi

# Build any other C files in examples/c/
for c_file in examples/c/*.c; do
    if [ -f "$c_file" ] && [ "$(basename "$c_file")" != "simple_test.c" ]; then
        base_name=$(basename "$c_file" .c)
        echo "🔨 Building $base_name..."
        gcc $CFLAGS -o "$BIN_DIR/$base_name" "$c_file" -lvulkan -ldl
        echo "   ✅ $BIN_DIR/$base_name"
    fi
done

# Build any other C files in tests/c/
for c_file in tests/c/*.c; do
    if [ -f "$c_file" ] && [ "$(basename "$c_file")" != "direct_test.c" ]; then
        base_name=$(basename "$c_file" .c)
        echo "🔨 Building $base_name..."
        gcc $CFLAGS -o "$BIN_DIR/$base_name" "$c_file" -lvulkan -ldl
        echo "   ✅ $BIN_DIR/$base_name"
    fi
done

echo
echo "✅ C programs built successfully"
echo "📁 Binaries location: $BIN_DIR/"
echo
echo "Built programs:"
ls -la "$BIN_DIR/" 2>/dev/null || echo "  (none found)"
