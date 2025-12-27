/**
 * TreeSheets WebAssembly Library
 *
 * This library provides the JavaScript side of the C++/JS boundary for TreeSheets web port.
 *
 * Phase 1 Enhancements (Completed):
 *
 * 1. Accurate Text Measurement
 *    - JS_GetCharHeight: Uses Canvas TextMetrics.actualBoundingBox* for accurate height
 *    - JS_GetTextHeight: Per-string height measurement with fallback for older browsers
 *    - Ensures proper text layout and rendering
 *
 * 2. File Upload Support
 *    - JS_TriggerUpload: Uses FileReader API to load .cts files
 *    - Creates temporary file input, reads as ArrayBuffer, passes to WASM
 *    - Properly manages WASM heap memory allocation/deallocation
 *
 * 3. Mouse Wheel Support
 *    - Added 'wheel' event listener with type 3
 *    - Normalizes deltaY across browsers (pixel/line/page modes)
 *    - Prevents default scroll behavior for canvas interaction
 *
 * 4. Enhanced Keyboard Handling
 *    - Modifier keys (Ctrl, Shift, Alt, Meta) passed as bitflags
 *    - Prevents browser defaults for common shortcuts (Ctrl+S, Ctrl+O, etc.)
 *    - Enables proper keyboard shortcut handling in TreeSheets
 *
 * Phase 2 Enhancements (Completed):
 *
 * 5. Image/Bitmap Rendering
 *    - Asynchronous image loading with caching
 *    - Automatic retry and placeholder on loading
 *    - Supports toolbar icons and embedded images
 *
 * 6. HTML Modal Dialog System
 *    - Replaces basic alert/prompt with styled modals
 *    - Color picker, font selector, multi-choice dialogs
 *    - Better UX than browser defaults
 *
 * Phase 3 Enhancements (Completed):
 *
 * 7. Touch/Mobile Support
 *    - Single-touch: tap, drag, swipe gestures
 *    - Two-finger pinch-to-zoom (simulates wheel events)
 *    - Touch duration detection for tap vs drag
 *    - Prevents default touch behavior for smooth interaction
 *
 * 8. Performance Optimizations
 *    - Cached canvas context (_getCtx) reduces repeated lookups
 *    - Minimizes C++/JS boundary crossings
 *    - All drawing functions use shared context instance
 *
 * 9. LocalStorage Integration
 *    - JS_LocalStorage_SetItem/GetItem/RemoveItem/Clear
 *    - Namespaced keys ('treesheets_' prefix)
 *    - Persistent settings and state across sessions
 *    - Graceful error handling for quota/privacy limits
 *
 * 10. Keyboard Dialog Controls
 *    - Enter key submits (clicks primary button)
 *    - Escape key cancels (closes dialog)
 *    - Proper event cleanup on dialog close
 *    - Works with all modal dialogs
 *
 * Code Quality Fixes (Phase 4):
 *
 * 11. Security Hardening
 *    - XSS prevention: textContent instead of innerHTML
 *    - File size limits: 50MB max upload
 *    - Input validation: bounds checking on all inputs
 *
 * 12. Code Quality
 *    - Extracted color codec utilities (DRY principle)
 *    - Named constants for magic numbers
 *    - Cache eviction (max 100 images)
 *    - Proper null/undefined checks
 *
 * Production Quality Fixes (Phase 5):
 *
 * 13. Critical Bug Fixes
 *    - Fixed color codec endianness (RGB byte order)
 *    - Fixed image loading race condition
 *    - Fixed async dialog handling with Asyncify
 *    - Added JSON.parse error handling
 *
 * 14. Error Boundaries
 *    - Memory allocation checks (_malloc failure handling)
 *    - File upload error handling and cleanup
 *    - Try-catch blocks for critical operations
 *    - Graceful degradation on errors
 *
 * 15. Resource Cleanup
 *    - Input element cleanup after file upload
 *    - Event listener deduplication guard
 *    - Modal keydown listener cleanup
 *    - Proper DOM element removal
 *
 * Critical Stubs Implementation (Phase 6):
 *
 * 16. Clipboard Read/Write
 *    - JS_GetClipboardText: Asyncify-based clipboard reading
 *    - Clipboard API with execCommand fallback
 *    - Error handling and browser compatibility
 *
 * 17. Pen/Brush Rendering
 *    - Complete SetPen implementation (9 pen types)
 *    - Complete SetBrush implementation (4 brush types)
 *    - Proper line styles and fill patterns
 *
 * 18. Virtual File System
 *    - LocalStorage-based file caching
 *    - Base64 encoding for binary data
 *    - JS_ReadFile implementation
 *    - Automatic file persistence on upload
 *
 * Accessibility (Phase 7 - WCAG 2.1 Compliant):
 *
 * 19. ARIA Labels & Roles
 *    - role="dialog" with aria-modal on modals
 *    - role="menubar", "menu", "menuitem" on navigation
 *    - role="toolbar" on toolbar
 *    - Comprehensive aria-label attributes
 *
 * 20. Keyboard Navigation
 *    - Full menu keyboard control (Arrow keys, Enter, Escape)
 *    - Tab navigation through interactive elements
 *    - Focus trap in modal dialogs
 *    - Focus restoration on dialog close
 *
 * 21. Focus Management
 *    - Automatic focus on first element in modals
 *    - Focus preservation across dialog interactions
 *    - Screen reader announcements
 *
 * Performance Optimizations (Phase 8):
 *
 * 22. Event Throttling/Debouncing
 *    - Resize events debounced (150ms)
 *    - Mousemove throttled to ~60fps (requestAnimationFrame)
 *    - Reduced C++/JS boundary crossings
 *
 * Infrastructure (Phase 9):
 *
 * 23. Build System
 *    - build.sh script with debug/release modes
 *    - Proper Emscripten flags configuration
 *    - Output optimization and minification
 *
 * 24. Production HTML Template
 *    - Loading screen with progress bar
 *    - Error handling and user feedback
 *    - Browser compatibility detection
 *    - Proper metadata and SEO
 *
 * 25. Deployment Documentation
 *    - Complete DEPLOYMENT.md guide
 *    - Web server configuration examples
 *    - Security headers (CSP, COOP, COEP)
 *    - HTTPS requirements and setup
 *
 * BUILD REQUIREMENTS:
 *    This library requires Emscripten ASYNCIFY to be enabled for proper
 *    dialog and clipboard functionality. Use the provided build script:
 *      ./build.sh release
 *    Or manually add to your build command:
 *      -s ASYNCIFY=1
 *      -s 'ASYNCIFY_IMPORTS=["JS_AskText","JS_AskNumber","JS_SingleChoice","JS_PickColor","JS_GetClipboardText"]'
 *    See DEPLOYMENT.md for complete build and deployment instructions.
 *
 * Modifier Bitflags:
 *   Bit 0 (1): Ctrl
 *   Bit 1 (2): Shift
 *   Bit 2 (4): Alt
 *   Bit 3 (8): Meta/Command
 */
mergeInto(LibraryManager.library, {
    // ===== CONSTANTS =====
    _CONSTANTS: {
        // Font style bits
        FONT_BOLD: 1,
        FONT_ITALIC: 2,
        FONT_MONOSPACE: 4,

        // Font size limits
        MIN_FONT_SIZE: 6,
        MAX_FONT_SIZE: 200,
        DEFAULT_FONT_SIZE: 12,

        // Touch detection
        TAP_DURATION_MS: 200,

        // File limits
        MAX_FILE_SIZE_BYTES: 50 * 1024 * 1024, // 50MB

        // Cache limits
        MAX_IMAGE_CACHE_SIZE: 100,

        // Image placeholder size
        PLACEHOLDER_SIZE: 16
    },

    // ===== UTILITY FUNCTIONS =====

    // Color codec utilities (eliminates duplication)
    _colorUtils: {
        // C++ contract: color is 0xRRGGBB (Red in bits 16-23, Green in 8-15, Blue in 0-7)
        decode: function(color) {
            return {
                r: (color >> 16) & 0xFF,  // Red in high byte
                g: (color >> 8) & 0xFF,   // Green in middle byte
                b: color & 0xFF           // Blue in low byte
            };
        },
        toRGBString: function(color) {
            var c = this.decode(color);
            return 'rgb(' + c.r + ',' + c.g + ',' + c.b + ')';
        },
        fromHex: function(hexString) {
            var hex = hexString.substring(1);
            var r = parseInt(hex.substring(0, 2), 16);
            var g = parseInt(hex.substring(2, 4), 16);
            var b = parseInt(hex.substring(4, 6), 16);
            return (r << 16) | (g << 8) | b;  // Encode as 0xRRGGBB
        },
        toHex: function(color) {
            var c = this.decode(color);
            return '#' +
                ('0' + c.r.toString(16)).slice(-2) +
                ('0' + c.g.toString(16)).slice(-2) +
                ('0' + c.b.toString(16)).slice(-2);
        }
    },

    // Input validation utilities
    _validation: {
        clampFontSize: function(size) {
            var constants = Module._CONSTANTS;
            if (typeof size !== 'number' || isNaN(size)) {
                return constants.DEFAULT_FONT_SIZE;
            }
            return Math.max(constants.MIN_FONT_SIZE,
                          Math.min(constants.MAX_FONT_SIZE, size));
        },
        isValidPointer: function(ptr) {
            return ptr !== 0 && ptr !== null && ptr !== undefined;
        },
        sanitizeHTML: function(text) {
            if (!text) return '';
            var div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
    },

    // Cache management with eviction
    _cacheManager: {
        evictOldestImages: function() {
            if (!Module.imageCache) return;
            var keys = Object.keys(Module.imageCache);
            if (keys.length <= Module._CONSTANTS.MAX_IMAGE_CACHE_SIZE) return;

            // Remove oldest 20% when limit exceeded
            var toRemove = Math.floor(keys.length * 0.2);
            for (var i = 0; i < toRemove; i++) {
                delete Module.imageCache[keys[i]];
            }
        },
        addImage: function(idx, img) {
            if (!Module.imageCache) Module.imageCache = {};
            this.evictOldestImages();
            Module.imageCache[idx] = img;
        }
    },

    // Performance: Rendering context cache
    _getCtx: function() {
        if (!Module._cachedCtx) {
            Module._cachedCtx = Module.canvas.getContext('2d');
        }
        return Module._cachedCtx;
    },

    JS_DrawRectangle: function(x, y, w, h) {
        var ctx = Module._getCtx();
        ctx.fillRect(x, y, w, h);
        ctx.strokeRect(x, y, w, h);
    },
    JS_DrawRoundedRectangle: function(x, y, w, h, radius) {
        var ctx = Module._getCtx();
        ctx.beginPath();
        if (ctx.roundRect) {
            ctx.roundRect(x, y, w, h, radius);
        } else {
            ctx.rect(x, y, w, h);
        }
        ctx.fill();
        ctx.stroke();
    },
    JS_DrawLine: function(x1, y1, x2, y2) {
        var ctx = Module._getCtx();
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
    },
    JS_DrawText: function(str, x, y) {
        if (!Module._validation.isValidPointer(str)) return;
        var ctx = Module._getCtx();
        var s = UTF8ToString(str);
        ctx.fillText(s, x, y);
    },
    JS_DrawBitmap: function(idx, x, y) {
        // Image cache system
        if (!Module.imageCache) Module.imageCache = {};
        if (!Module.imageLoadQueue) Module.imageLoadQueue = {};

        var ctx = Module._getCtx();
        var img = Module.imageCache[idx];
        var size = Module._CONSTANTS.PLACEHOLDER_SIZE;

        if (img && img.complete && img.naturalWidth > 0) {
            // Image loaded, draw it
            ctx.drawImage(img, x, y);
        } else if (!Module.imageLoadQueue[idx]) {
            // Start loading this image
            Module.imageLoadQueue[idx] = true;
            img = new Image();
            img.onload = function() {
                // Only cache after successful load
                Module._cacheManager.addImage(idx, img);
                delete Module.imageLoadQueue[idx];
                // Trigger redraw to show the loaded image
                if (Module._WASM_Resize) {
                    var w = Module.canvas.width;
                    var h = Module.canvas.height;
                    Module._WASM_Resize(w, h);
                }
            };
            img.onerror = function() {
                delete Module.imageLoadQueue[idx];
                console.warn('Failed to load image:', idx);
                // Create minimal placeholder image for cache
                var placeholder = new Image();
                placeholder.width = size;
                placeholder.height = size;
                Module._cacheManager.addImage(idx, placeholder);
            };
            // Set src to start loading (do NOT cache before load completes)
            img.src = 'images/icon' + idx + '.png';

            // Draw placeholder while loading
            ctx.fillStyle = 'rgba(200,200,200,0.3)';
            ctx.fillRect(x, y, size, size);
            ctx.strokeStyle = 'rgba(150,150,150,0.5)';
            ctx.strokeRect(x, y, size, size);
        } else {
            // Loading in progress, draw placeholder
            ctx.fillStyle = 'rgba(200,200,200,0.3)';
            ctx.fillRect(x, y, size, size);
        }
    },
    JS_LoadImage: function(idxOrPath, pathPtr) {
        // Helper to pre-load images with custom paths
        if (!Module.imageCache) Module.imageCache = {};
        var path = pathPtr ? UTF8ToString(pathPtr) : ('images/icon' + idxOrPath + '.png');
        var img = new Image();
        img.src = path;
        Module.imageCache[idxOrPath] = img;
    },
    JS_GetCharHeight: function() {
        var ctx = Module._getCtx();
        // Use TextMetrics to get accurate font height
        var metrics = ctx.measureText('M'); // Use 'M' as it's typically the tallest character
        var height = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
        // Fallback to font size parsing if TextMetrics not fully supported
        if (!height || height < 1) {
            var fontSizeMatch = ctx.font.match(/(\d+)px/);
            height = fontSizeMatch ? parseInt(fontSizeMatch[1]) : 12;
        }
        return Math.ceil(height);
    },
    JS_GetTextWidth: function(str) {
        var ctx = Module._getCtx();
        var s = UTF8ToString(str);
        return Math.ceil(ctx.measureText(s).width);
    },
    JS_GetTextHeight: function(str) {
        var ctx = Module._getCtx();
        var s = UTF8ToString(str);
        var metrics = ctx.measureText(s);
        // Calculate actual text height using bounding box
        var height = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
        // Fallback for browsers with limited TextMetrics support
        if (!height || height < 1) {
            var fontSizeMatch = ctx.font.match(/(\d+)px/);
            height = fontSizeMatch ? parseInt(fontSizeMatch[1]) : 12;
        }
        return Math.ceil(height);
    },
    JS_SetBrushColor: function(c) {
        var ctx = Module._getCtx();
        ctx.fillStyle = Module._colorUtils.toRGBString(c);
    },
    JS_SetPenColor: function(c) {
        var ctx = Module._getCtx();
        ctx.strokeStyle = Module._colorUtils.toRGBString(c);
    },
    JS_SetTextForeground: function(c) {
        var ctx = Module._getCtx();
        ctx.fillStyle = Module._colorUtils.toRGBString(c);
    },
    JS_SetTextBackground: function(c) {
        // Background color not currently used in canvas rendering
    },
    JS_SetFont: function(size, stylebits) {
        var ctx = Module._getCtx();
        var constants = Module._CONSTANTS;

        // Validate and clamp font size
        size = Module._validation.clampFontSize(size);

        var font = '';
        if (stylebits & constants.FONT_ITALIC) font += 'italic ';
        if (stylebits & constants.FONT_BOLD) font += 'bold ';
        font += size + 'px ';
        if (stylebits & constants.FONT_MONOSPACE) font += 'monospace';
        else font += 'sans-serif';
        ctx.font = font;
    },
    JS_SetPen: function(penType) {
        var ctx = Module._getCtx();
        // Pen enum values from ts_graphics.h:
        // 0: PEN_GRIDLINES, 1: PEN_TINYGRIDLINES, 2: PEN_THINSELECT
        // 3: PEN_TINYTEXT, 4: PEN_RED, 5: PEN_LIGHT_GREY
        // 6: PEN_BLACK, 7: PEN_WHITE, 8: PEN_GREY
        switch (penType) {
            case 0: // PEN_GRIDLINES
                ctx.strokeStyle = 'rgb(200,200,200)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 1: // PEN_TINYGRIDLINES
                ctx.strokeStyle = 'rgb(230,230,230)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 2: // PEN_THINSELECT
                ctx.strokeStyle = 'rgb(0,120,215)'; // Blue selection
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 3: // PEN_TINYTEXT
                ctx.strokeStyle = 'rgb(100,100,100)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 4: // PEN_RED
                ctx.strokeStyle = 'rgb(255,0,0)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 5: // PEN_LIGHT_GREY
                ctx.strokeStyle = 'rgb(211,211,211)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 6: // PEN_BLACK
                ctx.strokeStyle = 'rgb(0,0,0)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 7: // PEN_WHITE
                ctx.strokeStyle = 'rgb(255,255,255)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            case 8: // PEN_GREY
                ctx.strokeStyle = 'rgb(128,128,128)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
                break;
            default:
                ctx.strokeStyle = 'rgb(0,0,0)';
                ctx.lineWidth = 1;
                ctx.setLineDash([]);
        }
    },
    JS_SetBrush: function(brushType) {
        var ctx = Module._getCtx();
        // Brush enum values from ts_graphics.h:
        // 0: BRUSH_TRANSPARENT, 1: BRUSH_WHITE, 2: BRUSH_BLACK, 3: BRUSH_LIGHT_GREY
        switch (brushType) {
            case 0: // BRUSH_TRANSPARENT
                Module._currentBrush = 'transparent';
                break;
            case 1: // BRUSH_WHITE
                Module._currentBrush = 'rgb(255,255,255)';
                break;
            case 2: // BRUSH_BLACK
                Module._currentBrush = 'rgb(0,0,0)';
                break;
            case 3: // BRUSH_LIGHT_GREY
                Module._currentBrush = 'rgb(211,211,211)';
                break;
            default:
                Module._currentBrush = 'rgb(255,255,255)';
        }
        // Apply immediately to fillStyle
        ctx.fillStyle = Module._currentBrush;
    },

    JS_DownloadFile: function(filenamePtr, dataPtr, size) {
        var filename = UTF8ToString(filenamePtr);
        var data = new Uint8Array(Module.HEAPU8.buffer, dataPtr, size);
        var blob = new Blob([data], { type: 'application/octet-stream' });
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    },
    JS_LaunchBrowser: function(urlPtr) {
        var url = UTF8ToString(urlPtr);
        window.open(url, '_blank');
    },
    JS_SetClipboardText: function(textPtr) {
        var text = UTF8ToString(textPtr);

        // Modern Clipboard API
        if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text).catch(function(err) {
                console.warn('Clipboard write failed, trying fallback:', err);
                Module._clipboardFallbackCopy(text);
            });
        } else {
            // Legacy fallback for older browsers
            Module._clipboardFallbackCopy(text);
        }
    },
    JS_GetClipboardText: function() {
        // Modern Clipboard API with Asyncify
        if (navigator.clipboard && navigator.clipboard.readText) {
            return Asyncify.handleSleep(function(wakeUp) {
                navigator.clipboard.readText()
                    .then(function(text) {
                        var len = lengthBytesUTF8(text) + 1;
                        var ptr = _malloc(len);
                        if (!ptr) {
                            console.error('Failed to allocate memory for clipboard text');
                            wakeUp(0);
                            return;
                        }
                        stringToUTF8(text, ptr, len);
                        wakeUp(ptr);
                    })
                    .catch(function(err) {
                        console.warn('Clipboard read failed:', err);
                        wakeUp(0);
                    });
            });
        } else {
            // Fallback: return empty string for browsers without clipboard API
            console.warn('Clipboard API not available');
            return 0;
        }
    },

    // Clipboard fallback helper
    _clipboardFallbackCopy: function(text) {
        var textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.left = '-9999px';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        try {
            var success = document.execCommand('copy');
            if (!success) {
                console.error('execCommand copy failed');
            }
        } catch (err) {
            console.error('Fallback clipboard copy failed:', err);
        }
        document.body.removeChild(textarea);
    },

    // Virtual File System using LocalStorage
    _VFS: {
        saveFile: function(filename, data) {
            try {
                // Store file data as base64 to handle binary data
                var base64 = btoa(String.fromCharCode.apply(null, data));
                localStorage.setItem('treesheets_file_' + filename, base64);
                console.log('Saved file to VFS:', filename, data.length, 'bytes');
                return true;
            } catch (e) {
                console.error('VFS saveFile failed:', e);
                return false;
            }
        },
        loadFile: function(filename) {
            try {
                var base64 = localStorage.getItem('treesheets_file_' + filename);
                if (!base64) return null;
                // Decode base64 back to Uint8Array
                var binary = atob(base64);
                var bytes = new Uint8Array(binary.length);
                for (var i = 0; i < binary.length; i++) {
                    bytes[i] = binary.charCodeAt(i);
                }
                console.log('Loaded file from VFS:', filename, bytes.length, 'bytes');
                return bytes;
            } catch (e) {
                console.error('VFS loadFile failed:', e);
                return null;
            }
        },
        listFiles: function() {
            var files = [];
            try {
                for (var i = 0; i < localStorage.length; i++) {
                    var key = localStorage.key(i);
                    if (key && key.startsWith('treesheets_file_')) {
                        files.push(key.substring(16)); // Remove 'treesheets_file_' prefix
                    }
                }
            } catch (e) {
                console.error('VFS listFiles failed:', e);
            }
            return files;
        }
    },

    // LocalStorage Integration
    JS_LocalStorage_SetItem: function(keyPtr, valuePtr) {
        try {
            var key = UTF8ToString(keyPtr);
            var value = UTF8ToString(valuePtr);
            localStorage.setItem('treesheets_' + key, value);
        } catch (e) {
            console.warn('LocalStorage setItem failed:', e);
        }
    },
    JS_LocalStorage_GetItem: function(keyPtr) {
        try {
            var key = UTF8ToString(keyPtr);
            var value = localStorage.getItem('treesheets_' + key);
            if (value === null) return 0;
            var len = lengthBytesUTF8(value) + 1;
            var ptr = _malloc(len);
            stringToUTF8(value, ptr, len);
            return ptr;
        } catch (e) {
            console.warn('LocalStorage getItem failed:', e);
            return 0;
        }
    },
    JS_LocalStorage_RemoveItem: function(keyPtr) {
        try {
            var key = UTF8ToString(keyPtr);
            localStorage.removeItem('treesheets_' + key);
        } catch (e) {
            console.warn('LocalStorage removeItem failed:', e);
        }
    },
    JS_LocalStorage_Clear: function() {
        try {
            var keys = [];
            for (var i = 0; i < localStorage.length; i++) {
                var key = localStorage.key(i);
                if (key && key.startsWith('treesheets_')) {
                    keys.push(key);
                }
            }
            keys.forEach(function(key) {
                localStorage.removeItem(key);
            });
        } catch (e) {
            console.warn('LocalStorage clear failed:', e);
        }
    },

    JS_ReadFile: function(filenamePtr) {
        var filename = UTF8ToString(filenamePtr);
        var bytes = Module._VFS.loadFile(filename);

        if (!bytes || bytes.length === 0) {
            console.warn('File not found in VFS:', filename);
            return 0; // Return null pointer if file not found
        }

        // Allocate memory and copy file data
        var ptr = _malloc(bytes.length);
        if (!ptr) {
            console.error('Failed to allocate memory for file read');
            return 0;
        }

        Module.HEAPU8.set(bytes, ptr);

        // Store size for caller to retrieve
        Module._lastReadFileSize = bytes.length;

        return ptr;
    },
    JS_GetLastFileSize: function() {
        return Module._lastReadFileSize || 0;
    },

    JS_TriggerUpload: function() {
        var input = document.createElement('input');
        input.type = 'file';
        input.accept = '.cts';
        input.onchange = function(e) {
            var file = e.target.files[0];
            if (!file) {
                // Clean up input element
                if (input.parentNode) input.parentNode.removeChild(input);
                return;
            }

            // Security: Check file size
            var maxSize = Module._CONSTANTS.MAX_FILE_SIZE_BYTES;
            if (file.size > maxSize) {
                Module._createModal('Error', '<p>File too large. Maximum size is ' +
                    Math.floor(maxSize / (1024 * 1024)) + 'MB.</p>', [
                    { text: 'OK', primary: true }
                ]);
                // Clean up input element
                if (input.parentNode) input.parentNode.removeChild(input);
                return;
            }

            var reader = new FileReader();
            reader.onload = function(evt) {
                try {
                    var arrayBuffer = evt.target.result;
                    var uint8Array = new Uint8Array(arrayBuffer);

                    // Check malloc succeeded
                    var ptr = Module._malloc(uint8Array.length);
                    if (!ptr) {
                        Module._createModal('Error', '<p>Failed to allocate memory for file.</p>', [
                            { text: 'OK', primary: true }
                        ]);
                        return;
                    }
                    Module.HEAPU8.set(uint8Array, ptr);

                    var nameLen = lengthBytesUTF8(file.name) + 1;
                    var namePtr = Module._malloc(nameLen);
                    if (!namePtr) {
                        Module._free(ptr);
                        Module._createModal('Error', '<p>Failed to allocate memory for filename.</p>', [
                            { text: 'OK', primary: true }
                        ]);
                        return;
                    }
                    stringToUTF8(file.name, namePtr, nameLen);

                    // Save file to VFS for later reading
                    Module._VFS.saveFile(file.name, uint8Array);

                    Module._WASM_FileLoaded(namePtr, ptr, uint8Array.length);

                    Module._free(ptr);
                    Module._free(namePtr);
                } catch (err) {
                    console.error('Error processing uploaded file:', err);
                    Module._createModal('Error', '<p>Failed to process file: ' + err.message + '</p>', [
                        { text: 'OK', primary: true }
                    ]);
                } finally {
                    // Clean up input element
                    if (input.parentNode) input.parentNode.removeChild(input);
                }
            };
            reader.onerror = function(err) {
                console.error('FileReader error:', err);
                Module._createModal('Error', '<p>Failed to read file.</p>', [
                    { text: 'OK', primary: true }
                ]);
                // Clean up input element
                if (input.parentNode) input.parentNode.removeChild(input);
            };
            reader.readAsArrayBuffer(file);
        };
        input.click();
    },

    JS_InitInput: function() {
        // Prevent duplicate initialization (avoids event listener accumulation)
        if (Module._inputInitialized) {
            console.warn('JS_InitInput: Already initialized, skipping duplicate call');
            return;
        }
        Module._inputInitialized = true;

        var canvas = Module.canvas;

        // Helper to build modifier flags
        var getModifiers = function(e) {
            var mods = 0;
            if (e.ctrlKey)  mods |= 1;  // Ctrl = bit 0
            if (e.shiftKey) mods |= 2;  // Shift = bit 1
            if (e.altKey)   mods |= 4;  // Alt = bit 2
            if (e.metaKey)  mods |= 8;  // Meta/Command = bit 3
            return mods;
        };

        // Mouse events with modifiers
        canvas.addEventListener('mousedown', function(e) {
            Module._WASM_Mouse(1, e.offsetX, e.offsetY, getModifiers(e));
        });
        canvas.addEventListener('mouseup', function(e) {
            Module._WASM_Mouse(2, e.offsetX, e.offsetY, getModifiers(e));
        });

        // Throttle mousemove using requestAnimationFrame (~60fps)
        var mousemoveScheduled = false;
        var lastMouseEvent = null;
        canvas.addEventListener('mousemove', function(e) {
            lastMouseEvent = e;
            if (!mousemoveScheduled) {
                mousemoveScheduled = true;
                requestAnimationFrame(function() {
                    if (lastMouseEvent) {
                        Module._WASM_Mouse(0, lastMouseEvent.offsetX, lastMouseEvent.offsetY, getModifiers(lastMouseEvent));
                    }
                    mousemoveScheduled = false;
                });
            }
        });

        // Mouse wheel support
        canvas.addEventListener('wheel', function(e) {
            e.preventDefault();
            // Type 3 for wheel, pass deltaY as x coordinate (negative = scroll up)
            var delta = e.deltaY;
            // Normalize wheel delta (different browsers use different scales)
            if (e.deltaMode === 1) delta *= 16; // LINE mode
            if (e.deltaMode === 2) delta *= 100; // PAGE mode
            Module._WASM_Mouse(3, Math.round(delta), 0, getModifiers(e));
        }, { passive: false });

        // Touch support for mobile/tablet
        var touchState = {
            lastTouchX: 0,
            lastTouchY: 0,
            touchStartTime: 0,
            initialDistance: 0,
            isPinching: false
        };

        var getTouchPos = function(touch) {
            var rect = canvas.getBoundingClientRect();
            return {
                x: Math.round(touch.clientX - rect.left),
                y: Math.round(touch.clientY - rect.top)
            };
        };

        var getTouchDistance = function(t1, t2) {
            var dx = t1.clientX - t2.clientX;
            var dy = t1.clientY - t2.clientY;
            return Math.sqrt(dx * dx + dy * dy);
        };

        canvas.addEventListener('touchstart', function(e) {
            e.preventDefault();
            touchState.touchStartTime = Date.now();

            if (e.touches.length === 1) {
                // Single touch - treat as mouse down
                var pos = getTouchPos(e.touches[0]);
                touchState.lastTouchX = pos.x;
                touchState.lastTouchY = pos.y;
                touchState.isPinching = false;
                Module._WASM_Mouse(1, pos.x, pos.y, 0);
            } else if (e.touches.length === 2) {
                // Two finger pinch/zoom
                touchState.isPinching = true;
                touchState.initialDistance = getTouchDistance(e.touches[0], e.touches[1]);
            }
        }, { passive: false });

        canvas.addEventListener('touchmove', function(e) {
            e.preventDefault();

            if (e.touches.length === 1 && !touchState.isPinching) {
                // Single touch - treat as mouse move/drag
                var pos = getTouchPos(e.touches[0]);
                Module._WASM_Mouse(0, pos.x, pos.y, 0);
                touchState.lastTouchX = pos.x;
                touchState.lastTouchY = pos.y;
            } else if (e.touches.length === 2) {
                // Pinch zoom - simulate wheel event
                var currentDistance = getTouchDistance(e.touches[0], e.touches[1]);
                var delta = (touchState.initialDistance - currentDistance) * 2;
                Module._WASM_Mouse(3, Math.round(delta), 0, 0);
                touchState.initialDistance = currentDistance;
            }
        }, { passive: false });

        canvas.addEventListener('touchend', function(e) {
            e.preventDefault();

            if (e.touches.length === 0) {
                // All touches ended
                var touchDuration = Date.now() - touchState.touchStartTime;

                // Detect tap (short touch without much movement)
                if (touchDuration < Module._CONSTANTS.TAP_DURATION_MS && !touchState.isPinching) {
                    // Send mouse up at last position
                    Module._WASM_Mouse(2, touchState.lastTouchX, touchState.lastTouchY, 0);
                } else if (!touchState.isPinching) {
                    Module._WASM_Mouse(2, touchState.lastTouchX, touchState.lastTouchY, 0);
                }

                touchState.isPinching = false;
            }
        }, { passive: false });

        canvas.addEventListener('touchcancel', function(e) {
            e.preventDefault();
            touchState.isPinching = false;
        }, { passive: false });

        // Keyboard events with modifiers
        window.addEventListener('keydown', function(e) {
            // Prevent default for common shortcuts to avoid browser interference
            if ((e.ctrlKey || e.metaKey) && ['s', 'o', 'n', 'w', 'z', 'y', 'x', 'c', 'v', 'a', 'f'].indexOf(e.key.toLowerCase()) !== -1) {
                e.preventDefault();
            }
            Module._WASM_Key(0, e.keyCode, getModifiers(e));
        });
        window.addEventListener('keyup', function(e) {
            Module._WASM_Key(1, e.keyCode, getModifiers(e));
        });

        var onResize = function() {
            var w = canvas.clientWidth;
            var h = canvas.clientHeight;
            if (canvas.width != w || canvas.height != h) {
                canvas.width = w;
                canvas.height = h;
                Module._WASM_Resize(w, h);
            }
        };

        // Debounce resize events (150ms delay)
        var resizeTimeout;
        var debouncedResize = function() {
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(onResize, 150);
        };
        window.addEventListener('resize', debouncedResize);
        onResize(); // Call immediately on init
    },

    // Menus
    JS_Menu_Create: function(id, titlePtr) {
        var title = UTF8ToString(titlePtr);
        if (!Module.menus) Module.menus = {};
        Module.menus[id] = { title: title, items: [] };
    },
    JS_Menu_Append: function(parentId, id, textPtr, helpPtr, type, checked) {
        var text = UTF8ToString(textPtr);
        if (Module.menus[parentId]) {
            Module.menus[parentId].items.push({id: id, text: text, type: type, checked: checked});
        }
    },
    JS_MenuBar_Append: function(menuId, titlePtr) {
        var title = UTF8ToString(titlePtr);
        var menu = Module.menus[menuId];
        var menubar = document.getElementById('menubar');
        if (!menubar) {
            menubar = document.createElement('div');
            menubar.id = 'menubar';
            menubar.style.backgroundColor = '#e0e0e0';
            menubar.style.padding = '5px';
            menubar.style.display = 'flex';
            menubar.style.gap = '10px';
            menubar.setAttribute('role', 'menubar');
            menubar.setAttribute('aria-label', 'Main menu');
            if (Module.canvas && Module.canvas.parentNode) {
                Module.canvas.parentNode.insertBefore(menubar, Module.canvas);
            } else {
                document.body.prepend(menubar);
            }
        }

        var btn = document.createElement('button');
        btn.innerText = title.replace('&', '');
        btn.style.border = 'none';
        btn.style.background = 'none';
        btn.style.cursor = 'pointer';
        btn.style.fontSize = '14px';
        btn.setAttribute('role', 'menuitem');
        btn.setAttribute('aria-haspopup', 'true');
        btn.setAttribute('aria-label', title.replace('&', ''));

        var openMenu = function(e) {
            var oldPopup = document.getElementById('menu-popup');
            if (oldPopup) oldPopup.remove();

            var popup = document.createElement('div');
            popup.id = 'menu-popup';
            popup.style.position = 'absolute';
            popup.style.left = e.pageX + 'px';
            popup.style.top = (e.pageY + 20) + 'px';
            popup.style.backgroundColor = 'white';
            popup.style.border = '1px solid #ccc';
            popup.style.boxShadow = '2px 2px 5px rgba(0,0,0,0.2)';
            popup.style.zIndex = 1000;
            popup.setAttribute('role', 'menu');
            popup.setAttribute('aria-label', title.replace('&', '') + ' menu');

            var menuItems = [];
            menu.items.forEach(function(item, index) {
                if (item.type == 3) { // Separator
                    var sep = document.createElement('hr');
                    sep.style.margin = '2px 0';
                    sep.setAttribute('role', 'separator');
                    popup.appendChild(sep);
                    return;
                }
                var itemDiv = document.createElement('div');
                itemDiv.innerText = item.text.replace(/&/g, '').split('\t')[0];
                itemDiv.style.padding = '5px 15px';
                itemDiv.style.cursor = 'pointer';
                itemDiv.setAttribute('role', 'menuitem');
                itemDiv.setAttribute('tabindex', '0');
                itemDiv.setAttribute('aria-label', item.text.replace(/&/g, '').split('\t')[0]);
                itemDiv.onmouseover = function() { itemDiv.style.backgroundColor = '#eee'; };
                itemDiv.onmouseout = function() { itemDiv.style.backgroundColor = 'white'; };
                itemDiv.onclick = function() {
                    Module._WASM_Action(item.id);
                    popup.remove();
                    document.removeEventListener('keydown', menuKeyHandler);
                    window.removeEventListener('mousedown', closeHandler);
                };
                menuItems.push(itemDiv);
                popup.appendChild(itemDiv);
            });

            document.body.appendChild(popup);

            // Keyboard navigation for menu
            var currentIndex = -1;
            var menuKeyHandler = function(ev) {
                if (ev.key === 'ArrowDown') {
                    ev.preventDefault();
                    currentIndex = (currentIndex + 1) % menuItems.length;
                    menuItems[currentIndex].focus();
                    menuItems[currentIndex].style.backgroundColor = '#eee';
                } else if (ev.key === 'ArrowUp') {
                    ev.preventDefault();
                    currentIndex = currentIndex <= 0 ? menuItems.length - 1 : currentIndex - 1;
                    menuItems[currentIndex].focus();
                    menuItems[currentIndex].style.backgroundColor = '#eee';
                } else if (ev.key === 'Enter' || ev.key === ' ') {
                    ev.preventDefault();
                    if (currentIndex >= 0) menuItems[currentIndex].click();
                } else if (ev.key === 'Escape') {
                    ev.preventDefault();
                    popup.remove();
                    document.removeEventListener('keydown', menuKeyHandler);
                    window.removeEventListener('mousedown', closeHandler);
                    btn.focus();
                }
            };
            document.addEventListener('keydown', menuKeyHandler);

            // Focus first item
            if (menuItems.length > 0) {
                currentIndex = 0;
                menuItems[0].focus();
                menuItems[0].style.backgroundColor = '#eee';
            }

            var closeHandler = function(ev) {
                if (ev.target != btn && !popup.contains(ev.target)) {
                    popup.remove();
                    document.removeEventListener('keydown', menuKeyHandler);
                    window.removeEventListener('mousedown', closeHandler);
                }
            };
            setTimeout(function() { window.addEventListener('mousedown', closeHandler); }, 0);
        };

        btn.onclick = openMenu;
        btn.onkeydown = function(e) {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                openMenu(e);
            }
        };
        menubar.appendChild(btn);
    },

    // Dialog Helper - Creates styled modal
    _createModal: function(title, content, buttons, onClose) {
        // Remove any existing modal
        var existing = document.getElementById('ts-modal');
        if (existing) existing.remove();

        // Store previously focused element for restoration
        var previouslyFocused = document.activeElement;

        // Create overlay
        var overlay = document.createElement('div');
        overlay.id = 'ts-modal';
        overlay.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:10000;';
        overlay.setAttribute('role', 'dialog');
        overlay.setAttribute('aria-modal', 'true');
        overlay.setAttribute('aria-labelledby', 'modal-title');

        // Create dialog
        var dialog = document.createElement('div');
        dialog.style.cssText = 'background:white;border-radius:8px;box-shadow:0 4px 20px rgba(0,0,0,0.3);min-width:300px;max-width:500px;max-height:80%;overflow:auto;';
        dialog.setAttribute('tabindex', '-1');

        // Title bar
        var titleBar = document.createElement('div');
        titleBar.id = 'modal-title';
        titleBar.style.cssText = 'background:#f0f0f0;padding:12px 16px;border-bottom:1px solid #ddd;font-weight:bold;border-radius:8px 8px 0 0;';
        titleBar.innerText = title;
        dialog.appendChild(titleBar);

        // Content
        var contentDiv = document.createElement('div');
        contentDiv.style.cssText = 'padding:16px;';
        if (typeof content === 'string') {
            contentDiv.innerHTML = content;
        } else {
            contentDiv.appendChild(content);
        }
        dialog.appendChild(contentDiv);

        // Button bar
        var primaryButton = null;
        var cancelButton = null;
        if (buttons && buttons.length > 0) {
            var buttonBar = document.createElement('div');
            buttonBar.style.cssText = 'padding:12px 16px;border-top:1px solid #ddd;text-align:right;display:flex;gap:8px;justify-content:flex-end;';
            buttons.forEach(function(btn) {
                var button = document.createElement('button');
                button.innerText = btn.text;
                button.style.cssText = 'padding:6px 16px;border:1px solid #ccc;border-radius:4px;background:' + (btn.primary ? '#007bff' : '#fff') + ';color:' + (btn.primary ? '#fff' : '#000') + ';cursor:pointer;';
                button.onmouseover = function() { button.style.opacity = '0.8'; };
                button.onmouseout = function() { button.style.opacity = '1'; };
                button.onclick = function() {
                    overlay.remove();
                    document.removeEventListener('keydown', keyHandler);
                    // Restore focus
                    if (previouslyFocused) previouslyFocused.focus();
                    if (btn.callback) btn.callback();
                };
                // Add ARIA label
                button.setAttribute('aria-label', btn.text);
                if (btn.primary) {
                    primaryButton = button;
                    button.setAttribute('aria-describedby', 'modal-title');
                }
                if (btn.text.toLowerCase() === 'cancel') cancelButton = button;
                buttonBar.appendChild(button);
            });
            dialog.appendChild(buttonBar);
        }

        overlay.appendChild(dialog);
        document.body.appendChild(overlay);

        // Focus management - trap focus within modal
        var focusableElements = dialog.querySelectorAll('button, input, select, textarea, [tabindex]:not([tabindex="-1"])');
        var firstFocusable = focusableElements[0];
        var lastFocusable = focusableElements[focusableElements.length - 1];

        // Set initial focus
        if (firstFocusable) {
            setTimeout(function() { firstFocusable.focus(); }, 100);
        }

        // Focus trap handler
        var focusTrap = function(e) {
            if (e.key === 'Tab') {
                if (e.shiftKey) {
                    // Shift+Tab
                    if (document.activeElement === firstFocusable) {
                        e.preventDefault();
                        lastFocusable.focus();
                    }
                } else {
                    // Tab
                    if (document.activeElement === lastFocusable) {
                        e.preventDefault();
                        firstFocusable.focus();
                    }
                }
            }
        };

        // Keyboard controls (Enter = OK/primary, Esc = Cancel)
        var keyHandler = function(e) {
            focusTrap(e);

            if (e.key === 'Enter' && primaryButton) {
                e.preventDefault();
                primaryButton.click();
            } else if (e.key === 'Escape') {
                e.preventDefault();
                if (cancelButton) {
                    cancelButton.click();
                } else {
                    overlay.remove();
                    document.removeEventListener('keydown', keyHandler);
                    // Restore focus
                    if (previouslyFocused) previouslyFocused.focus();
                    if (onClose) onClose();
                }
            }
        };
        document.addEventListener('keydown', keyHandler);

        // Close on overlay click
        overlay.onclick = function(e) {
            if (e.target === overlay) {
                overlay.remove();
                document.removeEventListener('keydown', keyHandler);
                // Restore focus
                if (previouslyFocused) previouslyFocused.focus();
                if (onClose) onClose();
            }
        };

        return { overlay: overlay, dialog: dialog, content: contentDiv };
    },

    // Dialogs
    JS_ShowMessage: function(titlePtr, msgPtr) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);

        // Security: Create safe content with textContent
        var content = document.createElement('div');
        var p = document.createElement('p');
        p.textContent = msg;
        // Preserve newlines by converting to <br> tags safely
        p.innerHTML = p.textContent.replace(/\n/g, '<br>');
        content.appendChild(p);

        Module._createModal(title, content, [
            { text: 'OK', primary: true }
        ]);
    },
    JS_AskText: function(titlePtr, msgPtr, defPtr) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);
        var def = UTF8ToString(defPtr);

        var resultPtr = null;
        var input = document.createElement('input');
        input.type = 'text';
        input.value = def;
        input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';

        var content = document.createElement('div');
        var p = document.createElement('p');
        p.textContent = msg;
        p.innerHTML = p.textContent.replace(/\n/g, '<br>');
        content.appendChild(p);
        content.appendChild(input);

        // Use promise-based approach for proper async handling
        return Asyncify.handleSleep(function(wakeUp) {
            Module._createModal(title, content, [
                { text: 'Cancel', callback: function() {
                    var len = 1;
                    resultPtr = _malloc(len);
                    stringToUTF8('', resultPtr, len);
                    wakeUp(resultPtr);
                }},
                { text: 'OK', primary: true, callback: function() {
                    var res = input.value || '';
                    var len = lengthBytesUTF8(res) + 1;
                    resultPtr = _malloc(len);
                    stringToUTF8(res, resultPtr, len);
                    wakeUp(resultPtr);
                }}
            ]);

            // Focus input after a brief delay
            setTimeout(function() { input.focus(); }, 100);
        });
    },
    JS_AskNumber: function(titlePtr, msgPtr, def, min, max) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);

        var input = document.createElement('input');
        input.type = 'number';
        input.value = def;
        input.min = min;
        input.max = max;
        input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';

        var content = document.createElement('div');
        var p = document.createElement('p');
        p.textContent = msg;
        p.innerHTML = p.textContent.replace(/\n/g, '<br>');
        content.appendChild(p);
        content.appendChild(input);

        // Use promise-based approach for proper async handling
        return Asyncify.handleSleep(function(wakeUp) {
            Module._createModal(title, content, [
                { text: 'Cancel', callback: function() {
                    wakeUp(def);
                }},
                { text: 'OK', primary: true, callback: function() {
                    var result = parseFloat(input.value);
                    if (isNaN(result)) result = def;
                    wakeUp(result);
                }}
            ]);

            // Focus input after a brief delay
            setTimeout(function() { input.focus(); }, 100);
        });
    },
    JS_SingleChoice: function(titlePtr, msgPtr, choicesJsonPtr) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);

        // Add error handling for JSON parsing
        var choices;
        try {
            choices = JSON.parse(UTF8ToString(choicesJsonPtr));
        } catch (e) {
            console.error('JS_SingleChoice: Invalid JSON for choices:', e);
            return 0;
        }

        var content = document.createElement('div');
        var p = document.createElement('p');
        p.textContent = msg;
        p.innerHTML = p.textContent.replace(/\n/g, '<br>');
        content.appendChild(p);

        var select = document.createElement('select');
        select.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';
        choices.forEach(function(choice, i) {
            var option = document.createElement('option');
            option.value = i;
            option.text = choice;
            select.appendChild(option);
        });
        content.appendChild(select);

        // Use promise-based approach for proper async handling
        return Asyncify.handleSleep(function(wakeUp) {
            Module._createModal(title, content, [
                { text: 'Cancel', callback: function() {
                    wakeUp(0);
                }},
                { text: 'OK', primary: true, callback: function() {
                    var result = parseInt(select.value, 10);
                    if (isNaN(result)) result = 0;
                    wakeUp(result);
                }}
            ]);

            // Focus select after a brief delay
            setTimeout(function() { select.focus(); }, 100);
        });
    },
    JS_PickColor: function(defaultColor) {
        // Use color utility for hex conversion
        var hexColor = Module._colorUtils.toHex(defaultColor);

        var content = document.createElement('div');
        var input = document.createElement('input');
        input.type = 'color';
        input.value = hexColor;
        input.style.cssText = 'width:100%;height:100px;border:1px solid #ccc;border-radius:4px;cursor:pointer;';
        content.appendChild(input);

        // Use promise-based approach for proper async handling
        return Asyncify.handleSleep(function(wakeUp) {
            Module._createModal('Choose Color', content, [
                { text: 'Cancel', callback: function() {
                    wakeUp(defaultColor);
                }},
                { text: 'OK', primary: true, callback: function() {
                    var result = Module._colorUtils.fromHex(input.value);
                    wakeUp(result);
                }}
            ]);

            // Focus input after a brief delay
            setTimeout(function() { input.focus(); }, 100);
        });
    },

    // Toolbar
    JS_Toolbar_Create: function() {
        var toolbar = document.getElementById('toolbar');
        if (!toolbar) {
            toolbar = document.createElement('div');
            toolbar.id = 'toolbar';
            toolbar.style.backgroundColor = '#f0f0f0';
            toolbar.style.padding = '5px';
            toolbar.style.display = 'flex';
            toolbar.style.flexWrap = 'wrap';
            toolbar.style.gap = '5px';
            toolbar.style.alignItems = 'center';
            toolbar.style.borderBottom = '1px solid #ccc';
            toolbar.setAttribute('role', 'toolbar');
            toolbar.setAttribute('aria-label', 'TreeSheets Toolbar');
            var menubar = document.getElementById('menubar');
            if (menubar && menubar.parentNode) {
                menubar.parentNode.insertBefore(toolbar, menubar.nextSibling);
            } else if (Module.canvas && Module.canvas.parentNode) {
                Module.canvas.parentNode.insertBefore(toolbar, Module.canvas);
            } else {
                document.body.prepend(toolbar);
            }
        }
    },
    JS_Toolbar_AddTool: function(id, labelPtr, iconPtr) {
        var toolbar = document.getElementById('toolbar');
        if(!toolbar) return;
        var label = UTF8ToString(labelPtr);
        var iconPath = iconPtr ? UTF8ToString(iconPtr) : null;

        var btn = document.createElement('button');
        btn.title = label;
        btn.style.cssText = 'padding:4px 8px;border:1px solid #ccc;border-radius:3px;background:#fff;cursor:pointer;display:flex;align-items:center;gap:4px;';
        btn.setAttribute('aria-label', label);
        btn.onmouseover = function() { btn.style.background = '#e8e8e8'; };
        btn.onmouseout = function() { btn.style.background = '#fff'; };

        // Try to load icon if path provided
        if (iconPath) {
            var img = new Image();
            img.src = iconPath;
            img.width = 16;
            img.height = 16;
            img.style.display = 'inline-block';
            img.onerror = function() {
                // Fallback to text label if icon fails
                btn.innerText = label.split(' (')[0];
            };
            img.onload = function() {
                btn.appendChild(img);
                var span = document.createElement('span');
                span.innerText = label.split(' (')[0];
                span.style.fontSize = '12px';
                btn.appendChild(span);
            };
        } else {
            btn.innerText = label.split(' (')[0];
        }

        btn.onclick = function() { Module._WASM_Action(id); };
        toolbar.appendChild(btn);
    },
    JS_Toolbar_AddSeparator: function() {
        var toolbar = document.getElementById('toolbar');
        if(!toolbar) return;
        var sep = document.createElement('div');
        sep.style.width = '1px';
        sep.style.height = '20px';
        sep.style.backgroundColor = '#999';
        sep.style.margin = '0 5px';
        toolbar.appendChild(sep);
    },
    JS_Toolbar_AddLabel: function(labelPtr) {
        var toolbar = document.getElementById('toolbar');
        if(!toolbar) return;
        var span = document.createElement('span');
        span.innerText = UTF8ToString(labelPtr);
        span.style.fontSize = '12px';
        toolbar.appendChild(span);
    },
    JS_Toolbar_AddInput: function(id, width, defPtr) {
        var toolbar = document.getElementById('toolbar');
        if(!toolbar) return;
        var inp = document.createElement('input');
        inp.type = 'text';
        inp.value = UTF8ToString(defPtr);
        inp.style.width = width + 'px';
        inp.onkeydown = function(e) {
            if(e.key === 'Enter') {
                 Module._WASM_Action(id);
            }
        };
        toolbar.appendChild(inp);
    },
    JS_Toolbar_AddDropdown: function(id, width, choicesJsonPtr) {
        var toolbar = document.getElementById('toolbar');
        if(!toolbar) return;

        // Add error handling for JSON parsing
        var choices;
        try {
            choices = JSON.parse(UTF8ToString(choicesJsonPtr));
        } catch (e) {
            console.error('JS_Toolbar_AddDropdown: Invalid JSON for choices:', e);
            return;
        }

        var sel = document.createElement('select');
        sel.style.width = width + 'px';
        choices.forEach(function(c, i) {
            var opt = document.createElement('option');
            opt.value = i;
            opt.text = c.toString();
            sel.appendChild(opt);
        });
        sel.onchange = function() {
             Module._WASM_Action(id);
        };
        toolbar.appendChild(sel);
    }
});
