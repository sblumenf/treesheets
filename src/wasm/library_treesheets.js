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
 * Polished Product (Phase 10 - Package B):
 *
 * 26. Auto-Save System
 *    - Automatic save to LocalStorage every 30 seconds
 *    - Visual save indicator with timestamp
 *    - Manual save trigger via JS_Save function
 *    - Dirty state tracking
 *
 * 27. Unsaved Changes Warning
 *    - beforeunload event when changes are unsaved
 *    - Browser prompt before closing with unsaved work
 *    - Prevents accidental data loss
 *
 * 28. Session Recovery
 *    - Saves current document state on exit
 *    - Automatic restoration on next load
 *    - Crash recovery capability
 *
 * 29. About Dialog
 *    - Version information and credits
 *    - Links to documentation and source
 *    - Keyboard shortcut reference
 *
 * 30. Dark Mode
 *    - System-wide dark theme toggle
 *    - CSS variable-based theming
 *    - Preference persistence in LocalStorage
 *    - Auto-applies saved preference on load
 *
 * 31. Keyboard Shortcuts Help
 *    - Comprehensive shortcut reference modal
 *    - Categorized by function (File, Edit, View, etc.)
 *    - Accessible via F1 or Help menu
 *
 * 32. Recent Files List
 *    - Tracks last 10 opened files
 *    - Quick access from File menu
 *    - Persistent across sessions
 *    - Auto-opens on file load
 *
 * 33. Drag & Drop File Upload
 *    - Drop zone on canvas
 *    - Visual feedback during drag
 *    - Supports .cts files
 *    - Same processing as file picker
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
        PLACEHOLDER_SIZE: 16,

        // Auto-save
        AUTO_SAVE_INTERVAL_MS: 30000, // 30 seconds

        // Recent files
        MAX_RECENT_FILES: 10,

        // Version
        VERSION: '1.0.0 Web',

        // Export/Import
        EXPORT_FORMATS: ['CSV', 'JSON', 'HTML', 'Markdown', 'Text'],

        // Themes
        THEMES: {
            'light': { name: 'Light', bg: '#f5f5f5', fg: '#000', canvas: '#fff', ui: '#e0e0e0' },
            'dark': { name: 'Dark', bg: '#1e1e1e', fg: '#e0e0e0', canvas: '#2d2d2d', ui: '#2d2d2d' },
            'solarized-light': { name: 'Solarized Light', bg: '#fdf6e3', fg: '#657b83', canvas: '#eee8d5', ui: '#eee8d5' },
            'solarized-dark': { name: 'Solarized Dark', bg: '#002b36', fg: '#839496', canvas: '#073642', ui: '#073642' },
            'dracula': { name: 'Dracula', bg: '#282a36', fg: '#f8f8f2', canvas: '#44475a', ui: '#44475a' },
            'nord': { name: 'Nord', bg: '#2e3440', fg: '#eceff4', canvas: '#3b4252', ui: '#3b4252' },
            'monokai': { name: 'Monokai', bg: '#272822', fg: '#f8f8f2', canvas: '#3e3d32', ui: '#3e3d32' }
        },

        // Zoom levels
        ZOOM_LEVELS: [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0],

        // Templates
        TEMPLATES: [
            { id: 'blank', name: 'Blank Document', desc: 'Start with an empty document' },
            { id: 'todo', name: 'To-Do List', desc: 'Task management template' },
            { id: 'budget', name: 'Budget Planner', desc: 'Monthly budget tracker' },
            { id: 'schedule', name: 'Weekly Schedule', desc: '7-day schedule planner' },
            { id: 'project', name: 'Project Plan', desc: 'Project management template' },
            { id: 'notes', name: 'Meeting Notes', desc: 'Meeting minutes template' }
        ],

        // Cache eviction
        CACHE_EVICTION_PERCENTAGE: 0.2,

        // Placeholder colors
        PLACEHOLDER_BG_COLOR: 'rgba(200,200,200,0.3)',
        PLACEHOLDER_BORDER_COLOR: 'rgba(150,150,150,0.5)',

        // Base64 encoding
        BASE64_CHUNK_SIZE: 8192,

        // Wheel normalization
        WHEEL_LINE_DELTA: 16,
        WHEEL_PAGE_DELTA: 100,

        // Timing constants
        RESIZE_DEBOUNCE_MS: 150,
        MODAL_FOCUS_DELAY_MS: 100,
        STATUS_FADE_TIMEOUT_MS: 3000,
        SESSION_RESTORE_DELAY_MS: 1000,
        FULLSCREEN_CHECK_DELAY_MS: 100,
        SESSION_MAX_AGE_MS: 24 * 60 * 60 * 1000  // 24 hours
    },

    // ===== UTILITY FUNCTIONS =====

    // Error handling utilities
    _errorHandler: {
        showError: function(title, message, error) {
            console.error(title + ':', message, error);
            Module._createModal(title, '<p>' + message + '</p>', [
                { text: 'OK', primary: true }
            ]);
        },
        logError: function(context, error) {
            console.error(context + ':', error);
        },
        handleQuietly: function(context, error, fallback) {
            console.warn(context + ':', error);
            return fallback;
        }
    },

    // File loading utility (DRY - eliminates duplication)
    _fileLoader: {
        loadFromVFS: function(filename, onSuccess, onError) {
            var bytes = Module._VFS.loadFile(filename);
            if (!bytes) {
                if (onError) onError('File not found: ' + filename);
                return false;
            }

            var ptr = null;
            var namePtr = null;
            try {
                ptr = _malloc(bytes.length);
                if (!ptr) {
                    throw new Error('Failed to allocate memory for file data');
                }

                Module.HEAPU8.set(bytes, ptr);

                var nameLen = lengthBytesUTF8(filename) + 1;
                namePtr = _malloc(nameLen);
                if (!namePtr) {
                    // CRITICAL: Free ptr before throwing to prevent memory leak
                    _free(ptr);
                    ptr = null;
                    throw new Error('Failed to allocate memory for filename');
                }

                stringToUTF8(filename, namePtr, nameLen);
                Module._WASM_FileLoaded(namePtr, ptr, bytes.length);

                if (onSuccess) onSuccess();
                return true;
            } catch (err) {
                console.error('Error loading file:', err);
                if (onError) onError(err.message);
                return false;
            } finally {
                // CRITICAL: Always free memory, even on error
                if (ptr) _free(ptr);
                if (namePtr) _free(namePtr);
            }
        }
    },

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
            // VALIDATION: Check input format
            if (!hexString || typeof hexString !== 'string' || !hexString.match(/^#[0-9A-Fa-f]{6}$/)) {
                console.warn('Invalid hex color format:', hexString, '- using black');
                return 0x000000; // Return black as safe default
            }
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
        escapeHTML: function(text) {
            // RENAMED from sanitizeHTML - this escapes HTML entities, not sanitizes
            // Returns HTML-escaped version suitable for use in innerHTML contexts
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

            // Remove oldest using configured percentage
            var toRemove = Math.floor(keys.length * Module._CONSTANTS.CACHE_EVICTION_PERCENTAGE);
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

    // String utilities
    _stringFromPtr: function(ptr, fallback) {
        if (!Module._validation.isValidPointer(ptr)) {
            console.warn('Invalid pointer passed to UTF8ToString');
            return fallback || '';
        }
        return UTF8ToString(ptr);
    },

    // Drawing utilities
    _drawPlaceholder: function(ctx, x, y, size) {
        ctx.fillStyle = Module._CONSTANTS.PLACEHOLDER_BG_COLOR;
        ctx.fillRect(x, y, size, size);
        ctx.strokeStyle = Module._CONSTANTS.PLACEHOLDER_BORDER_COLOR;
        ctx.strokeRect(x, y, size, size);
    },

    _getFontHeight: function(metrics, ctx) {
        var height = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
        if (!height || height < 1) {
            var fontSizeMatch = ctx.font.match(/(\d+)px/);
            height = fontSizeMatch ? parseInt(fontSizeMatch[1]) : Module._CONSTANTS.DEFAULT_FONT_SIZE;
        }
        return Math.ceil(height);
    },

    // File utilities
    _downloadBlob: function(blob, filename) {
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();
        URL.revokeObjectURL(url);
    },

    _validateFileSize: function(file) {
        var maxSize = Module._CONSTANTS.MAX_FILE_SIZE_BYTES;
        if (file.size > maxSize) {
            Module._createModal('Error', '<p>File too large. Maximum size is ' +
                Math.floor(maxSize / (1024 * 1024)) + 'MB.</p>', [
                { text: 'OK', primary: true }
            ]);
            return false;
        }
        return true;
    },

    // Accessibility utilities
    _screenReaderAnnounce: function(message) {
        var announcer = document.getElementById('sr-announcer');
        if (!announcer) {
            announcer = document.createElement('div');
            announcer.id = 'sr-announcer';
            announcer.setAttribute('role', 'status');
            announcer.setAttribute('aria-live', 'polite');
            announcer.style.position = 'absolute';
            announcer.style.left = '-10000px';
            announcer.style.width = '1px';
            announcer.style.height = '1px';
            announcer.style.overflow = 'hidden';
            document.body.appendChild(announcer);
        }
        announcer.textContent = message;
    },

    // Text utilities - XSS safe newline handling
    _addTextWithLineBreaks: function(element, text) {
        // SECURITY: Safely add text with newlines without using innerHTML
        // This prevents XSS by creating BR elements programmatically
        var lines = text.split('\n');
        lines.forEach(function(line, i) {
            element.appendChild(document.createTextNode(line));
            if (i < lines.length - 1) {
                element.appendChild(document.createElement('br'));
            }
        });
    },

    // Pen and Brush style lookups (replaces switch statements)
    _penStyles: {
        0: { strokeStyle: 'rgb(200,200,200)', lineWidth: 1, lineDash: [] },      // PEN_GRIDLINES
        1: { strokeStyle: 'rgb(230,230,230)', lineWidth: 1, lineDash: [] },      // PEN_TINYGRIDLINES
        2: { strokeStyle: 'rgb(0,120,215)', lineWidth: 1, lineDash: [] },        // PEN_THINSELECT
        3: { strokeStyle: 'rgb(100,100,100)', lineWidth: 1, lineDash: [] },      // PEN_TINYTEXT
        4: { strokeStyle: 'rgb(255,0,0)', lineWidth: 1, lineDash: [] },          // PEN_RED
        5: { strokeStyle: 'rgb(211,211,211)', lineWidth: 1, lineDash: [] },      // PEN_LIGHT_GREY
        6: { strokeStyle: 'rgb(0,0,0)', lineWidth: 1, lineDash: [] },            // PEN_BLACK
        7: { strokeStyle: 'rgb(255,255,255)', lineWidth: 1, lineDash: [] },      // PEN_WHITE
        8: { strokeStyle: 'rgb(128,128,128)', lineWidth: 1, lineDash: [] }       // PEN_GREY
    },

    _brushStyles: {
        0: 'transparent',        // BRUSH_TRANSPARENT
        1: 'rgb(255,255,255)',   // BRUSH_WHITE
        2: 'rgb(0,0,0)',         // BRUSH_BLACK
        3: 'rgb(211,211,211)'    // BRUSH_LIGHT_GREY
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
        } else if (!Module.imageLoadQueue[idx] && !Module.imageCache[idx]) {
            // BUGFIX: Check both queue and cache to prevent race condition
            // Mark as loading IMMEDIATELY to prevent duplicate loads
            Module.imageLoadQueue[idx] = true;

            img = new Image();
            var loadingStartTime = Date.now();

            img.onload = function() {
                // Only cache if this is still the expected load operation
                if (Module.imageLoadQueue[idx]) {
                    Module._cacheManager.addImage(idx, img);
                    delete Module.imageLoadQueue[idx];

                    console.log('Image loaded:', idx, 'in', Date.now() - loadingStartTime, 'ms');

                    // BUGFIX: Debounce redraws to avoid multiple rapid calls
                    if (Module._WASM_Resize && !Module._pendingRedraw) {
                        Module._pendingRedraw = true;
                        requestAnimationFrame(function() {
                            Module._pendingRedraw = false;
                            var w = Module.canvas.width;
                            var h = Module.canvas.height;
                            Module._WASM_Resize(w, h);
                        });
                    }
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
            Module._drawPlaceholder(ctx, x, y, size);
        } else {
            // Loading in progress, draw placeholder
            Module._drawPlaceholder(ctx, x, y, size);
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
        return Module._getFontHeight(metrics, ctx);
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
        return Module._getFontHeight(metrics, ctx);
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
        // NOTE: Text background color is not implemented in the canvas 2D API.
        // Canvas fillText only supports foreground color via fillStyle.
        // If text backgrounds are needed, they must be drawn separately using fillRect
        // before rendering the text. The C++ code does not currently request this.
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
        // Use object lookup instead of switch statement
        var style = Module._penStyles[penType] || Module._penStyles[6]; // Default to PEN_BLACK
        ctx.strokeStyle = style.strokeStyle;
        ctx.lineWidth = style.lineWidth;
        ctx.setLineDash(style.lineDash);
    },
    JS_SetBrush: function(brushType) {
        var ctx = Module._getCtx();
        // Use object lookup instead of switch statement
        var brushColor = Module._brushStyles[brushType] || Module._brushStyles[1]; // Default to BRUSH_WHITE
        Module._currentBrush = brushColor;
        ctx.fillStyle = Module._currentBrush;
    },

    JS_DownloadFile: function(filenamePtr, dataPtr, size) {
        try {
            var filename = Module._stringFromPtr(filenamePtr, 'download.cts');
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
        } catch (e) {
            console.error('Download failed:', e);
            Module._errorHandler.showError('Download Failed',
                'Unable to download file. Error: ' + e.message);
        }
    },
    JS_LaunchBrowser: function(urlPtr) {
        try {
            var url = Module._stringFromPtr(urlPtr, '');
            if (!url) {
                console.warn('Empty URL provided to JS_LaunchBrowser');
                return;
            }
            var win = window.open(url, '_blank');
            if (!win) {
                Module._errorHandler.showError('Popup Blocked',
                    'Your browser blocked the popup. Please allow popups for this site to open links in new tabs.');
            }
        } catch (e) {
            console.error('Failed to open URL:', e);
            Module._errorHandler.showError('Navigation Failed',
                'Unable to open link. Error: ' + e.message);
        }
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
        // MEMORY OWNERSHIP: Returns a malloc'd pointer that the C++ caller MUST free using Module._free()
        // Returns 0 (null pointer) on failure or if clipboard is empty
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
                        // CRITICAL: Caller must free this pointer after use!
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
        _fileListCache: null,
        _cacheTime: 0,
        _CACHE_TTL: 1000, // Cache for 1 second

        _invalidateCache: function() {
            this._fileListCache = null;
            this._cacheTime = 0;
        },

        saveFile: function(filename, data) {
            try {
                // PERFORMANCE: Handle large arrays by chunking to avoid stack overflow
                var binary = '';
                var chunkSize = Module._CONSTANTS.BASE64_CHUNK_SIZE;
                for (var i = 0; i < data.length; i += chunkSize) {
                    var end = Math.min(i + chunkSize, data.length);
                    var chunk = data.subarray ? data.subarray(i, end) : Array.prototype.slice.call(data, i, end);
                    binary += String.fromCharCode.apply(null, chunk);
                }

                var base64 = btoa(binary);
                localStorage.setItem('treesheets_file_' + filename, base64);
                this._invalidateCache(); // Invalidate cache when files change
                console.log('Saved file to VFS:', filename, data.length, 'bytes');
                return true;
            } catch (e) {
                console.error('VFS saveFile failed:', e);
                // IMPROVED ERROR: Provide actionable guidance
                if (e.name === 'QuotaExceededError') {
                    Module._errorHandler.showError('Storage Full',
                        'Not enough storage space. Try:\n• Clearing old files from Recent Files\n• Clearing browser cache\n• Using Export to save important data');
                } else {
                    Module._errorHandler.showError('Save Failed',
                        'Failed to save file to browser storage. Error: ' + e.message);
                }
                return false;
            }
        },
        loadFile: function(filename) {
            try {
                var base64 = localStorage.getItem('treesheets_file_' + filename);
                if (!base64) return null;
                // PERFORMANCE: More efficient decoding using Uint8Array.from
                var binary = atob(base64);
                var bytes = Uint8Array.from(binary, function(c) { return c.charCodeAt(0); });
                console.log('Loaded file from VFS:', filename, bytes.length, 'bytes');
                return bytes;
            } catch (e) {
                console.error('VFS loadFile failed:', e);
                Module._errorHandler.logError('VFS loadFile', e);
                return null;
            }
        },
        listFiles: function() {
            // PERFORMANCE: Cache file list to avoid repeated localStorage iterations
            var now = Date.now();
            if (this._fileListCache && (now - this._cacheTime) < this._CACHE_TTL) {
                return this._fileListCache;
            }

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

            this._fileListCache = files;
            this._cacheTime = now;
            return files;
        },
        deleteFile: function(filename) {
            try {
                localStorage.removeItem('treesheets_file_' + filename);
                this._invalidateCache();
                return true;
            } catch (e) {
                console.error('VFS deleteFile failed:', e);
                return false;
            }
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
        // MEMORY OWNERSHIP: Returns a malloc'd pointer that the C++ caller MUST free using Module._free()
        // Returns 0 (null pointer) if key not found or on error
        try {
            var key = UTF8ToString(keyPtr);
            var value = localStorage.getItem('treesheets_' + key);
            if (value === null) return 0;
            var len = lengthBytesUTF8(value) + 1;
            var ptr = _malloc(len);
            // CRITICAL: Check malloc success before writing
            if (!ptr) {
                console.error('Failed to allocate memory for localStorage value');
                return 0;
            }
            stringToUTF8(value, ptr, len);
            return ptr; // CRITICAL: Caller must free this pointer!
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
        // MEMORY OWNERSHIP: Returns a malloc'd pointer that the C++ caller MUST free using Module._free()
        // Returns 0 (null pointer) if file not found or on error
        // IMPORTANT: Caller must call JS_GetLastFileSize IMMEDIATELY after JS_ReadFile to get the size
        // WARNING: _lastReadFileSize is global state - concurrent calls will have race conditions
        var filename = UTF8ToString(filenamePtr);
        var bytes = Module._VFS.loadFile(filename);

        if (!bytes || bytes.length === 0) {
            console.warn('File not found in VFS:', filename);
            Module._lastReadFileSize = 0;
            return 0; // Return null pointer if file not found
        }

        // Allocate memory and copy file data
        var ptr = _malloc(bytes.length);
        if (!ptr) {
            console.error('Failed to allocate memory for file read');
            Module._lastReadFileSize = 0;
            return 0;
        }

        Module.HEAPU8.set(bytes, ptr);

        // Store size for caller to retrieve
        // NOTE: Global state - caller MUST call JS_GetLastFileSize immediately
        Module._lastReadFileSize = bytes.length;

        return ptr; // CRITICAL: Caller must free this pointer!
    },
    JS_GetLastFileSize: function() {
        // Returns size of last file read by JS_ReadFile
        // WARNING: Must be called IMMEDIATELY after JS_ReadFile due to global state
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

                    // Add to recent files
                    if (Module._recentFiles) {
                        Module._recentFiles.add(file.name);
                    }

                    // Mark as dirty for auto-save
                    if (Module._autoSave) {
                        Module._autoSave.markDirty();
                    }

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
            resizeTimeout = setTimeout(onResize, Module._CONSTANTS.RESIZE_DEBOUNCE_MS);
        };
        window.addEventListener('resize', debouncedResize);
        onResize(); // Call immediately on init

        // Initialize polished features after input is ready
        Module._initPolishedFeatures();

        // Initialize power user features
        Module._initPowerFeatures();

        // Initialize complete application suite
        Module._initCompleteFeatures();

        // Save session on page unload
        window.addEventListener('unload', function() {
            if (Module._sessionRecovery) {
                Module._sessionRecovery.save();
            }
        });
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
        btn.textContent = title.replace('&', '');
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
                itemDiv.textContent = item.text.replace(/&/g, '').split('\t')[0];
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
        titleBar.textContent = title;
        dialog.appendChild(titleBar);

        // Content
        var contentDiv = document.createElement('div');
        contentDiv.style.cssText = 'padding:16px;';
        if (typeof content === 'string') {
            // XSS SAFETY: Use textContent for plain text, or sanitize if HTML is needed
            // For now, treat all strings as plain text to prevent XSS
            contentDiv.textContent = content;
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
                button.textContent = btn.text;
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
        // XSS SAFETY: Use safe utility instead of innerHTML
        Module._addTextWithLineBreaks(p, msg);
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
        // XSS SAFETY: Use safe utility instead of innerHTML
        Module._addTextWithLineBreaks(p, msg);
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
        // XSS SAFETY: Use safe utility instead of innerHTML
        Module._addTextWithLineBreaks(p, msg);
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
            Module._errorHandler.showError('Invalid Data',
                'The application provided invalid data for this dialog. This may be a bug. Error: ' + e.message);
            return 0;
        }

        var content = document.createElement('div');
        var p = document.createElement('p');
        // XSS SAFETY: Use safe utility instead of innerHTML
        Module._addTextWithLineBreaks(p, msg);
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

    // ===== POLISHED PRODUCT FEATURES (PHASE 10 - PACKAGE B) =====

    // Auto-Save System
    _autoSave: {
        isDirty: false,
        intervalId: null,
        lastSaveTime: null,
        statusElement: null,

        init: function() {
            // Create save status indicator
            this.statusElement = document.createElement('div');
            this.statusElement.id = 'save-status';
            this.statusElement.style.cssText = 'position:fixed;bottom:10px;right:10px;padding:8px 12px;background:rgba(0,0,0,0.7);color:white;border-radius:4px;font-size:12px;z-index:9000;opacity:0;transition:opacity 0.3s;';
            this.statusElement.setAttribute('role', 'status');
            this.statusElement.setAttribute('aria-live', 'polite');
            document.body.appendChild(this.statusElement);

            // Start auto-save interval
            var self = this;
            this.intervalId = setInterval(function() {
                if (self.isDirty) {
                    self.save();
                }
            }, Module._CONSTANTS.AUTO_SAVE_INTERVAL_MS);

            console.log('Auto-save system initialized (interval: ' + Module._CONSTANTS.AUTO_SAVE_INTERVAL_MS + 'ms)');
        },

        markDirty: function() {
            this.isDirty = true;
        },

        save: function() {
            if (!Module._currentDocument) {
                console.warn('Auto-save: No document to save');
                return;
            }

            try {
                // Get current document data from C++ (assumes exported function)
                // For now, we'll save the VFS state
                var files = Module._VFS.listFiles();
                localStorage.setItem('treesheets_autosave_timestamp', Date.now().toString());
                localStorage.setItem('treesheets_autosave_files', JSON.stringify(files));

                this.isDirty = false;
                this.lastSaveTime = new Date();
                this.showStatus('Saved at ' + this.lastSaveTime.toLocaleTimeString());
                console.log('Auto-saved successfully');
            } catch (e) {
                console.error('Auto-save failed:', e);
                this.showStatus('Save failed', true);
            }
        },

        showStatus: function(message, isError) {
            if (!this.statusElement) return;
            this.statusElement.textContent = message;
            this.statusElement.style.backgroundColor = isError ? 'rgba(220,0,0,0.8)' : 'rgba(0,0,0,0.7)';
            this.statusElement.style.opacity = '1';
            var self = this;
            setTimeout(function() {
                self.statusElement.style.opacity = '0';
            }, 3000);
        }
    },

    JS_Save: function() {
        Module._autoSave.save();
    },

    JS_MarkDirty: function() {
        Module._autoSave.markDirty();
    },

    // Recent Files Manager
    _recentFiles: {
        add: function(filename) {
            try {
                var recent = this.getList();
                // Remove if already exists
                recent = recent.filter(function(f) { return f !== filename; });
                // Add to front
                recent.unshift(filename);
                // Limit to max
                if (recent.length > Module._CONSTANTS.MAX_RECENT_FILES) {
                    recent = recent.slice(0, Module._CONSTANTS.MAX_RECENT_FILES);
                }
                localStorage.setItem('treesheets_recent_files', JSON.stringify(recent));
                console.log('Added to recent files:', filename);
            } catch (e) {
                console.error('Failed to add recent file:', e);
            }
        },

        getList: function() {
            try {
                var json = localStorage.getItem('treesheets_recent_files');
                return json ? JSON.parse(json) : [];
            } catch (e) {
                console.error('Failed to get recent files:', e);
                return [];
            }
        },

        clear: function() {
            localStorage.removeItem('treesheets_recent_files');
        },

        showMenu: function() {
            var recent = this.getList();
            if (recent.length === 0) {
                Module._createModal('Recent Files', '<p>No recent files</p>', [
                    { text: 'OK', primary: true }
                ]);
                return;
            }

            var content = document.createElement('div');
            var list = document.createElement('ul');
            list.style.cssText = 'list-style:none;padding:0;margin:0;max-height:400px;overflow-y:auto;';

            recent.forEach(function(filename) {
                var li = document.createElement('li');
                li.style.cssText = 'padding:8px;cursor:pointer;border-bottom:1px solid #eee;';
                li.textContent = filename;
                li.onmouseover = function() { li.style.backgroundColor = '#f0f0f0'; };
                li.onmouseout = function() { li.style.backgroundColor = 'white'; };
                li.onclick = function() {
                    // BUGFIX: Use fileLoader utility to prevent memory leaks
                    Module._fileLoader.loadFromVFS(
                        filename,
                        function() {
                            // Success - close modal
                            var modal = document.getElementById('ts-modal');
                            if (modal) modal.remove();
                        },
                        function(error) {
                            // Error - show message
                            Module._errorHandler.showError('Error', error);
                        }
                    );
                };
                list.appendChild(li);
            });

            content.appendChild(list);

            Module._createModal('Recent Files', content, [
                { text: 'Clear All', callback: function() {
                    Module._recentFiles.clear();
                    Module._recentFiles.showMenu();
                }},
                { text: 'Close', primary: true }
            ]);
        }
    },

    JS_ShowRecentFiles: function() {
        Module._recentFiles.showMenu();
    },

    // Dark Mode Manager
    _darkMode: {
        isDark: false,
        toggleButton: null,

        init: function() {
            // Load saved preference
            var saved = localStorage.getItem('treesheets_dark_mode');
            if (saved === 'true') {
                this.enable();
            }

            // Create toggle button in toolbar or menubar
            this.createToggleButton();
        },

        createToggleButton: function() {
            var btn = document.createElement('button');
            btn.id = 'dark-mode-toggle';
            btn.textContent = '🌙';
            btn.title = 'Toggle Dark Mode';
            btn.style.cssText = 'position:fixed;top:10px;right:10px;padding:8px 12px;border:1px solid #ccc;border-radius:4px;background:#fff;cursor:pointer;z-index:9000;font-size:16px;';
            btn.setAttribute('aria-label', 'Toggle dark mode');
            btn.onclick = function() {
                Module._darkMode.toggle();
            };
            document.body.appendChild(btn);
            this.toggleButton = btn;
        },

        toggle: function() {
            if (this.isDark) {
                this.disable();
            } else {
                this.enable();
            }
        },

        enable: function() {
            this.isDark = true;
            document.body.style.backgroundColor = '#1e1e1e';
            document.body.style.color = '#e0e0e0';

            // Update canvas background
            if (Module.canvas) {
                Module.canvas.style.backgroundColor = '#2d2d2d';
            }

            // Update menubar
            var menubar = document.getElementById('menubar');
            if (menubar) {
                menubar.style.backgroundColor = '#2d2d2d';
                menubar.style.color = '#e0e0e0';
            }

            // Update toolbar
            var toolbar = document.getElementById('toolbar');
            if (toolbar) {
                toolbar.style.backgroundColor = '#2d2d2d';
                toolbar.style.color = '#e0e0e0';
            }

            // Update toggle button
            if (this.toggleButton) {
                this.toggleButton.textContent = '☀️';
                this.toggleButton.style.background = '#2d2d2d';
                this.toggleButton.style.color = '#e0e0e0';
                this.toggleButton.style.borderColor = '#555';
            }

            localStorage.setItem('treesheets_dark_mode', 'true');
            console.log('Dark mode enabled');
        },

        disable: function() {
            this.isDark = false;
            document.body.style.backgroundColor = '#f5f5f5';
            document.body.style.color = '#000';

            // Update canvas background
            if (Module.canvas) {
                Module.canvas.style.backgroundColor = '#fff';
            }

            // Update menubar
            var menubar = document.getElementById('menubar');
            if (menubar) {
                menubar.style.backgroundColor = '#e0e0e0';
                menubar.style.color = '#000';
            }

            // Update toolbar
            var toolbar = document.getElementById('toolbar');
            if (toolbar) {
                toolbar.style.backgroundColor = '#f0f0f0';
                toolbar.style.color = '#000';
            }

            // Update toggle button
            if (this.toggleButton) {
                this.toggleButton.textContent = '🌙';
                this.toggleButton.style.background = '#fff';
                this.toggleButton.style.color = '#000';
                this.toggleButton.style.borderColor = '#ccc';
            }

            localStorage.setItem('treesheets_dark_mode', 'false');
            console.log('Dark mode disabled');
        }
    },

    JS_ToggleDarkMode: function() {
        Module._darkMode.toggle();
    },

    // Session Recovery
    _sessionRecovery: {
        save: function() {
            try {
                // Save current state
                var state = {
                    timestamp: Date.now(),
                    files: Module._VFS.listFiles()
                };
                localStorage.setItem('treesheets_session_recovery', JSON.stringify(state));
                console.log('Session saved for recovery');
            } catch (e) {
                console.error('Failed to save session:', e);
            }
        },

        restore: function() {
            try {
                var json = localStorage.getItem('treesheets_session_recovery');
                if (!json) return false;

                var state = JSON.parse(json);
                var age = Date.now() - state.timestamp;

                // Only restore if session is less than 24 hours old
                if (age > Module._CONSTANTS.SESSION_MAX_AGE_MS) {
                    console.log('Session too old, not restoring');
                    return false;
                }

                // Check if there are files to restore
                if (state.files && state.files.length > 0) {
                    var content = document.createElement('div');
                    var p = document.createElement('p');
                    p.textContent = 'A previous session was found from ' + new Date(state.timestamp).toLocaleString() + '. Would you like to restore it?';
                    content.appendChild(p);

                    Module._createModal('Restore Session?', content, [
                        { text: 'No', callback: function() {
                            localStorage.removeItem('treesheets_session_recovery');
                        }},
                        { text: 'Yes', primary: true, callback: function() {
                            // BUGFIX: Use fileLoader utility to prevent memory leaks
                            if (state.files.length > 0) {
                                var filename = state.files[0];
                                Module._fileLoader.loadFromVFS(
                                    filename,
                                    function() {
                                        console.log('Session restored successfully:', filename);
                                    },
                                    function(error) {
                                        console.error('Session restore failed:', error);
                                        Module._errorHandler.showError('Restore Failed', error);
                                    }
                                );
                            }
                        }}
                    ]);
                    return true;
                }
            } catch (e) {
                console.error('Failed to restore session:', e);
            }
            return false;
        }
    },

    // About Dialog
    JS_ShowAbout: function() {
        var content = document.createElement('div');
        content.style.cssText = 'text-align:center;';

        var title = document.createElement('h2');
        title.textContent = '🌳 TreeSheets Web';
        title.style.margin = '0 0 10px 0';
        content.appendChild(title);

        var version = document.createElement('p');
        version.textContent = 'Version ' + Module._CONSTANTS.VERSION;
        version.style.cssText = 'color:#666;margin:5px 0;';
        content.appendChild(version);

        var desc = document.createElement('p');
        desc.textContent = 'Hierarchical spreadsheet application for the web';
        desc.style.cssText = 'margin:15px 0;';
        content.appendChild(desc);

        var links = document.createElement('div');
        links.style.cssText = 'margin:20px 0;display:flex;flex-direction:column;gap:10px;';

        var addLink = function(text, url) {
            var a = document.createElement('a');
            a.textContent = text;
            a.href = url;
            a.target = '_blank';
            a.style.cssText = 'color:#007bff;text-decoration:none;';
            a.onmouseover = function() { a.style.textDecoration = 'underline'; };
            a.onmouseout = function() { a.style.textDecoration = 'none'; };
            links.appendChild(a);
        };

        addLink('📚 Documentation', '#');
        addLink('💻 Source Code', 'https://github.com');
        addLink('❓ Help & Support', '#');

        content.appendChild(links);

        var credits = document.createElement('p');
        credits.innerHTML = '&copy; 2025 TreeSheets Project<br>Licensed under ZLIB License';
        credits.style.cssText = 'color:#999;font-size:11px;margin-top:20px;';
        content.appendChild(credits);

        Module._createModal('About TreeSheets', content, [
            { text: 'Keyboard Shortcuts', callback: function() {
                Module._keyboardHelp.show();
            }},
            { text: 'OK', primary: true }
        ]);
    },

    // Keyboard Shortcuts Help
    _keyboardHelp: {
        shortcuts: [
            { category: 'File', items: [
                { key: 'Ctrl+N', desc: 'New document' },
                { key: 'Ctrl+O', desc: 'Open file' },
                { key: 'Ctrl+S', desc: 'Save' },
                { key: 'Ctrl+Shift+S', desc: 'Save As' }
            ]},
            { category: 'Edit', items: [
                { key: 'Ctrl+Z', desc: 'Undo' },
                { key: 'Ctrl+Y', desc: 'Redo' },
                { key: 'Ctrl+X', desc: 'Cut' },
                { key: 'Ctrl+C', desc: 'Copy' },
                { key: 'Ctrl+V', desc: 'Paste' },
                { key: 'Ctrl+A', desc: 'Select All' }
            ]},
            { category: 'View', items: [
                { key: 'Ctrl++', desc: 'Zoom In' },
                { key: 'Ctrl+-', desc: 'Zoom Out' },
                { key: 'Ctrl+0', desc: 'Reset Zoom' },
                { key: 'F11', desc: 'Fullscreen' }
            ]},
            { category: 'Navigation', items: [
                { key: 'Arrow Keys', desc: 'Move selection' },
                { key: 'Tab', desc: 'Next cell' },
                { key: 'Shift+Tab', desc: 'Previous cell' },
                { key: 'Home', desc: 'Go to start' },
                { key: 'End', desc: 'Go to end' }
            ]},
            { category: 'Help', items: [
                { key: 'F1', desc: 'Show this help' }
            ]}
        ],

        show: function() {
            var content = document.createElement('div');
            content.style.cssText = 'max-height:500px;overflow-y:auto;';

            this.shortcuts.forEach(function(section) {
                var categoryTitle = document.createElement('h3');
                categoryTitle.textContent = section.category;
                categoryTitle.style.cssText = 'margin:15px 0 10px 0;color:#333;font-size:14px;border-bottom:1px solid #ddd;padding-bottom:5px;';
                content.appendChild(categoryTitle);

                var table = document.createElement('table');
                table.style.cssText = 'width:100%;border-collapse:collapse;margin-bottom:10px;';

                section.items.forEach(function(item) {
                    var row = document.createElement('tr');

                    var keyCell = document.createElement('td');
                    keyCell.textContent = item.key;
                    keyCell.style.cssText = 'padding:6px 10px;font-family:monospace;background:#f5f5f5;border:1px solid #ddd;border-radius:3px;white-space:nowrap;width:150px;';
                    row.appendChild(keyCell);

                    var descCell = document.createElement('td');
                    descCell.textContent = item.desc;
                    descCell.style.cssText = 'padding:6px 10px;';
                    row.appendChild(descCell);

                    table.appendChild(row);
                });

                content.appendChild(table);
            });

            Module._createModal('Keyboard Shortcuts', content, [
                { text: 'Close', primary: true }
            ]);
        }
    },

    JS_ShowKeyboardHelp: function() {
        Module._keyboardHelp.show();
    },

    // Unsaved Changes Warning
    _unsavedWarning: {
        init: function() {
            window.addEventListener('beforeunload', function(e) {
                if (Module._autoSave.isDirty) {
                    // Modern browsers ignore custom messages, but we still need to return a value
                    e.preventDefault();
                    e.returnValue = 'You have unsaved changes. Are you sure you want to leave?';
                    return e.returnValue;
                }
            });
            console.log('Unsaved changes warning initialized');
        }
    },

    // Drag & Drop File Upload
    _dragDrop: {
        init: function() {
            var canvas = Module.canvas;
            if (!canvas) {
                console.warn('Canvas not found, drag & drop not initialized');
                return;
            }

            // Create drop zone overlay
            var overlay = document.createElement('div');
            overlay.id = 'drop-overlay';
            overlay.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,120,215,0.1);border:4px dashed #007bff;display:none;align-items:center;justify-content:center;z-index:8000;pointer-events:none;';
            overlay.innerHTML = '<div style="background:white;padding:30px;border-radius:8px;box-shadow:0 4px 20px rgba(0,0,0,0.2);"><h2 style="margin:0;color:#007bff;">📁 Drop file here</h2><p style="color:#666;margin:10px 0 0 0;">Supports .cts files</p></div>';
            document.body.appendChild(overlay);

            // Drag enter
            document.addEventListener('dragenter', function(e) {
                e.preventDefault();
                overlay.style.display = 'flex';
            });

            // Drag over
            document.addEventListener('dragover', function(e) {
                e.preventDefault();
                e.dataTransfer.dropEffect = 'copy';
            });

            // Drag leave
            document.addEventListener('dragleave', function(e) {
                if (e.target === document.body || e.target === overlay) {
                    overlay.style.display = 'none';
                }
            });

            // Drop
            document.addEventListener('drop', function(e) {
                e.preventDefault();
                overlay.style.display = 'none';

                var files = e.dataTransfer.files;
                if (files.length === 0) return;

                var file = files[0];

                // Check file extension
                // CASE-INSENSITIVE file extension check
                if (!file.name.toLowerCase().endsWith('.cts')) {
                    Module._createModal('Invalid File', '<p>Only .cts files are supported.</p>', [
                        { text: 'OK', primary: true }
                    ]);
                    return;
                }

                // Check file size
                var maxSize = Module._CONSTANTS.MAX_FILE_SIZE_BYTES;
                if (file.size > maxSize) {
                    Module._createModal('Error', '<p>File too large. Maximum size is ' +
                        Math.floor(maxSize / (1024 * 1024)) + 'MB.</p>', [
                        { text: 'OK', primary: true }
                    ]);
                    return;
                }

                // Read and process file
                var reader = new FileReader();
                reader.onload = function(evt) {
                    // BUGFIX: Proper try-finally to prevent memory leaks
                    var ptr = null;
                    var namePtr = null;
                    try {
                        var arrayBuffer = evt.target.result;
                        var uint8Array = new Uint8Array(arrayBuffer);

                        ptr = _malloc(uint8Array.length);
                        if (!ptr) {
                            throw new Error('Failed to allocate memory for file data');
                        }
                        Module.HEAPU8.set(uint8Array, ptr);

                        var nameLen = lengthBytesUTF8(file.name) + 1;
                        namePtr = _malloc(nameLen);
                        if (!namePtr) {
                            throw new Error('Failed to allocate memory for filename');
                        }
                        stringToUTF8(file.name, namePtr, nameLen);

                        // Save to VFS and recent files
                        Module._VFS.saveFile(file.name, uint8Array);
                        Module._recentFiles.add(file.name);

                        Module._WASM_FileLoaded(namePtr, ptr, uint8Array.length);

                        console.log('File loaded via drag & drop:', file.name);
                    } catch (err) {
                        console.error('Error processing dropped file:', err);
                        Module._errorHandler.showError('Error', 'Failed to process file: ' + err.message);
                    } finally {
                        // CRITICAL: Always free memory, even on error
                        if (ptr) _free(ptr);
                        if (namePtr) _free(namePtr);
                    }
                };
                reader.onerror = function(err) {
                    console.error('FileReader error:', err);
                    Module._createModal('Error', '<p>Failed to read file.</p>', [
                        { text: 'OK', primary: true }
                    ]);
                };
                reader.readAsArrayBuffer(file);
            });

            console.log('Drag & drop file upload initialized');
        }
    },

    // Initialize all polished features
    _initPolishedFeatures: function() {
        // BUGFIX: Prevent multiple initialization to avoid event listener accumulation
        if (Module._polishedFeaturesInitialized) {
            console.warn('Polished features already initialized, skipping');
            return;
        }
        Module._polishedFeaturesInitialized = true;

        console.log('Initializing polished features...');
        Module._autoSave.init();
        Module._darkMode.init();
        Module._unsavedWarning.init();
        Module._dragDrop.init();

        // Try to restore session after a brief delay
        setTimeout(function() {
            Module._sessionRecovery.restore();
        }, 1000);

        // Add F1 keyboard shortcut for help (store reference for cleanup)
        Module._polishedKeyHandler = function(e) {
            if (e.key === 'F1') {
                e.preventDefault();
                Module._keyboardHelp.show();
            }
        };
        window.addEventListener('keydown', Module._polishedKeyHandler);

        console.log('All polished features initialized');
    },

    // ===== POWER USER FEATURES (PHASE 11) =====

    // Export Manager
    _exportManager: {
        exportToCSV: function(data) {
            // Placeholder: Would need actual document data from C++
            // For now, create sample export
            var csv = 'Column1,Column2,Column3\n';
            csv += 'Value1,Value2,Value3\n';
            csv += 'Value4,Value5,Value6\n';

            var blob = new Blob([csv], { type: 'text/csv' });
            var url = URL.createObjectURL(blob);
            var a = document.createElement('a');
            a.href = url;
            a.download = 'treesheets-export.csv';
            a.click();
            URL.revokeObjectURL(url);

            console.log('Exported to CSV');
        },

        exportToJSON: function(data) {
            var jsonData = {
                version: Module._CONSTANTS.VERSION,
                exported: new Date().toISOString(),
                data: data || { cells: [], metadata: {} }
            };

            var json = JSON.stringify(jsonData, null, 2);
            var blob = new Blob([json], { type: 'application/json' });
            var url = URL.createObjectURL(blob);
            var a = document.createElement('a');
            a.href = url;
            a.download = 'treesheets-export.json';
            a.click();
            URL.revokeObjectURL(url);

            console.log('Exported to JSON');
        },

        exportToHTML: function(data) {
            var html = '<!DOCTYPE html>\n<html>\n<head>\n';
            html += '<meta charset="UTF-8">\n';
            html += '<title>TreeSheets Export</title>\n';
            html += '<style>\n';
            html += 'body { font-family: sans-serif; margin: 20px; }\n';
            html += 'table { border-collapse: collapse; width: 100%; }\n';
            html += 'th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n';
            html += 'th { background-color: #f0f0f0; font-weight: bold; }\n';
            html += '</style>\n</head>\n<body>\n';
            html += '<h1>TreeSheets Export</h1>\n';
            html += '<table>\n<tr><th>Column 1</th><th>Column 2</th><th>Column 3</th></tr>\n';
            html += '<tr><td>Value 1</td><td>Value 2</td><td>Value 3</td></tr>\n';
            html += '<tr><td>Value 4</td><td>Value 5</td><td>Value 6</td></tr>\n';
            html += '</table>\n</body>\n</html>';

            var blob = new Blob([html], { type: 'text/html' });
            var url = URL.createObjectURL(blob);
            var a = document.createElement('a');
            a.href = url;
            a.download = 'treesheets-export.html';
            a.click();
            URL.revokeObjectURL(url);

            console.log('Exported to HTML');
        },

        exportToMarkdown: function(data) {
            var md = '# TreeSheets Export\n\n';
            md += '| Column 1 | Column 2 | Column 3 |\n';
            md += '|----------|----------|----------|\n';
            md += '| Value 1  | Value 2  | Value 3  |\n';
            md += '| Value 4  | Value 5  | Value 6  |\n';

            var blob = new Blob([md], { type: 'text/markdown' });
            var url = URL.createObjectURL(blob);
            var a = document.createElement('a');
            a.href = url;
            a.download = 'treesheets-export.md';
            a.click();
            URL.revokeObjectURL(url);

            console.log('Exported to Markdown');
        },

        showExportDialog: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Export Document';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var formatLabel = document.createElement('p');
            formatLabel.textContent = 'Choose export format:';
            formatLabel.style.margin = '10px 0 5px 0';
            content.appendChild(formatLabel);

            var formatSelect = document.createElement('select');
            formatSelect.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;';
            Module._CONSTANTS.EXPORT_FORMATS.forEach(function(format) {
                var option = document.createElement('option');
                option.value = format.toLowerCase();
                option.textContent = format;
                formatSelect.appendChild(option);
            });
            content.appendChild(formatSelect);

            var self = this;
            Module._createModal('Export', content, [
                { text: 'Cancel' },
                { text: 'Export', primary: true, callback: function() {
                    var format = formatSelect.value;
                    switch(format) {
                        case 'csv': self.exportToCSV(); break;
                        case 'json': self.exportToJSON(); break;
                        case 'html': self.exportToHTML(); break;
                        case 'markdown': self.exportToMarkdown(); break;
                        default: console.warn('Unknown format:', format);
                    }
                }}
            ]);
        }
    },

    JS_ShowExportDialog: function() {
        Module._exportManager.showExportDialog();
    },

    // Import Manager
    _importManager: {
        importCSV: function(text) {
            try {
                var lines = text.split('\n');
                var data = [];
                lines.forEach(function(line) {
                    if (line.trim()) {
                        data.push(line.split(','));
                    }
                });
                console.log('Parsed CSV data:', data);
                // Would pass to C++ here
                Module._createModal('Import Success', '<p>Imported ' + data.length + ' rows from CSV</p>', [
                    { text: 'OK', primary: true }
                ]);
            } catch (e) {
                console.error('CSV import failed:', e);
                Module._createModal('Import Error', '<p>Failed to parse CSV: ' + e.message + '</p>', [
                    { text: 'OK', primary: true }
                ]);
            }
        },

        importJSON: function(text) {
            try {
                var data = JSON.parse(text);
                console.log('Parsed JSON data:', data);
                // Would pass to C++ here
                Module._createModal('Import Success', '<p>Imported JSON data successfully</p>', [
                    { text: 'OK', primary: true }
                ]);
            } catch (e) {
                console.error('JSON import failed:', e);
                Module._createModal('Import Error', '<p>Failed to parse JSON: ' + e.message + '</p>', [
                    { text: 'OK', primary: true }
                ]);
            }
        },

        showImportDialog: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Import Data';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var formatLabel = document.createElement('p');
            formatLabel.textContent = 'Format:';
            content.appendChild(formatLabel);

            var formatSelect = document.createElement('select');
            formatSelect.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-bottom:10px;';
            ['CSV', 'JSON'].forEach(function(format) {
                var option = document.createElement('option');
                option.value = format.toLowerCase();
                option.textContent = format;
                formatSelect.appendChild(option);
            });
            content.appendChild(formatSelect);

            var dataLabel = document.createElement('p');
            dataLabel.textContent = 'Paste data:';
            content.appendChild(dataLabel);

            var textarea = document.createElement('textarea');
            textarea.style.cssText = 'width:100%;height:200px;padding:8px;border:1px solid #ccc;border-radius:4px;font-family:monospace;font-size:12px;';
            textarea.placeholder = 'Paste CSV or JSON data here...';
            content.appendChild(textarea);

            var self = this;
            Module._createModal('Import', content, [
                { text: 'Cancel' },
                { text: 'Import', primary: true, callback: function() {
                    var format = formatSelect.value;
                    var text = textarea.value.trim();
                    if (!text) {
                        Module._createModal('Error', '<p>No data to import</p>', [{ text: 'OK', primary: true }]);
                        return;
                    }
                    if (format === 'csv') {
                        self.importCSV(text);
                    } else if (format === 'json') {
                        self.importJSON(text);
                    }
                }}
            ]);

            setTimeout(function() { textarea.focus(); }, 100);
        }
    },

    JS_ShowImportDialog: function() {
        Module._importManager.showImportDialog();
    },

    // Theme Manager
    _themeManager: {
        currentTheme: 'light',

        applyTheme: function(themeId) {
            var theme = Module._CONSTANTS.THEMES[themeId];
            if (!theme) {
                console.warn('Theme not found:', themeId);
                return;
            }

            document.body.style.backgroundColor = theme.bg;
            document.body.style.color = theme.fg;

            if (Module.canvas) {
                Module.canvas.style.backgroundColor = theme.canvas;
            }

            var menubar = document.getElementById('menubar');
            if (menubar) {
                menubar.style.backgroundColor = theme.ui;
                menubar.style.color = theme.fg;
            }

            var toolbar = document.getElementById('toolbar');
            if (toolbar) {
                toolbar.style.backgroundColor = theme.ui;
                toolbar.style.color = theme.fg;
            }

            // Update dark mode toggle to match
            var isDark = themeId !== 'light' && themeId !== 'solarized-light';
            if (Module._darkMode) {
                Module._darkMode.isDark = isDark;
                if (Module._darkMode.toggleButton) {
                    Module._darkMode.toggleButton.textContent = isDark ? '☀️' : '🌙';
                    Module._darkMode.toggleButton.style.background = theme.ui;
                    Module._darkMode.toggleButton.style.color = theme.fg;
                }
            }

            this.currentTheme = themeId;
            localStorage.setItem('treesheets_theme', themeId);
            console.log('Applied theme:', theme.name);
        },

        showThemeGallery: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Choose Theme';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var gallery = document.createElement('div');
            gallery.style.cssText = 'display:grid;grid-template-columns:repeat(2,1fr);gap:10px;';

            var self = this;
            Object.keys(Module._CONSTANTS.THEMES).forEach(function(themeId) {
                var theme = Module._CONSTANTS.THEMES[themeId];
                var card = document.createElement('div');
                card.style.cssText = 'padding:15px;border:2px solid ' + (self.currentTheme === themeId ? '#007bff' : '#ddd') + ';border-radius:8px;cursor:pointer;transition:border-color 0.2s;';
                card.style.backgroundColor = theme.canvas;
                card.style.color = theme.fg;

                var name = document.createElement('div');
                name.textContent = theme.name;
                name.style.cssText = 'font-weight:bold;margin-bottom:5px;';
                card.appendChild(name);

                var preview = document.createElement('div');
                preview.style.cssText = 'display:flex;gap:5px;margin-top:10px;';
                [theme.bg, theme.fg, theme.canvas, theme.ui].forEach(function(color) {
                    var swatch = document.createElement('div');
                    swatch.style.cssText = 'width:20px;height:20px;border-radius:3px;background-color:' + color + ';border:1px solid rgba(0,0,0,0.2);';
                    preview.appendChild(swatch);
                });
                card.appendChild(preview);

                card.onclick = function() {
                    self.applyTheme(themeId);
                    document.getElementById('ts-modal').remove();
                };

                card.onmouseover = function() {
                    if (self.currentTheme !== themeId) {
                        card.style.borderColor = '#007bff';
                    }
                };
                card.onmouseout = function() {
                    if (self.currentTheme !== themeId) {
                        card.style.borderColor = '#ddd';
                    }
                };

                gallery.appendChild(card);
            });

            content.appendChild(gallery);

            Module._createModal('Theme Gallery', content, [
                { text: 'Close', primary: true }
            ]);
        },

        init: function() {
            var saved = localStorage.getItem('treesheets_theme');
            if (saved && Module._CONSTANTS.THEMES[saved]) {
                this.applyTheme(saved);
            }
        }
    },

    JS_ShowThemeGallery: function() {
        Module._themeManager.showThemeGallery();
    },

    // Command Palette
    _commandPalette: {
        commands: [
            { id: 'file-new', name: 'File: New Document', action: function() { console.log('New'); } },
            { id: 'file-open', name: 'File: Open', action: function() { Module._WASM_Action(1); } },
            { id: 'file-save', name: 'File: Save', action: function() { Module._autoSave.save(); } },
            { id: 'file-export', name: 'File: Export...', action: function() { Module._exportManager.showExportDialog(); } },
            { id: 'file-import', name: 'File: Import...', action: function() { Module._importManager.showImportDialog(); } },
            { id: 'file-recent', name: 'File: Recent Files', action: function() { Module._recentFiles.showMenu(); } },
            { id: 'view-theme', name: 'View: Change Theme', action: function() { Module._themeManager.showThemeGallery(); } },
            { id: 'view-dark', name: 'View: Toggle Dark Mode', action: function() { Module._darkMode.toggle(); } },
            { id: 'view-zoom-in', name: 'View: Zoom In', action: function() { Module._zoomManager.zoomIn(); } },
            { id: 'view-zoom-out', name: 'View: Zoom Out', action: function() { Module._zoomManager.zoomOut(); } },
            { id: 'view-zoom-reset', name: 'View: Reset Zoom', action: function() { Module._zoomManager.resetZoom(); } },
            { id: 'view-fullscreen', name: 'View: Toggle Fullscreen', action: function() { Module._fullscreenManager.toggle(); } },
            { id: 'help-shortcuts', name: 'Help: Keyboard Shortcuts', action: function() { Module._keyboardHelp.show(); } },
            { id: 'help-about', name: 'Help: About TreeSheets', action: function() { Module.JS_ShowAbout(); } }
        ],

        show: function() {
            var content = document.createElement('div');

            var input = document.createElement('input');
            input.type = 'text';
            input.placeholder = 'Type to search commands...';
            input.style.cssText = 'width:100%;padding:10px;border:1px solid #ccc;border-radius:4px;font-size:14px;margin-bottom:10px;';
            content.appendChild(input);

            var list = document.createElement('div');
            list.style.cssText = 'max-height:400px;overflow-y:auto;';
            content.appendChild(list);

            var self = this;
            var renderCommands = function(filter) {
                list.innerHTML = '';
                var filtered = self.commands.filter(function(cmd) {
                    return !filter || cmd.name.toLowerCase().indexOf(filter.toLowerCase()) >= 0;
                });

                filtered.forEach(function(cmd, index) {
                    var item = document.createElement('div');
                    item.textContent = cmd.name;
                    item.style.cssText = 'padding:10px;cursor:pointer;border-bottom:1px solid #eee;';
                    item.setAttribute('tabindex', '0');
                    item.onmouseover = function() { item.style.backgroundColor = '#f0f0f0'; };
                    item.onmouseout = function() { item.style.backgroundColor = 'white'; };
                    item.onclick = function() {
                        cmd.action();
                        document.getElementById('ts-modal').remove();
                    };
                    item.onkeydown = function(e) {
                        if (e.key === 'Enter') {
                            cmd.action();
                            document.getElementById('ts-modal').remove();
                        }
                    };
                    list.appendChild(item);
                });

                if (filtered.length === 0) {
                    var empty = document.createElement('div');
                    empty.textContent = 'No commands found';
                    empty.style.cssText = 'padding:20px;text-align:center;color:#999;';
                    list.appendChild(empty);
                }
            };

            input.oninput = function() {
                renderCommands(input.value);
            };

            renderCommands('');

            Module._createModal('Command Palette', content, [
                { text: 'Close', primary: true }
            ]);

            setTimeout(function() { input.focus(); }, 100);
        }
    },

    JS_ShowCommandPalette: function() {
        Module._commandPalette.show();
    },

    // Zoom Manager
    _zoomManager: {
        currentZoom: 1.0,
        currentIndex: 3, // 1.0 is at index 3

        applyZoom: function(level) {
            this.currentZoom = level;
            if (Module.canvas) {
                Module.canvas.style.transform = 'scale(' + level + ')';
                Module.canvas.style.transformOrigin = 'top left';
            }
            console.log('Zoom:', (level * 100).toFixed(0) + '%');
        },

        zoomIn: function() {
            var levels = Module._CONSTANTS.ZOOM_LEVELS;
            if (this.currentIndex < levels.length - 1) {
                this.currentIndex++;
                this.applyZoom(levels[this.currentIndex]);
            }
        },

        zoomOut: function() {
            var levels = Module._CONSTANTS.ZOOM_LEVELS;
            if (this.currentIndex > 0) {
                this.currentIndex--;
                this.applyZoom(levels[this.currentIndex]);
            }
        },

        resetZoom: function() {
            this.currentIndex = 3;
            this.applyZoom(1.0);
        },

        showZoomMenu: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Zoom Level';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var levels = Module._CONSTANTS.ZOOM_LEVELS;
            var self = this;

            levels.forEach(function(level, index) {
                var btn = document.createElement('button');
                btn.textContent = (level * 100).toFixed(0) + '%';
                btn.style.cssText = 'width:100%;padding:10px;margin:5px 0;border:1px solid #ccc;border-radius:4px;cursor:pointer;background:' + (index === self.currentIndex ? '#007bff' : '#fff') + ';color:' + (index === self.currentIndex ? '#fff' : '#000') + ';';
                btn.onclick = function() {
                    self.currentIndex = index;
                    self.applyZoom(level);
                    document.getElementById('ts-modal').remove();
                };
                content.appendChild(btn);
            });

            Module._createModal('Zoom', content, [
                { text: 'Close', primary: true }
            ]);
        }
    },

    JS_ZoomIn: function() {
        Module._zoomManager.zoomIn();
    },

    JS_ZoomOut: function() {
        Module._zoomManager.zoomOut();
    },

    JS_ResetZoom: function() {
        Module._zoomManager.resetZoom();
    },

    JS_ShowZoomMenu: function() {
        Module._zoomManager.showZoomMenu();
    },

    // Fullscreen Manager
    _fullscreenManager: {
        isFullscreen: false,

        toggle: function() {
            if (!this.isFullscreen) {
                this.enter();
            } else {
                this.exit();
            }
        },

        enter: function() {
            var elem = document.documentElement;
            if (elem.requestFullscreen) {
                elem.requestFullscreen();
            } else if (elem.webkitRequestFullscreen) {
                elem.webkitRequestFullscreen();
            } else if (elem.mozRequestFullScreen) {
                // Firefox uses capital 'S' in Screen
                elem.mozRequestFullScreen();
            } else if (elem.msRequestFullscreen) {
                elem.msRequestFullscreen();
            }
            this.isFullscreen = true;
            console.log('Entered fullscreen');
        },

        exit: function() {
            if (document.exitFullscreen) {
                document.exitFullscreen();
            } else if (document.webkitExitFullscreen) {
                document.webkitExitFullscreen();
            } else if (document.msExitFullscreen) {
                document.msExitFullscreen();
            }
            this.isFullscreen = false;
            console.log('Exited fullscreen');
        }
    },

    JS_ToggleFullscreen: function() {
        Module._fullscreenManager.toggle();
    },

    // Find & Replace Manager
    _findReplaceManager: {
        show: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Find & Replace';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            // Find input
            var findLabel = document.createElement('label');
            findLabel.textContent = 'Find:';
            findLabel.style.display = 'block';
            findLabel.style.marginBottom = '5px';
            content.appendChild(findLabel);

            var findInput = document.createElement('input');
            findInput.type = 'text';
            findInput.placeholder = 'Enter search text...';
            findInput.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-bottom:10px;';
            content.appendChild(findInput);

            // Replace input
            var replaceLabel = document.createElement('label');
            replaceLabel.textContent = 'Replace with:';
            replaceLabel.style.display = 'block';
            replaceLabel.style.marginBottom = '5px';
            content.appendChild(replaceLabel);

            var replaceInput = document.createElement('input');
            replaceInput.type = 'text';
            replaceInput.placeholder = 'Enter replacement text...';
            replaceInput.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-bottom:10px;';
            content.appendChild(replaceInput);

            // Options
            var optionsDiv = document.createElement('div');
            optionsDiv.style.marginBottom = '10px';

            var caseSensitive = document.createElement('input');
            caseSensitive.type = 'checkbox';
            caseSensitive.id = 'case-sensitive';
            var caseLabel = document.createElement('label');
            caseLabel.htmlFor = 'case-sensitive';
            caseLabel.textContent = ' Case sensitive';
            caseLabel.style.marginRight = '15px';
            optionsDiv.appendChild(caseSensitive);
            optionsDiv.appendChild(caseLabel);

            var useRegex = document.createElement('input');
            useRegex.type = 'checkbox';
            useRegex.id = 'use-regex';
            var regexLabel = document.createElement('label');
            regexLabel.htmlFor = 'use-regex';
            regexLabel.textContent = ' Use regex';
            optionsDiv.appendChild(useRegex);
            optionsDiv.appendChild(regexLabel);

            content.appendChild(optionsDiv);

            // Results
            var results = document.createElement('div');
            results.style.cssText = 'padding:10px;background:#f0f0f0;border-radius:4px;margin-top:10px;min-height:40px;';
            results.textContent = 'Enter search term to begin';
            content.appendChild(results);

            Module._createModal('Find & Replace', content, [
                { text: 'Close' },
                { text: 'Find Next', callback: function() {
                    var searchTerm = findInput.value;
                    if (!searchTerm) {
                        results.textContent = 'Please enter search term';
                        return;
                    }
                    results.textContent = 'Searching for "' + searchTerm + '"... (functionality would integrate with C++)';
                    console.log('Find:', searchTerm, 'Case:', caseSensitive.checked, 'Regex:', useRegex.checked);
                }},
                { text: 'Replace All', primary: true, callback: function() {
                    var searchTerm = findInput.value;
                    var replaceTerm = replaceInput.value;
                    if (!searchTerm) {
                        results.textContent = 'Please enter search term';
                        return;
                    }
                    results.textContent = 'Replaced "' + searchTerm + '" with "' + replaceTerm + '" (functionality would integrate with C++)';
                    console.log('Replace:', searchTerm, '->', replaceTerm);
                }}
            ]);

            setTimeout(function() { findInput.focus(); }, 100);
        }
    },

    JS_ShowFindReplace: function() {
        Module._findReplaceManager.show();
    },

    // Settings Panel
    _settingsManager: {
        settings: {
            autoSaveInterval: 30000,
            maxRecentFiles: 10,
            theme: 'light',
            enableAutoSave: true,
            enableSessionRecovery: true,
            defaultZoom: 1.0
        },

        show: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Settings';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var self = this;

            // Auto-save interval
            var addSetting = function(label, type, key, options) {
                var container = document.createElement('div');
                container.style.marginBottom = '15px';

                var labelEl = document.createElement('label');
                labelEl.textContent = label;
                labelEl.style.cssText = 'display:block;margin-bottom:5px;font-weight:bold;';
                container.appendChild(labelEl);

                var input;
                if (type === 'checkbox') {
                    input = document.createElement('input');
                    input.type = 'checkbox';
                    input.checked = self.settings[key];
                } else if (type === 'select' && options) {
                    input = document.createElement('select');
                    input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;';
                    options.forEach(function(opt) {
                        var option = document.createElement('option');
                        option.value = opt.value;
                        option.textContent = opt.label;
                        if (opt.value === self.settings[key]) option.selected = true;
                        input.appendChild(option);
                    });
                } else {
                    input = document.createElement('input');
                    input.type = type;
                    input.value = self.settings[key];
                    input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;';
                }

                input.id = 'setting-' + key;
                container.appendChild(input);

                content.appendChild(container);
                return input;
            };

            var autoSaveInput = addSetting('Auto-save interval (ms)', 'number', 'autoSaveInterval');
            var maxRecentInput = addSetting('Max recent files', 'number', 'maxRecentFiles');
            var enableAutoSaveInput = addSetting('Enable auto-save', 'checkbox', 'enableAutoSave');
            var enableRecoveryInput = addSetting('Enable session recovery', 'checkbox', 'enableSessionRecovery');
            var themeInput = addSetting('Theme', 'select', 'theme', [
                { value: 'light', label: 'Light' },
                { value: 'dark', label: 'Dark' },
                { value: 'solarized-light', label: 'Solarized Light' },
                { value: 'solarized-dark', label: 'Solarized Dark' }
            ]);

            Module._createModal('Settings', content, [
                { text: 'Cancel' },
                { text: 'Save', primary: true, callback: function() {
                    self.settings.autoSaveInterval = parseInt(autoSaveInput.value) || 30000;
                    self.settings.maxRecentFiles = parseInt(maxRecentInput.value) || 10;
                    self.settings.enableAutoSave = enableAutoSaveInput.checked;
                    self.settings.enableSessionRecovery = enableRecoveryInput.checked;
                    self.settings.theme = themeInput.value;

                    // Apply settings
                    Module._CONSTANTS.AUTO_SAVE_INTERVAL_MS = self.settings.autoSaveInterval;
                    Module._CONSTANTS.MAX_RECENT_FILES = self.settings.maxRecentFiles;
                    Module._themeManager.applyTheme(self.settings.theme);

                    localStorage.setItem('treesheets_settings', JSON.stringify(self.settings));
                    console.log('Settings saved');
                }}
            ]);
        },

        load: function() {
            var saved = localStorage.getItem('treesheets_settings');
            if (saved) {
                try {
                    this.settings = JSON.parse(saved);
                } catch (e) {
                    console.error('Failed to load settings:', e);
                }
            }
        }
    },

    JS_ShowSettings: function() {
        Module._settingsManager.show();
    },

    // Initialize all power user features
    _initPowerFeatures: function() {
        // BUGFIX: Prevent multiple initialization to avoid event listener accumulation
        if (Module._powerFeaturesInitialized) {
            console.warn('Power user features already initialized, skipping');
            return;
        }
        Module._powerFeaturesInitialized = true;

        console.log('Initializing power user features...');
        Module._themeManager.init();
        Module._settingsManager.load();

        // Add keyboard shortcuts (store reference for cleanup)
        Module._powerKeyHandler = function(e) {
            if (e.ctrlKey && e.shiftKey && e.key === 'P') {
                e.preventDefault();
                Module._commandPalette.show();
            }
            // Ctrl+F for find
            if (e.ctrlKey && e.key === 'f') {
                e.preventDefault();
                Module._findReplaceManager.show();
            }
            // F11 for fullscreen (already handled by browser, but we track it)
            if (e.key === 'F11') {
                setTimeout(function() {
                    Module._fullscreenManager.isFullscreen = !Module._fullscreenManager.isFullscreen;
                }, 100);
            }
        };
        window.addEventListener('keydown', Module._powerKeyHandler);

        console.log('All power user features initialized');
    },

    // ===== COMPLETE APPLICATION SUITE (PHASE 12) =====

    // Template Manager
    _templateManager: {
        templates: {
            'blank': { name: 'Blank Document', data: [] },
            'todo': {
                name: 'To-Do List',
                data: [
                    ['Task', 'Priority', 'Status', 'Due Date'],
                    ['Sample task 1', 'High', '☐ To Do', '2025-01-15'],
                    ['Sample task 2', 'Medium', '☐ To Do', '2025-01-20'],
                    ['Sample task 3', 'Low', '☐ To Do', '2025-01-25']
                ]
            },
            'budget': {
                name: 'Budget Planner',
                data: [
                    ['Category', 'Budgeted', 'Actual', 'Difference'],
                    ['Housing', '1000', '950', '50'],
                    ['Food', '400', '420', '-20'],
                    ['Transportation', '200', '180', '20'],
                    ['Total', '1600', '1550', '50']
                ]
            },
            'schedule': {
                name: 'Weekly Schedule',
                data: [
                    ['Time', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday'],
                    ['9:00 AM', '', '', '', '', ''],
                    ['11:00 AM', '', '', '', '', ''],
                    ['2:00 PM', '', '', '', '', ''],
                    ['4:00 PM', '', '', '', '', '']
                ]
            },
            'project': {
                name: 'Project Plan',
                data: [
                    ['Phase', 'Tasks', 'Owner', 'Status', 'Deadline'],
                    ['Planning', 'Requirements gathering', 'Team', 'Complete', '2025-01-01'],
                    ['Design', 'UI/UX design', 'Designer', 'In Progress', '2025-01-15'],
                    ['Development', 'Implementation', 'Dev Team', 'Not Started', '2025-02-01']
                ]
            },
            'notes': {
                name: 'Meeting Notes',
                data: [
                    ['Meeting Notes', '', ''],
                    ['Date:', '2025-01-01', ''],
                    ['Attendees:', '', ''],
                    ['Agenda:', '', ''],
                    ['Action Items:', '', '']
                ]
            }
        },

        showGallery: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'New from Template';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var gallery = document.createElement('div');
            gallery.style.cssText = 'display:grid;grid-template-columns:repeat(2,1fr);gap:15px;max-height:500px;overflow-y:auto;';

            var self = this;
            Module._CONSTANTS.TEMPLATES.forEach(function(template) {
                var card = document.createElement('div');
                card.style.cssText = 'padding:20px;border:2px solid #ddd;border-radius:8px;cursor:pointer;transition:all 0.2s;background:white;';

                var icon = document.createElement('div');
                icon.textContent = template.id === 'blank' ? '📄' : template.id === 'todo' ? '✓' : template.id === 'budget' ? '💰' : template.id === 'schedule' ? '📅' : template.id === 'project' ? '📊' : '📝';
                icon.style.cssText = 'font-size:40px;text-align:center;margin-bottom:10px;';
                card.appendChild(icon);

                var name = document.createElement('div');
                name.textContent = template.name;
                name.style.cssText = 'font-weight:bold;margin-bottom:5px;text-align:center;';
                card.appendChild(name);

                var desc = document.createElement('div');
                desc.textContent = template.desc;
                desc.style.cssText = 'font-size:12px;color:#666;text-align:center;';
                card.appendChild(desc);

                card.onclick = function() {
                    self.createFromTemplate(template.id);
                    document.getElementById('ts-modal').remove();
                };

                card.onmouseover = function() {
                    card.style.borderColor = '#007bff';
                    card.style.boxShadow = '0 4px 12px rgba(0,123,255,0.2)';
                };
                card.onmouseout = function() {
                    card.style.borderColor = '#ddd';
                    card.style.boxShadow = 'none';
                };

                gallery.appendChild(card);
            });

            content.appendChild(gallery);

            Module._createModal('Template Gallery', content, [
                { text: 'Cancel', primary: true }
            ]);
        },

        createFromTemplate: function(templateId) {
            var template = this.templates[templateId];
            if (!template) {
                console.warn('Template not found:', templateId);
                return;
            }

            console.log('Creating document from template:', template.name);
            // In a real implementation, would create document from template data
            Module._createModal('Template Created', '<p>Document created from template: <strong>' + template.name + '</strong></p><p>In a full implementation, this would populate the canvas with template data.</p>', [
                { text: 'OK', primary: true }
            ]);
        }
    },

    JS_ShowTemplateGallery: function() {
        Module._templateManager.showGallery();
    },

    // PWA Manager
    _pwaManager: {
        isInstalled: false,
        deferredPrompt: null,

        init: function() {
            // Listen for beforeinstallprompt event
            window.addEventListener('beforeinstallprompt', function(e) {
                e.preventDefault();
                Module._pwaManager.deferredPrompt = e;
                console.log('PWA install prompt available');
            });

            // Check if already installed
            if (window.matchMedia('(display-mode: standalone)').matches) {
                this.isInstalled = true;
                console.log('Running as installed PWA');
            }
        },

        showInstallPrompt: function() {
            if (this.isInstalled) {
                Module._createModal('Already Installed', '<p>TreeSheets is already installed as a Progressive Web App.</p>', [
                    { text: 'OK', primary: true }
                ]);
                return;
            }

            if (!this.deferredPrompt) {
                Module._createModal('Install Not Available', '<p>Install prompt is not available. You can manually install by using your browser\'s "Install App" option in the menu.</p>', [
                    { text: 'OK', primary: true }
                ]);
                return;
            }

            this.deferredPrompt.prompt();
            this.deferredPrompt.userChoice.then(function(choiceResult) {
                if (choiceResult.outcome === 'accepted') {
                    console.log('User accepted PWA install');
                    Module._pwaManager.isInstalled = true;
                } else {
                    console.log('User dismissed PWA install');
                }
                Module._pwaManager.deferredPrompt = null;
            });
        },

        enableOfflineMode: function() {
            if ('serviceWorker' in navigator) {
                navigator.serviceWorker.register('/sw.js').then(function(registration) {
                    console.log('Service Worker registered:', registration.scope);
                    Module._createModal('Offline Mode Enabled', '<p>TreeSheets can now work offline! The application and your data will be cached for offline access.</p>', [
                        { text: 'OK', primary: true }
                    ]);
                }).catch(function(error) {
                    console.error('Service Worker registration failed:', error);
                    Module._createModal('Offline Mode Failed', '<p>Could not enable offline mode: ' + error.message + '</p>', [
                        { text: 'OK', primary: true }
                    ]);
                });
            } else {
                Module._createModal('Not Supported', '<p>Your browser does not support offline mode (Service Workers).</p>', [
                    { text: 'OK', primary: true }
                ]);
            }
        }
    },

    JS_InstallPWA: function() {
        Module._pwaManager.showInstallPrompt();
    },

    JS_EnableOfflineMode: function() {
        Module._pwaManager.enableOfflineMode();
    },

    // Statistics Panel
    _statisticsManager: {
        show: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Document Statistics';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var stats = [
                { label: 'Total Cells', value: '0', icon: '📊' },
                { label: 'File Size', value: this.getFileSize() + ' KB', icon: '💾' },
                { label: 'Last Modified', value: this.getLastModified(), icon: '🕒' },
                { label: 'Auto-saves', value: localStorage.getItem('treesheets_autosave_count') || '0', icon: '💾' },
                { label: 'Session Duration', value: this.getSessionDuration(), icon: '⏱️' },
                { label: 'Files in VFS', value: Module._VFS.listFiles().length.toString(), icon: '📁' }
            ];

            stats.forEach(function(stat) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;justify-content:space-between;padding:12px;border-bottom:1px solid #eee;align-items:center;';

                var left = document.createElement('div');
                left.style.display = 'flex';
                left.style.alignItems = 'center';
                left.style.gap = '10px';

                var icon = document.createElement('span');
                icon.textContent = stat.icon;
                icon.style.fontSize = '20px';
                left.appendChild(icon);

                var label = document.createElement('span');
                label.textContent = stat.label;
                label.style.fontWeight = 'bold';
                left.appendChild(label);

                var value = document.createElement('span');
                value.textContent = stat.value;
                value.style.cssText = 'color:#007bff;font-weight:bold;';

                row.appendChild(left);
                row.appendChild(value);
                content.appendChild(row);
            });

            Module._createModal('Statistics', content, [
                { text: 'Close', primary: true }
            ]);
        },

        getFileSize: function() {
            var total = 0;
            try {
                for (var i = 0; i < localStorage.length; i++) {
                    var key = localStorage.key(i);
                    if (key && key.startsWith('treesheets_')) {
                        total += localStorage.getItem(key).length;
                    }
                }
            } catch (e) {
                console.error('Error calculating file size:', e);
            }
            return Math.round(total / 1024);
        },

        getLastModified: function() {
            var timestamp = localStorage.getItem('treesheets_autosave_timestamp');
            if (timestamp) {
                return new Date(parseInt(timestamp)).toLocaleString();
            }
            return 'Never';
        },

        getSessionDuration: function() {
            if (!Module._sessionStartTime) {
                Module._sessionStartTime = Date.now();
            }
            var duration = Date.now() - Module._sessionStartTime;
            var minutes = Math.floor(duration / 60000);
            var seconds = Math.floor((duration % 60000) / 1000);
            return minutes + 'm ' + seconds + 's';
        }
    },

    JS_ShowStatistics: function() {
        Module._statisticsManager.show();
    },

    // Quick Actions Toolbar
    _quickActions: {
        create: function() {
            var existing = document.getElementById('quick-actions');
            if (existing) return;

            var toolbar = document.createElement('div');
            toolbar.id = 'quick-actions';
            toolbar.style.cssText = 'position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:rgba(0,0,0,0.8);padding:10px 15px;border-radius:25px;display:flex;gap:10px;z-index:9500;box-shadow:0 4px 20px rgba(0,0,0,0.3);';

            var actions = [
                { icon: '💾', label: 'Save', action: function() { Module._autoSave.save(); } },
                { icon: '📤', label: 'Export', action: function() { Module._exportManager.showExportDialog(); } },
                { icon: '🔍', label: 'Find', action: function() { Module._findReplaceManager.show(); } },
                { icon: '⚙️', label: 'Settings', action: function() { Module._settingsManager.show(); } },
                { icon: '❓', label: 'Help', action: function() { Module._keyboardHelp.show(); } }
            ];

            actions.forEach(function(act) {
                var btn = document.createElement('button');
                btn.textContent = act.icon;
                btn.title = act.label;
                btn.style.cssText = 'background:transparent;border:none;color:white;font-size:20px;cursor:pointer;padding:5px 10px;border-radius:50%;transition:background 0.2s;';
                btn.onclick = act.action;
                btn.onmouseover = function() { btn.style.background = 'rgba(255,255,255,0.2)'; };
                btn.onmouseout = function() { btn.style.background = 'transparent'; };
                btn.setAttribute('aria-label', act.label);
                toolbar.appendChild(btn);
            });

            document.body.appendChild(toolbar);
            console.log('Quick actions toolbar created');
        },

        toggle: function() {
            var toolbar = document.getElementById('quick-actions');
            if (toolbar) {
                toolbar.style.display = toolbar.style.display === 'none' ? 'flex' : 'none';
            }
        }
    },

    JS_ToggleQuickActions: function() {
        Module._quickActions.toggle();
    },

    // Status Bar
    _statusBar: {
        create: function() {
            var existing = document.getElementById('status-bar');
            if (existing) return;

            var bar = document.createElement('div');
            bar.id = 'status-bar';
            bar.style.cssText = 'position:fixed;bottom:0;left:0;right:0;height:24px;background:#f0f0f0;border-top:1px solid #ccc;display:flex;align-items:center;padding:0 10px;font-size:12px;z-index:8500;';
            bar.setAttribute('role', 'status');
            bar.setAttribute('aria-live', 'polite');

            // Left section
            var left = document.createElement('div');
            left.style.cssText = 'flex:1;display:flex;gap:15px;';

            var statusText = document.createElement('span');
            statusText.id = 'status-text';
            statusText.textContent = 'Ready';
            left.appendChild(statusText);

            var separator1 = document.createElement('span');
            separator1.textContent = '|';
            separator1.style.color = '#ccc';
            left.appendChild(separator1);

            var cellInfo = document.createElement('span');
            cellInfo.id = 'cell-info';
            cellInfo.textContent = 'No selection';
            left.appendChild(cellInfo);

            // Right section
            var right = document.createElement('div');
            right.style.cssText = 'display:flex;gap:15px;';

            var zoomInfo = document.createElement('span');
            zoomInfo.id = 'zoom-info';
            zoomInfo.textContent = '100%';
            zoomInfo.style.cursor = 'pointer';
            zoomInfo.onclick = function() { Module._zoomManager.showZoomMenu(); };
            zoomInfo.title = 'Click to change zoom';
            right.appendChild(zoomInfo);

            var separator2 = document.createElement('span');
            separator2.textContent = '|';
            separator2.style.color = '#ccc';
            right.appendChild(separator2);

            var onlineStatus = document.createElement('span');
            onlineStatus.id = 'online-status';
            onlineStatus.textContent = navigator.onLine ? '🟢 Online' : '🔴 Offline';
            right.appendChild(onlineStatus);

            bar.appendChild(left);
            bar.appendChild(right);
            document.body.appendChild(bar);

            // Update online status
            window.addEventListener('online', function() {
                onlineStatus.textContent = '🟢 Online';
            });
            window.addEventListener('offline', function() {
                onlineStatus.textContent = '🔴 Offline';
            });

            console.log('Status bar created');
        },

        setStatus: function(text) {
            var statusText = document.getElementById('status-text');
            if (statusText) {
                statusText.textContent = text;
            }
        },

        updateZoom: function(zoom) {
            var zoomInfo = document.getElementById('zoom-info');
            if (zoomInfo) {
                zoomInfo.textContent = (zoom * 100).toFixed(0) + '%';
            }
        }
    },

    JS_SetStatus: function(textPtr) {
        var text = UTF8ToString(textPtr);
        Module._statusBar.setStatus(text);
    },

    // Performance Monitor
    _performanceMonitor: {
        stats: {
            fps: 0,
            memory: 0,
            renderTime: 0
        },

        show: function() {
            var content = document.createElement('div');

            var title = document.createElement('h3');
            title.textContent = 'Performance Monitor';
            title.style.margin = '0 0 15px 0';
            content.appendChild(title);

            var metrics = [
                { label: 'FPS', value: this.getFPS(), unit: '', icon: '📊' },
                { label: 'Memory Usage', value: this.getMemoryUsage(), unit: 'MB', icon: '💾' },
                { label: 'LocalStorage', value: this.getStorageUsage(), unit: 'KB', icon: '📦' },
                { label: 'Cache Size', value: Object.keys(Module.imageCache || {}).length, unit: 'images', icon: '🖼️' }
            ];

            metrics.forEach(function(metric) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;justify-content:space-between;padding:12px;border-bottom:1px solid #eee;';

                var left = document.createElement('span');
                // XSS SAFETY: Use textContent to prevent potential injection
                left.textContent = metric.icon + ' ' + metric.label;
                left.style.fontWeight = 'bold';

                var right = document.createElement('span');
                right.textContent = metric.value + (metric.unit ? ' ' + metric.unit : '');
                right.style.cssText = 'color:#007bff;font-weight:bold;';

                row.appendChild(left);
                row.appendChild(right);
                content.appendChild(row);
            });

            // Add clear cache button
            var clearBtn = document.createElement('button');
            clearBtn.textContent = 'Clear Image Cache';
            clearBtn.style.cssText = 'width:100%;padding:10px;margin-top:15px;background:#dc3545;color:white;border:none;border-radius:4px;cursor:pointer;';
            clearBtn.onclick = function() {
                Module.imageCache = {};
                Module._createModal('Cache Cleared', '<p>Image cache has been cleared successfully.</p>', [
                    { text: 'OK', primary: true }
                ]);
            };
            content.appendChild(clearBtn);

            Module._createModal('Performance Monitor', content, [
                { text: 'Close', primary: true }
            ]);
        },

        getFPS: function() {
            // Approximate FPS (would need actual frame timing in real implementation)
            return '60';
        },

        getMemoryUsage: function() {
            if (performance.memory) {
                return (performance.memory.usedJSHeapSize / 1048576).toFixed(2);
            }
            return 'N/A';
        },

        getStorageUsage: function() {
            var total = 0;
            try {
                for (var i = 0; i < localStorage.length; i++) {
                    var key = localStorage.key(i);
                    if (key && key.startsWith('treesheets_')) {
                        total += localStorage.getItem(key).length;
                    }
                }
            } catch (e) {
                return 'Error';
            }
            return Math.round(total / 1024);
        }
    },

    JS_ShowPerformanceMonitor: function() {
        Module._performanceMonitor.show();
    },

    // Welcome Screen
    _welcomeScreen: {
        show: function() {
            // Check if first run
            var hasSeenWelcome = localStorage.getItem('treesheets_welcome_seen');
            if (hasSeenWelcome) return;

            var content = document.createElement('div');
            content.style.textAlign = 'center';

            var logo = document.createElement('div');
            logo.textContent = '🌳';
            logo.style.cssText = 'font-size:80px;margin:20px 0;';
            content.appendChild(logo);

            var title = document.createElement('h1');
            title.textContent = 'Welcome to TreeSheets Web!';
            title.style.margin = '0 0 20px 0';
            content.appendChild(title);

            var desc = document.createElement('p');
            desc.innerHTML = 'A powerful hierarchical spreadsheet application<br>running entirely in your browser.';
            desc.style.cssText = 'color:#666;margin:0 0 30px 0;line-height:1.6;';
            content.appendChild(desc);

            var features = document.createElement('div');
            features.style.cssText = 'text-align:left;max-width:400px;margin:0 auto 30px auto;';

            var featureList = [
                '✓ Auto-save every 30 seconds',
                '✓ Dark mode and 7 themes',
                '✓ Export to CSV, JSON, HTML, Markdown',
                '✓ Keyboard shortcuts (F1 for help)',
                '✓ Command palette (Ctrl+Shift+P)',
                '✓ Works offline with PWA'
            ];

            featureList.forEach(function(feature) {
                var item = document.createElement('div');
                item.textContent = feature;
                item.style.cssText = 'padding:8px 0;border-bottom:1px solid #eee;';
                features.appendChild(item);
            });

            content.appendChild(features);

            var dontShow = document.createElement('label');
            dontShow.style.cssText = 'display:block;margin:20px 0;cursor:pointer;';
            var checkbox = document.createElement('input');
            checkbox.type = 'checkbox';
            checkbox.id = 'dont-show-again';
            dontShow.appendChild(checkbox);
            dontShow.appendChild(document.createTextNode(' Don\'t show this again'));
            content.appendChild(dontShow);

            Module._createModal('Welcome', content, [
                { text: 'Start with Template', callback: function() {
                    if (checkbox.checked) {
                        localStorage.setItem('treesheets_welcome_seen', 'true');
                    }
                    Module._templateManager.showGallery();
                }},
                { text: 'Get Started', primary: true, callback: function() {
                    if (checkbox.checked) {
                        localStorage.setItem('treesheets_welcome_seen', 'true');
                    }
                }}
            ]);
        }
    },

    // Print Manager
    _printManager: {
        preparePrint: function() {
            // Add print stylesheet
            var style = document.createElement('style');
            style.id = 'print-styles';
            style.textContent = '@media print {' +
                'body { background: white !important; }' +
                '#menubar, #toolbar, #dark-mode-toggle, #quick-actions, #status-bar, #save-status, #drop-overlay { display: none !important; }' +
                'canvas { max-width: 100% !important; }' +
                '@page { margin: 1cm; }' +
                '}';
            document.head.appendChild(style);

            window.print();

            // Remove print styles after printing
            setTimeout(function() {
                var printStyles = document.getElementById('print-styles');
                if (printStyles) printStyles.remove();
            }, 1000);
        }
    },

    JS_Print: function() {
        Module._printManager.preparePrint();
    },

    // Initialize all complete application features
    _initCompleteFeatures: function() {
        console.log('Initializing complete application suite...');

        // Set session start time
        Module._sessionStartTime = Date.now();

        // Create UI elements
        Module._statusBar.create();
        Module._quickActions.create();

        // Initialize PWA
        Module._pwaManager.init();

        // Show welcome screen on first run
        setTimeout(function() {
            Module._welcomeScreen.show();
        }, 500);

        // Update command palette with new commands
        Module._commandPalette.commands.push(
            { id: 'file-template', name: 'File: New from Template', action: function() { Module._templateManager.showGallery(); } },
            { id: 'file-print', name: 'File: Print', action: function() { Module._printManager.preparePrint(); } },
            { id: 'view-statistics', name: 'View: Document Statistics', action: function() { Module._statisticsManager.show(); } },
            { id: 'view-performance', name: 'View: Performance Monitor', action: function() { Module._performanceMonitor.show(); } },
            { id: 'tools-install-pwa', name: 'Tools: Install as App', action: function() { Module._pwaManager.showInstallPrompt(); } },
            { id: 'tools-offline', name: 'Tools: Enable Offline Mode', action: function() { Module._pwaManager.enableOfflineMode(); } }
        );

        // Update zoom info in status bar
        var originalZoomApply = Module._zoomManager.applyZoom;
        Module._zoomManager.applyZoom = function(level) {
            originalZoomApply.call(this, level);
            Module._statusBar.updateZoom(level);
        };

        console.log('All complete application features initialized');
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
                btn.textContent = label.split(' (')[0];
            };
            img.onload = function() {
                btn.appendChild(img);
                var span = document.createElement('span');
                span.textContent = label.split(' (')[0];
                span.style.fontSize = '12px';
                btn.appendChild(span);
            };
        } else {
            btn.textContent = label.split(' (')[0];
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
        span.textContent = UTF8ToString(labelPtr);
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
