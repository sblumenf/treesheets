cmake_minimum_required(VERSION 3.13)

project(TreeSheetsWeb
    DESCRIPTION "TreeSheets Web Port (WebAssembly)"
    VERSION 1.0.0)

# Require Emscripten
if(NOT EMSCRIPTEN)
    message(FATAL_ERROR "This CMakeLists.txt is for Emscripten builds only.
    Use: emcmake cmake -S src/wasm -B build-wasm && cmake --build build-wasm")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Source files
set(WASM_SOURCES
    wasm_main.cpp
)

# Create executable
add_executable(treesheets ${WASM_SOURCES})

# Include directories
target_include_directories(treesheets PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/..
)

# Emscripten-specific settings
set(EMSCRIPTEN_FLAGS
    # JavaScript library with browser bindings
    "--js-library=${CMAKE_CURRENT_SOURCE_DIR}/library_treesheets.js"

    # Export functions for JavaScript to call
    "-sEXPORTED_FUNCTIONS=['_main','_WASM_FileLoaded','_WASM_Mouse','_WASM_Key','_WASM_Resize','_WASM_Action']"
    "-sEXPORTED_RUNTIME_METHODS=['ccall','cwrap','UTF8ToString','stringToUTF8','lengthBytesUTF8']"

    # Allow memory growth for dynamic content
    "-sALLOW_MEMORY_GROWTH=1"

    # Initial memory (16MB)
    "-sINITIAL_MEMORY=16777216"

    # Stack size (1MB)
    "-sSTACK_SIZE=1048576"

    # Disable filesystem (we use browser APIs)
    "-sNO_FILESYSTEM=1"

    # Modularize output
    "-sMODULARIZE=1"
    "-sEXPORT_NAME='TreeSheets'"

    # Environment settings
    "-sENVIRONMENT='web'"

    # Enable assertions in debug builds
    "$<$<CONFIG:Debug>:-sASSERTIONS=2>"
    "$<$<CONFIG:Debug>:-sSTACK_OVERFLOW_CHECK=2>"
    "$<$<CONFIG:Debug>:-g>"

    # Optimization in release builds
    "$<$<CONFIG:Release>:-O3>"
    "$<$<CONFIG:Release>:-sASSERTIONS=0>"
)

# Convert list to string for linker flags
string(REPLACE ";" " " EMSCRIPTEN_FLAGS_STR "${EMSCRIPTEN_FLAGS}")

# Set linker flags
set_target_properties(treesheets PROPERTIES
    SUFFIX ".js"
    LINK_FLAGS "${EMSCRIPTEN_FLAGS_STR}"
)

# Compiler flags
target_compile_options(treesheets PRIVATE
    -Wall
    -Wextra
    -Wno-unused-parameter
    "$<$<CONFIG:Debug>:-g>"
    "$<$<CONFIG:Release>:-O3>"
)

# Create output directory for web files
set(WEB_OUTPUT_DIR ${CMAKE_BINARY_DIR}/web)
file(MAKE_DIRECTORY ${WEB_OUTPUT_DIR})

# Custom command to copy output files to web directory
add_custom_command(TARGET treesheets POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy
        ${CMAKE_BINARY_DIR}/treesheets.js
        ${CMAKE_BINARY_DIR}/treesheets.wasm
        ${WEB_OUTPUT_DIR}/
    COMMENT "Copying WASM files to web directory"
)

# Create HTML template
file(WRITE ${WEB_OUTPUT_DIR}/index.html [=[
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TreeSheets Web</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: #f0f0f0;
            display: flex;
            flex-direction: column;
            height: 100vh;
        }
        #menubar {
            display: flex;
            background: #333;
            color: white;
            padding: 0;
            font-size: 14px;
        }
        .menu-item {
            padding: 8px 16px;
            cursor: pointer;
            position: relative;
        }
        .menu-item:hover { background: #555; }
        #toolbar {
            display: flex;
            background: #d8c7bc;
            padding: 4px 8px;
            gap: 4px;
            align-items: center;
            flex-wrap: wrap;
        }
        .toolbar-button {
            padding: 4px 8px;
            background: #e8d7cc;
            border: 1px solid #999;
            cursor: pointer;
            font-size: 12px;
        }
        .toolbar-button:hover { background: #f8e7dc; }
        .toolbar-separator {
            width: 1px;
            height: 24px;
            background: #999;
            margin: 0 4px;
        }
        #canvas-container {
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: stretch;
            padding: 10px;
            overflow: hidden;
        }
        #canvas {
            background: white;
            border: 1px solid #ccc;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            width: 100%;
            height: 100%;
        }
        #status {
            background: #e0e0e0;
            padding: 4px 8px;
            font-size: 12px;
            border-top: 1px solid #ccc;
        }
        #loading {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(255,255,255,0.9);
            display: flex;
            justify-content: center;
            align-items: center;
            font-size: 24px;
            z-index: 1000;
        }
        #loading.hidden { display: none; }
        #file-input { display: none; }
    </style>
</head>
<body>
    <div id="loading">Loading TreeSheets...</div>
    <div id="menubar"></div>
    <div id="toolbar"></div>
    <div id="canvas-container">
        <canvas id="canvas"></canvas>
    </div>
    <div id="status">Ready</div>
    <input type="file" id="file-input" accept=".cts,.tsff">

    <script src="treesheets.js"></script>
    <script>
        // Initialize the WASM module
        TreeSheets({
            canvas: document.getElementById('canvas'),
            print: function(text) { console.log(text); },
            printErr: function(text) { console.error(text); }
        }).then(function(Module) {
            window.Module = Module;
            document.getElementById('loading').classList.add('hidden');
            console.log('TreeSheets Web initialized!');

            // Handle canvas resize
            function resizeCanvas() {
                const container = document.getElementById('canvas-container');
                const canvas = document.getElementById('canvas');
                canvas.width = container.clientWidth;
                canvas.height = container.clientHeight;
                if (Module._WASM_Resize) {
                    Module._WASM_Resize(canvas.width, canvas.height);
                }
            }
            window.addEventListener('resize', resizeCanvas);
            resizeCanvas();
        });
    </script>
</body>
</html>
]=])

message(STATUS "TreeSheets Web build configured")
message(STATUS "Build with: cmake --build ${CMAKE_BINARY_DIR}")
message(STATUS "Output will be in: ${WEB_OUTPUT_DIR}")
