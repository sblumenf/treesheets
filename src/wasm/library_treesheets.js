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
        // Canvas fillText uses baseline, so we need to adjust y position
        // TreeSheets expects top-left origin for text
        var metrics = ctx.measureText(s);
        var ascent = metrics.actualBoundingBoxAscent || metrics.fontBoundingBoxAscent || 10;
        ctx.fillText(s, x, y + ascent);
    },
    JS_DrawBitmap: function(idx, x, y) {
        var ctx = Module.canvas.getContext('2d');
        ctx.fillStyle = 'rgba(255,0,0,0.5)';
        ctx.fillRect(x, y, 16, 16);
    },
    JS_GetCharHeight: function() {
        var ctx = Module.canvas.getContext('2d');
        // Use font metrics if available, otherwise estimate from font size
        var metrics = ctx.measureText('M');
        if (metrics.fontBoundingBoxAscent !== undefined && metrics.fontBoundingBoxDescent !== undefined) {
            return Math.ceil(metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent);
        }
        if (metrics.actualBoundingBoxAscent !== undefined && metrics.actualBoundingBoxDescent !== undefined) {
            return Math.ceil(metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent);
        }
        // Fallback: parse font size from context
        var fontMatch = ctx.font.match(/(\d+)px/);
        return fontMatch ? parseInt(fontMatch[1]) : 12;
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
        if (metrics.actualBoundingBoxAscent !== undefined && metrics.actualBoundingBoxDescent !== undefined) {
            return Math.ceil(metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent);
        }
        // Fallback: use char height
        var fontMatch = ctx.font.match(/(\d+)px/);
        return fontMatch ? parseInt(fontMatch[1]) : 12;
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

        // Mouse modifiers: bit 0 = left button, bit 1 = right button, bit 2 = middle button
        // bit 4 = ctrl, bit 5 = shift, bit 6 = alt
        var getMouseModifiers = function(e) {
            var mods = 0;
            if (e.ctrlKey || e.metaKey) mods |= 0x10;
            if (e.shiftKey) mods |= 0x20;
            if (e.altKey) mods |= 0x40;
            return mods;
        };

        canvas.addEventListener('mousedown', function(e) {
            Module._WASM_Mouse(1, e.offsetX, e.offsetY, getMouseModifiers(e) | (1 << e.button));
        });
        canvas.addEventListener('mouseup', function(e) {
            Module._WASM_Mouse(2, e.offsetX, e.offsetY, getMouseModifiers(e) | (1 << e.button));
        });
        canvas.addEventListener('mousemove', function(e) {
            Module._WASM_Mouse(0, e.offsetX, e.offsetY, getMouseModifiers(e));
        });
        canvas.addEventListener('wheel', function(e) {
            e.preventDefault();
            // type 3 = wheel, pack delta in modifiers field (simplified)
            var delta = e.deltaY > 0 ? 1 : -1;
            Module._WASM_Mouse(3, e.offsetX, e.offsetY, getMouseModifiers(e) | (delta << 8));
        }, { passive: false });
        canvas.addEventListener('contextmenu', function(e) {
            e.preventDefault();
        });

        // Key modifiers: bit 0 = ctrl, bit 1 = shift, bit 2 = alt
        var getKeyModifiers = function(e) {
            var mods = 0;
            if (e.ctrlKey || e.metaKey) mods |= 0x01;
            if (e.shiftKey) mods |= 0x02;
            if (e.altKey) mods |= 0x04;
            return mods;
        };

        window.addEventListener('keydown', function(e) {
            var mods = getKeyModifiers(e);
            // Prevent browser default for common shortcuts
            if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'o' || e.key === 'n' || e.key === 'p' || e.key === 'z' || e.key === 'y')) {
                e.preventDefault();
            }
            Module._WASM_Key(0, e.keyCode, mods);
        });
        window.addEventListener('keyup', function(e) {
            Module._WASM_Key(1, e.keyCode, getKeyModifiers(e));
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
            Module.menus[parentId].items.push({id: id, text: text, type: type, checked: checked, isSubmenu: false});
        }
    },
    JS_Menu_AppendSubMenu: function(parentId, submenuId, textPtr, helpPtr) {
        var text = UTF8ToString(textPtr);
        if (Module.menus[parentId]) {
            Module.menus[parentId].items.push({id: submenuId, text: text, type: 4, isSubmenu: true, submenuId: submenuId});
        }
    },
    JS_Menu_Check: function(menuId, itemId, checked) {
        if (Module.menus[menuId]) {
            Module.menus[menuId].items.forEach(function(item) {
                if (item.id === itemId) {
                    item.checked = checked;
                }
            });
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
            // Helper function to create menu popup
            var createMenuPopup = function(menuData, x, y, parentPopup) {
                var popup = document.createElement('div');
                popup.className = 'menu-popup';
                popup.style.position = 'absolute';
                popup.style.left = x + 'px';
                popup.style.top = y + 'px';
                popup.style.backgroundColor = 'white';
                popup.style.border = '1px solid #ccc';
                popup.style.boxShadow = '2px 2px 5px rgba(0,0,0,0.2)';
                popup.style.zIndex = 1000 + (parentPopup ? 1 : 0);
                popup.style.minWidth = '150px';

                menuData.items.forEach(function(item) {
                    if (item.type == 3) { // Separator
                        var sep = document.createElement('hr');
                        sep.style.margin = '2px 0';
                        popup.appendChild(sep);
                        return;
                    }
                    var itemDiv = document.createElement('div');
                    var displayText = item.text.replace(/&/g, '').split('\t')[0];

                    // Handle checkbox/radio items
                    if (item.type == 1 || item.type == 2) {
                        displayText = (item.checked ? '✓ ' : '   ') + displayText;
                    }

                    // Handle submenu indicator
                    if (item.isSubmenu) {
                        displayText = displayText + ' ▶';
                    }

                    itemDiv.innerText = displayText;
                    itemDiv.style.padding = '5px 15px';
                    itemDiv.style.cursor = 'pointer';
                    itemDiv.style.whiteSpace = 'nowrap';
                    itemDiv.onmouseover = function() {
                        itemDiv.style.backgroundColor = '#eee';
                        // Show submenu on hover
                        if (item.isSubmenu && Module.menus[item.submenuId]) {
                            // Remove any existing submenus at this level
                            var existingSub = popup.querySelector('.menu-popup');
                            if (existingSub) existingSub.remove();

                            var rect = itemDiv.getBoundingClientRect();
                            var subPopup = createMenuPopup(Module.menus[item.submenuId], rect.right, rect.top, popup);
                            popup.appendChild(subPopup);
                        }
                    };
                    itemDiv.onmouseout = function() {
                        itemDiv.style.backgroundColor = 'white';
                    };
                    itemDiv.onclick = function(ev) {
                        if (!item.isSubmenu) {
                            Module._WASM_Action(item.id);
                            // Close all menus
                            document.querySelectorAll('.menu-popup').forEach(function(p) { p.remove(); });
                        }
                        ev.stopPropagation();
                    };
                    popup.appendChild(itemDiv);
                });

                return popup;
            };

            // Remove existing popups
            document.querySelectorAll('.menu-popup').forEach(function(p) { p.remove(); });

            var popup = createMenuPopup(menu, e.pageX, e.pageY + 20, null);
            popup.id = 'menu-popup';
            document.body.appendChild(popup);

            var closeHandler = function(ev) {
                var isMenuClick = false;
                document.querySelectorAll('.menu-popup').forEach(function(p) {
                    if (p.contains(ev.target)) isMenuClick = true;
                });
                if (ev.target != btn && !isMenuClick) {
                    document.querySelectorAll('.menu-popup').forEach(function(p) { p.remove(); });
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
    JS_IsDarkMode: function() {
        return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 1 : 0;
    },
    JS_SelectFont: function(defaultFontPtr, defaultSize) {
        var fonts = [
            'Arial', 'Helvetica', 'Times New Roman', 'Georgia', 'Verdana',
            'Courier New', 'Consolas', 'Monaco', 'Lucida Console',
            'Trebuchet MS', 'Tahoma', 'Impact', 'Comic Sans MS'
        ];
        var defaultFont = UTF8ToString(defaultFontPtr);

        // Build font list string
        var txt = "Select Font:\n";
        fonts.forEach(function(f, i) { txt += i + ": " + f + "\n"; });
        txt += "\nEnter font number (or -1 to cancel):";

        var fontIdx = parseInt(prompt(txt, "0"));
        if (isNaN(fontIdx) || fontIdx < 0 || fontIdx >= fonts.length) return 0; // Cancelled

        var sizeStr = prompt("Enter font size (8-72):", defaultSize.toString());
        var size = parseInt(sizeStr);
        if (isNaN(size) || size < 8 || size > 72) return 0; // Cancelled

        // Store selected font name and size in Module for retrieval
        var selectedFont = fonts[fontIdx];
        var len = lengthBytesUTF8(selectedFont) + 1;
        var ptr = _malloc(len);
        stringToUTF8(selectedFont, ptr, len);
        Module._lastSelectedFont = ptr;
        Module._lastSelectedFontSize = size;

        return 1; // Success
    },
    JS_GetSelectedFont: function() {
        return Module._lastSelectedFont || 0;
    },
    JS_GetSelectedFontSize: function() {
        return Module._lastSelectedFontSize || 12;
    },
    JS_PickColor: function(defaultColor) {
        // Convert 0xBBGGRR to #RRGGBB for HTML color input
        var r = defaultColor & 0xFF;
        var g = (defaultColor >> 8) & 0xFF;
        var b = (defaultColor >> 16) & 0xFF;
        var hexColor = '#' + ('0' + r.toString(16)).slice(-2) +
                             ('0' + g.toString(16)).slice(-2) +
                             ('0' + b.toString(16)).slice(-2);

        // Create a temporary color input
        var input = document.createElement('input');
        input.type = 'color';
        input.value = hexColor;
        input.style.position = 'fixed';
        input.style.top = '50%';
        input.style.left = '50%';
        input.style.opacity = '0';

        // Use a promise-like approach with synchronous return (simplified)
        // For a proper implementation, this would need async handling
        // For now, prompt for hex color as fallback
        var result = prompt('Enter color (hex format #RRGGBB or RRGGBB):', hexColor);
        if (result === null) return -1; // Cancelled

        // Parse the result
        result = result.replace('#', '');
        if (result.length === 6) {
            var rr = parseInt(result.substring(0, 2), 16);
            var gg = parseInt(result.substring(2, 4), 16);
            var bb = parseInt(result.substring(4, 6), 16);
            // Return as 0xBBGGRR (TreeSheets format)
            return (bb << 16) | (gg << 8) | rr;
        }
        return defaultColor;
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
