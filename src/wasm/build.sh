#!/bin/bash
#
# TreeSheets WebAssembly Build Script
#
# This script compiles the TreeSheets C++ code to WebAssembly using Emscripten.
#
# Requirements:
#   - Emscripten SDK installed and activated
#   - Run: source /path/to/emsdk/emsdk_env.sh
#
# Usage:
#   ./build.sh [debug|release]
#

set -e  # Exit on error

# Configuration
BUILD_TYPE="${1:-release}"
OUTPUT_DIR="../../dist/wasm"
SOURCE_FILE="wasm_main.cpp"
OUTPUT_FILE="treesheets"

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "================================================"
echo "TreeSheets WebAssembly Build"
echo "================================================"
echo "Build type: $BUILD_TYPE"
echo "Output dir: $OUTPUT_DIR"
echo ""

# Common flags
COMMON_FLAGS=(
    -s WASM=1
    -s ALLOW_MEMORY_GROWTH=1
    -s INITIAL_MEMORY=67108864
    -s ASYNCIFY=1
    -s 'ASYNCIFY_IMPORTS=["JS_AskText","JS_AskNumber","JS_SingleChoice","JS_PickColor","JS_GetClipboardText"]'
    -s 'EXPORTED_FUNCTIONS=["_main","_WASM_FileLoaded","_WASM_Mouse","_WASM_Key","_WASM_Resize","_WASM_Action"]'
    -s 'EXPORTED_RUNTIME_METHODS=["UTF8ToString","stringToUTF8","lengthBytesUTF8","ccall","cwrap"]'
    -s MODULARIZE=1
    -s 'EXPORT_NAME="TreeSheetsModule"'
    --js-library library_treesheets.js
    --shell-file shell.html
    -I../
)

# Build-specific flags
if [ "$BUILD_TYPE" = "debug" ]; then
    echo "Building DEBUG version..."
    FLAGS=(
        "${COMMON_FLAGS[@]}"
        -O0
        -g4
        -s ASSERTIONS=2
        -s SAFE_HEAP=1
        -s STACK_OVERFLOW_CHECK=2
    )
else
    echo "Building RELEASE version..."
    FLAGS=(
        "${COMMON_FLAGS[@]}"
        -O3
        --closure 1
        -s ASSERTIONS=0
    )
fi

# Compile
echo "Compiling $SOURCE_FILE..."
emcc "$SOURCE_FILE" "${FLAGS[@]}" -o "$OUTPUT_DIR/$OUTPUT_FILE.html"

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo ""
    echo "================================================"
    echo "Build successful!"
    echo "================================================"
    echo "Output files:"
    echo "  - $OUTPUT_DIR/$OUTPUT_FILE.html"
    echo "  - $OUTPUT_DIR/$OUTPUT_FILE.js"
    echo "  - $OUTPUT_DIR/$OUTPUT_FILE.wasm"
    echo ""
    echo "To serve locally:"
    echo "  cd $OUTPUT_DIR && python3 -m http.server 8000"
    echo "  Then open: http://localhost:8000/$OUTPUT_FILE.html"
    echo ""
else
    echo ""
    echo "================================================"
    echo "Build FAILED!"
    echo "================================================"
    exit 1
fi
