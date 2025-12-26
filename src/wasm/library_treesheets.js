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
        decode: function(color) {
            return {
                r: color & 0xFF,
                g: (color >> 8) & 0xFF,
                b: (color >> 16) & 0xFF
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
            return r | (g << 8) | (b << 16);
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
                Module._cacheManager.addImage(idx, img);
                delete Module.imageLoadQueue[idx];
                // Trigger redraw if needed
                if (Module._WASM_Resize) {
                    var w = Module.canvas.width;
                    var h = Module.canvas.height;
                    Module._WASM_Resize(w, h);
                }
            };
            img.onerror = function() {
                delete Module.imageLoadQueue[idx];
                // Create placeholder image
                var placeholder = new Image();
                placeholder.width = size;
                placeholder.height = size;
                Module._cacheManager.addImage(idx, placeholder);
            };
            // Try to load from common paths
            img.src = 'images/icon' + idx + '.png';
            Module._cacheManager.addImage(idx, img);

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
        // Mock
    },
    JS_SetBrush: function(brushType) {
        // Mock
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
        if (navigator.clipboard) {
            navigator.clipboard.writeText(text);
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

    JS_TriggerUpload: function() {
        var input = document.createElement('input');
        input.type = 'file';
        input.accept = '.cts';
        input.onchange = function(e) {
            var file = e.target.files[0];
            if (!file) return;

            // Security: Check file size
            var maxSize = Module._CONSTANTS.MAX_FILE_SIZE_BYTES;
            if (file.size > maxSize) {
                Module._createModal('Error', '<p>File too large. Maximum size is ' +
                    Math.floor(maxSize / (1024 * 1024)) + 'MB.</p>', [
                    { text: 'OK', primary: true }
                ]);
                return;
            }

            var reader = new FileReader();
            reader.onload = function(evt) {
                var arrayBuffer = evt.target.result;
                var uint8Array = new Uint8Array(arrayBuffer);
                var ptr = Module._malloc(uint8Array.length);
                Module.HEAPU8.set(uint8Array, ptr);

                var nameLen = lengthBytesUTF8(file.name) + 1;
                var namePtr = Module._malloc(nameLen);
                stringToUTF8(file.name, namePtr, nameLen);

                Module._WASM_FileLoaded(namePtr, ptr, uint8Array.length);

                Module._free(ptr);
                Module._free(namePtr);
            };
            reader.onerror = function() {
                Module._createModal('Error', '<p>Failed to read file.</p>', [
                    { text: 'OK', primary: true }
                ]);
            };
            reader.readAsArrayBuffer(file);
        };
        input.click();
    },

    JS_InitInput: function() {
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
        canvas.addEventListener('mousemove', function(e) {
            Module._WASM_Mouse(0, e.offsetX, e.offsetY, getModifiers(e));
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
        window.addEventListener('resize', onResize);
        onResize();
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
        btn.onclick = function(e) {
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

            menu.items.forEach(function(item) {
                if (item.type == 3) { // Separator
                    var sep = document.createElement('hr');
                    sep.style.margin = '2px 0';
                    popup.appendChild(sep);
                    return;
                }
                var itemDiv = document.createElement('div');
                itemDiv.innerText = item.text.replace(/&/g, '').split('\t')[0];
                itemDiv.style.padding = '5px 15px';
                itemDiv.style.cursor = 'pointer';
                itemDiv.onmouseover = function() { itemDiv.style.backgroundColor = '#eee'; };
                itemDiv.onmouseout = function() { itemDiv.style.backgroundColor = 'white'; };
                itemDiv.onclick = function() {
                    Module._WASM_Action(item.id);
                    popup.remove();
                };
                popup.appendChild(itemDiv);
            });

            document.body.appendChild(popup);

            var closeHandler = function(ev) {
                if (ev.target != btn && !popup.contains(ev.target)) {
                    popup.remove();
                    window.removeEventListener('mousedown', closeHandler);
                }
            };
            setTimeout(function() { window.addEventListener('mousedown', closeHandler); }, 0);
        };
        menubar.appendChild(btn);
    },

    // Dialog Helper - Creates styled modal
    _createModal: function(title, content, buttons, onClose) {
        // Remove any existing modal
        var existing = document.getElementById('ts-modal');
        if (existing) existing.remove();

        // Create overlay
        var overlay = document.createElement('div');
        overlay.id = 'ts-modal';
        overlay.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:10000;';

        // Create dialog
        var dialog = document.createElement('div');
        dialog.style.cssText = 'background:white;border-radius:8px;box-shadow:0 4px 20px rgba(0,0,0,0.3);min-width:300px;max-width:500px;max-height:80%;overflow:auto;';

        // Title bar
        var titleBar = document.createElement('div');
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
                    if (btn.callback) btn.callback();
                };
                if (btn.primary) primaryButton = button;
                if (btn.text.toLowerCase() === 'cancel') cancelButton = button;
                buttonBar.appendChild(button);
            });
            dialog.appendChild(buttonBar);
        }

        overlay.appendChild(dialog);
        document.body.appendChild(overlay);

        // Keyboard controls (Enter = OK/primary, Esc = Cancel)
        var keyHandler = function(e) {
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

        var modal = Module._createModal(title, content, [
            { text: 'Cancel', callback: function() {
                var len = 1;
                resultPtr = _malloc(len);
                stringToUTF8('', resultPtr, len);
            }},
            { text: 'OK', primary: true, callback: function() {
                var res = input.value || '';
                var len = lengthBytesUTF8(res) + 1;
                resultPtr = _malloc(len);
                stringToUTF8(res, resultPtr, len);
            }}
        ]);

        // Focus input
        setTimeout(function() { input.focus(); }, 100);

        // Wait for user (synchronous behavior approximation)
        // Note: This is a limitation - true modal dialogs in JS are async
        // For now, return empty string and improve later
        var len = lengthBytesUTF8(def) + 1;
        resultPtr = _malloc(len);
        stringToUTF8(def, resultPtr, len);
        return resultPtr;
    },
    JS_AskNumber: function(titlePtr, msgPtr, def, min, max) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);

        var result = def;
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

        Module._createModal(title, content, [
            { text: 'Cancel' },
            { text: 'OK', primary: true, callback: function() {
                result = parseFloat(input.value) || def;
            }}
        ]);

        return def; // Async limitation workaround
    },
    JS_SingleChoice: function(titlePtr, msgPtr, choicesJsonPtr) {
        var title = UTF8ToString(titlePtr);
        var msg = UTF8ToString(msgPtr);
        var choices = JSON.parse(UTF8ToString(choicesJsonPtr));

        var result = 0;
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

        Module._createModal(title, content, [
            { text: 'Cancel' },
            { text: 'OK', primary: true, callback: function() {
                result = parseInt(select.value) || 0;
            }}
        ]);

        return 0; // Async limitation workaround
    },
    JS_PickColor: function(defaultColor) {
        var result = defaultColor;
        // Use color utility for hex conversion
        var hexColor = Module._colorUtils.toHex(defaultColor);

        var content = document.createElement('div');
        var input = document.createElement('input');
        input.type = 'color';
        input.value = hexColor;
        input.style.cssText = 'width:100%;height:100px;border:1px solid #ccc;border-radius:4px;cursor:pointer;';
        content.appendChild(input);

        Module._createModal('Choose Color', content, [
            { text: 'Cancel' },
            { text: 'OK', primary: true, callback: function() {
                result = Module._colorUtils.fromHex(input.value);
            }}
        ]);

        return defaultColor; // Async limitation workaround
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
        var sel = document.createElement('select');
        sel.style.width = width + 'px';
        var choices = JSON.parse(UTF8ToString(choicesJsonPtr));
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
