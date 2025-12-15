#pragma once

#include "wx_shim.h" // For basic types if needed, though we should prefer primitive C types for the boundary

extern "C" {
    // Drawing operations
    void JS_DrawRectangle(int x, int y, int w, int h);
    void JS_DrawRoundedRectangle(int x, int y, int w, int h, int radius);
    void JS_DrawLine(int x1, int y1, int x2, int y2);
    void JS_DrawText(const char* str, int x, int y);
    // Bitmap drawing is complex. For now, pass an ID or similar.
    // wxBitmap in shim has no data. We assume image loading happens in JS or we pass pointer.
    // For Phase 1, we might skip bitmaps or just draw a placeholder.
    void JS_DrawBitmap(int imageIndex, int x, int y);

    // Measurement operations
    int JS_GetCharHeight();
    int JS_GetTextWidth(const char* str);
    int JS_GetTextHeight(const char* str);

    // Styling operations
    // Colors passed as uint (0xRRGGBB)
    void JS_SetBrushColor(unsigned int color);
    void JS_SetPenColor(unsigned int color);
    void JS_SetTextForeground(unsigned int color);
    void JS_SetTextBackground(unsigned int color);

    // Font
    void JS_SetFont(int size, int stylebits);

    // Standard Pens/Brushes (enums)
    void JS_SetPen(int penType);
    void JS_SetBrush(int brushType);

    // OS/Filesystem
    void JS_DownloadFile(const char* filename, const uint8_t* data, int size);
    void JS_LaunchBrowser(const char* url);
    void JS_SetClipboardText(const char* text);
    void JS_TriggerUpload(); // Opens file picker
}

// C++ exports for JS
extern "C" {
    void WASM_FileLoaded(const char* filename, const uint8_t* data, int size);
}
