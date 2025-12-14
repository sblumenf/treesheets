#pragma once

// This header is expected to be included INSIDE the treesheets struct in main.cpp
// So it has access to 'sys' and 'STYLE_*' enums.

struct TSWxGraphics : public TSGraphics {
    wxDC& dc;

    TSWxGraphics(wxDC& dc) : dc(dc) {}

    void DrawRectangle(int x, int y, int w, int h) override {
        dc.DrawRectangle(x, y, w, h);
    }

    void DrawRoundedRectangle(int x, int y, int w, int h, int radius) override {
        dc.DrawRoundedRectangle(x, y, w, h, radius);
    }

    void DrawLine(int x1, int y1, int x2, int y2) override {
        dc.DrawLine(x1, y1, x2, y2);
    }

    void DrawText(const wxString& str, int x, int y) override {
        dc.DrawText(str, x, y);
    }

    void DrawBitmap(const wxBitmap& bmp, int x, int y) override {
        dc.DrawBitmap(bmp, x, y, true);
    }

    int GetCharHeight() override {
        return dc.GetCharHeight();
    }

    void GetTextExtent(const wxString& str, int* w, int* h) override {
        dc.GetTextExtent(str, w, h);
    }

    void SetBrushColor(uint color) override {
        dc.SetBrush(wxBrush(wxColour(color)));
    }

    void SetPenColor(uint color) override {
        dc.SetPen(wxPen(wxColour(color)));
    }

    void SetTextForeground(uint color) override {
        dc.SetTextForeground(wxColour(color));
    }

    void SetTextBackground(uint color) override {
        dc.SetTextBackground(wxColour(color));
    }

    void SetFont(int size, int stylebits) override {
        wxFont font(size,
                    stylebits & STYLE_FIXED ? wxFONTFAMILY_TELETYPE : wxFONTFAMILY_DEFAULT,
                    stylebits & STYLE_ITALIC ? wxFONTSTYLE_ITALIC : wxFONTSTYLE_NORMAL,
                    stylebits & STYLE_BOLD ? wxFONTWEIGHT_BOLD : wxFONTWEIGHT_NORMAL,
                    (stylebits & STYLE_UNDERLINE) != 0,
                    stylebits & STYLE_FIXED ? sys->defaultfixedfont : sys->defaultfont);
        if (stylebits & STYLE_STRIKETHRU) font.SetStrikethrough(true);
        dc.SetFont(font);
    }

    void SetPen(PenType pen) override {
        switch (pen) {
            case PEN_GRIDLINES: dc.SetPen(sys->pen_gridlines); break;
            case PEN_TINYGRIDLINES: dc.SetPen(sys->pen_tinygridlines); break;
            case PEN_THINSELECT: dc.SetPen(sys->pen_thinselect); break;
            case PEN_TINYTEXT: dc.SetPen(sys->pen_tinytext); break;
            case PEN_RED: dc.SetPen(*wxRED_PEN); break;
            case PEN_LIGHT_GREY: dc.SetPen(*wxLIGHT_GREY_PEN); break;
            case PEN_BLACK: dc.SetPen(*wxBLACK_PEN); break;
            case PEN_WHITE: dc.SetPen(*wxWHITE_PEN); break;
            case PEN_GREY: dc.SetPen(*wxGREY_PEN); break;
        }
    }

    void SetBrush(BrushType brush) override {
        switch (brush) {
            case BRUSH_TRANSPARENT: dc.SetBrush(*wxTRANSPARENT_BRUSH); break;
            case BRUSH_WHITE: dc.SetBrush(*wxWHITE_BRUSH); break;
            case BRUSH_BLACK: dc.SetBrush(*wxBLACK_BRUSH); break;
            case BRUSH_LIGHT_GREY: dc.SetBrush(*wxLIGHT_GREY_BRUSH); break;
        }
    }

    wxDC& GetDC() { return dc; }
};
