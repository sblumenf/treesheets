#!/bin/bash
# Build script for TreeSheets Web (WASM)
#
# Prerequisites:
#   - Emscripten SDK installed and activated
#   - Run: source /path/to/emsdk/emsdk_env.sh
#
# Usage:
#   ./build.sh          # Debug build
#   ./build.sh release  # Release build

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_TYPE="${1:-Debug}"

# Convert to proper case for CMake
if [ "${BUILD_TYPE,,}" = "release" ]; then
    BUILD_TYPE="Release"
else
    BUILD_TYPE="Debug"
fi

BUILD_DIR="${SCRIPT_DIR}/../../build-wasm-${BUILD_TYPE,,}"

echo "=== TreeSheets Web Build ==="
echo "Build type: ${BUILD_TYPE}"
echo "Build directory: ${BUILD_DIR}"
echo ""

# Check for Emscripten
if ! command -v emcc &> /dev/null; then
    echo "Error: Emscripten not found!"
    echo "Please install and activate the Emscripten SDK:"
    echo "  git clone https://github.com/emscripten-core/emsdk.git"
    echo "  cd emsdk && ./emsdk install latest && ./emsdk activate latest"
    echo "  source ./emsdk_env.sh"
    exit 1
fi

echo "Using Emscripten: $(emcc --version | head -n1)"
echo ""

# Configure
echo "=== Configuring ==="
emcmake cmake -S "${SCRIPT_DIR}" -B "${BUILD_DIR}" \
    -DCMAKE_BUILD_TYPE="${BUILD_TYPE}"

# Build
echo ""
echo "=== Building ==="
cmake --build "${BUILD_DIR}" -j$(nproc 2>/dev/null || echo 4)

echo ""
echo "=== Build Complete ==="
echo "Output files are in: ${BUILD_DIR}/web/"
echo ""
echo "To run locally:"
echo "  cd ${BUILD_DIR}/web"
echo "  python3 -m http.server 8000"
echo "  # Open http://localhost:8000 in your browser"
