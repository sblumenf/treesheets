#pragma once
#include "../ts_graphics.h"
#include "wx_shim.h"

struct TSWasmGraphics : public TSGraphics {
    void DrawRectangle(int x, int y, int w, int h) override {}
    void DrawRoundedRectangle(int x, int y, int w, int h, int radius) override {}
    void DrawLine(int x1, int y1, int x2, int y2) override {}
    void DrawText(const wxString& str, int x, int y) override {}
    void DrawBitmap(const wxBitmap& bmp, int x, int y) override {}

    int GetCharHeight() override { return 10; } // Mock size
    void GetTextExtent(const wxString& str, int* w, int* h) override {
        if(w) *w = (int)str.Len() * 8; // Mock width
        if(h) *h = 10;
    }

    void SetBrushColor(uint color) override {}
    void SetPenColor(uint color) override {}
    void SetTextForeground(uint color) override {}
    void SetTextBackground(uint color) override {}
    void SetFont(int size, int stylebits) override {}
    void SetPen(PenType pen) override {}
    void SetBrush(BrushType brush) override {}
};
