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
 * Modifier Bitflags:
 *   Bit 0 (1): Ctrl
 *   Bit 1 (2): Shift
 *   Bit 2 (4): Alt
 *   Bit 3 (8): Meta/Command
 */
mergeInto(LibraryManager.library, {
    JS_DrawRectangle: function(x, y, w, h) {
        var ctx = Module.canvas.getContext('2d');
        ctx.fillRect(x, y, w, h);
        ctx.strokeRect(x, y, w, h);
    },
    JS_DrawRoundedRectangle: function(x, y, w, h, radius) {
        var ctx = Module.canvas.getContext('2d');
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
        var ctx = Module.canvas.getContext('2d');
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
    },
    JS_DrawText: function(str, x, y) {
        var ctx = Module.canvas.getContext('2d');
        var s = UTF8ToString(str);
        ctx.fillText(s, x, y);
    },
    JS_DrawBitmap: function(idx, x, y) {
        var ctx = Module.canvas.getContext('2d');
        ctx.fillStyle = 'rgba(255,0,0,0.5)';
        ctx.fillRect(x, y, 16, 16);
    },
    JS_GetCharHeight: function() {
        var ctx = Module.canvas.getContext('2d');
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
        var ctx = Module.canvas.getContext('2d');
        var s = UTF8ToString(str);
        return Math.ceil(ctx.measureText(s).width);
    },
    JS_GetTextHeight: function(str) {
        var ctx = Module.canvas.getContext('2d');
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
        var ctx = Module.canvas.getContext('2d');
        var r = c & 0xFF;
        var g = (c >> 8) & 0xFF;
        var b = (c >> 16) & 0xFF;
        ctx.fillStyle = 'rgb(' + r + ',' + g + ',' + b + ')';
    },
    JS_SetPenColor: function(c) {
        var ctx = Module.canvas.getContext('2d');
        var r = c & 0xFF;
        var g = (c >> 8) & 0xFF;
        var b = (c >> 16) & 0xFF;
        ctx.strokeStyle = 'rgb(' + r + ',' + g + ',' + b + ')';
    },
    JS_SetTextForeground: function(c) {
        var ctx = Module.canvas.getContext('2d');
        var r = c & 0xFF;
        var g = (c >> 8) & 0xFF;
        var b = (c >> 16) & 0xFF;
        ctx.fillStyle = 'rgb(' + r + ',' + g + ',' + b + ')';
    },
    JS_SetTextBackground: function(c) {
    },
    JS_SetFont: function(size, stylebits) {
        var ctx = Module.canvas.getContext('2d');
        var font = '';
        if(stylebits & 2) font += 'italic ';
        if(stylebits & 1) font += 'bold ';
        font += size + 'px ';
        if(stylebits & 4) font += 'monospace';
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

    JS_TriggerUpload: function() {
        var input = document.createElement('input');
        input.type = 'file';
        input.accept = '.cts';
        input.onchange = function(e) {
            var file = e.target.files[0];
            if (!file) return;
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

    // Dialogs
    JS_ShowMessage: function(titlePtr, msgPtr) {
        alert(UTF8ToString(titlePtr) + "\n\n" + UTF8ToString(msgPtr));
    },
    JS_AskText: function(titlePtr, msgPtr, defPtr) {
        var res = prompt(UTF8ToString(titlePtr) + "\n" + UTF8ToString(msgPtr), UTF8ToString(defPtr));
        if (res === null) res = "";
        var len = lengthBytesUTF8(res) + 1;
        var ptr = _malloc(len);
        stringToUTF8(res, ptr, len);
        return ptr;
    },
    JS_AskNumber: function(titlePtr, msgPtr, def, min, max) {
        var res = prompt(UTF8ToString(titlePtr) + "\n" + UTF8ToString(msgPtr), def);
        return parseFloat(res) || def;
    },
    JS_SingleChoice: function(titlePtr, msgPtr, choicesJsonPtr) {
        var choices = JSON.parse(UTF8ToString(choicesJsonPtr));
        var txt = UTF8ToString(titlePtr) + "\n" + UTF8ToString(msgPtr) + "\n";
        choices.forEach(function(c, i) { txt += i + ": " + c + "\n"; });
        var res = prompt(txt, "0");
        return parseInt(res) || 0;
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
        var btn = document.createElement('button');
        btn.innerText = label.split(' (')[0];
        btn.title = label;
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
