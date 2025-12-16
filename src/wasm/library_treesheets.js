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
        return 12;
    },
    JS_GetTextWidth: function(str) {
        var ctx = Module.canvas.getContext('2d');
        var s = UTF8ToString(str);
        return ctx.measureText(s).width;
    },
    JS_GetTextHeight: function(str) {
        return 12;
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
        canvas.addEventListener('mousedown', function(e) { Module._WASM_Mouse(1, e.offsetX, e.offsetY, 0); });
        canvas.addEventListener('mouseup', function(e) { Module._WASM_Mouse(2, e.offsetX, e.offsetY, 0); });
        canvas.addEventListener('mousemove', function(e) { Module._WASM_Mouse(0, e.offsetX, e.offsetY, 0); });
        window.addEventListener('keydown', function(e) { Module._WASM_Key(0, e.keyCode, 0); });
        window.addEventListener('keyup', function(e) { Module._WASM_Key(1, e.keyCode, 0); });

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
            // Insert before canvas. Assuming canvas has id 'canvas' or is Module.canvas
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
            // Very simple popup logic: create a div with items
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
                itemDiv.innerText = item.text.replace(/&/g, '').split('\t')[0]; // Strip accel and shortcut
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

            // Close on click outside
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
        return parseInt(res) || 0; // -1 for cancel not easy with prompt
    }
});
