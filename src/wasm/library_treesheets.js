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

                // Filename
                var nameLen = lengthBytesUTF8(file.name) + 1;
                var namePtr = Module._malloc(nameLen);
                stringToUTF8(file.name, namePtr, nameLen);

                // Call C++
                Module._WASM_FileLoaded(namePtr, ptr, uint8Array.length);

                Module._free(ptr);
                Module._free(namePtr);
            };
            reader.readAsArrayBuffer(file);
        };
        input.click();
    }
});
