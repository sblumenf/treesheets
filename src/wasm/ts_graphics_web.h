#pragma once
#include "../ts_graphics.h"
#include "web_interface.h"

struct TSWebGraphics : public TSGraphics {

    void DrawRectangle(int x, int y, int w, int h) override {
        JS_DrawRectangle(x, y, w, h);
    }

    void DrawRoundedRectangle(int x, int y, int w, int h, int radius) override {
        JS_DrawRoundedRectangle(x, y, w, h, radius);
    }

    void DrawLine(int x1, int y1, int x2, int y2) override {
        JS_DrawLine(x1, y1, x2, y2);
    }

    void DrawText(const wxString& str, int x, int y) override {
        // str.c_str() returns const char* from shim
        JS_DrawText(str.c_str(), x, y);
    }

    void DrawBitmap(const wxBitmap& bmp, int x, int y) override {
        // Phase 1 limitation: no bitmap support yet
        JS_DrawBitmap(0, x, y);
    }

    int GetCharHeight() override {
        return JS_GetCharHeight();
    }

    void GetTextExtent(const wxString& str, int* w, int* h) override {
        if(w) *w = JS_GetTextWidth(str.c_str());
        if(h) *h = JS_GetCharHeight(); // Use char height for now as it's standard
    }

    void SetBrushColor(uint color) override {
        JS_SetBrushColor(color);
    }

    void SetPenColor(uint color) override {
        JS_SetPenColor(color);
    }

    void SetTextForeground(uint color) override {
        JS_SetTextForeground(color);
    }

    void SetTextBackground(uint color) override {
        JS_SetTextBackground(color);
    }

    void SetFont(int size, int stylebits) override {
        JS_SetFont(size, stylebits);
    }

    void SetPen(PenType pen) override {
        JS_SetPen((int)pen);
    }

    void SetBrush(BrushType brush) override {
        JS_SetBrush((int)brush);
    }
};
