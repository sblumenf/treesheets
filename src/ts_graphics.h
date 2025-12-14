#pragma once

#include <wx/string.h>

class TSGraphics {
public:
    virtual ~TSGraphics() {}

    // Drawing operations
    virtual void DrawRectangle(int x, int y, int w, int h) = 0;
    virtual void DrawRoundedRectangle(int x, int y, int w, int h, int radius) = 0;
    virtual void DrawLine(int x1, int y1, int x2, int y2) = 0;
    virtual void DrawText(const wxString& str, int x, int y) = 0;
    virtual void DrawBitmap(const wxBitmap& bmp, int x, int y) = 0;

    // Measurement operations
    virtual int GetCharHeight() = 0;
    virtual void GetTextExtent(const wxString& str, int* w, int* h) = 0;

    // Styling operations
    virtual void SetBrushColor(uint color) = 0;
    virtual void SetPenColor(uint color) = 0;

    virtual void SetTextForeground(uint color) = 0;
    virtual void SetTextBackground(uint color) = 0;

    virtual void SetFont(int size, int stylebits) = 0;

    enum PenType {
        PEN_GRIDLINES,
        PEN_TINYGRIDLINES,
        PEN_THINSELECT,
        PEN_TINYTEXT,
        PEN_RED,
        PEN_LIGHT_GREY,
        PEN_BLACK,
        PEN_WHITE,
        PEN_GREY
    };
    virtual void SetPen(PenType pen) = 0;

    enum BrushType {
        BRUSH_TRANSPARENT,
        BRUSH_WHITE,
        BRUSH_BLACK,
        BRUSH_LIGHT_GREY
    };
    virtual void SetBrush(BrushType brush) = 0;
};
