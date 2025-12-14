#include "wx_shim.h"
#include "../tools.h"
#include "../ts_graphics.h"
#include "ts_graphics_wasm.h"
#include "ts_graphics_web.h"

#include <map>
#include <vector>
#include <memory>
#include <climits>
#include <cassert>

using namespace std;

// Globals
const int g_margin_extra = 2;
const int g_line_width = 1;
const int g_selmargin = 2;
const int g_deftextsize_default = 12;
static auto g_deftextsize = g_deftextsize_default;
const int g_mintextsize_delta = 8;
const int g_maxtextsize_delta = 32;
static int g_mintextsize() { return g_deftextsize - g_mintextsize_delta; }
static int g_maxtextsize() { return g_deftextsize + g_maxtextsize_delta; }
const uint g_cellcolor_default = 0xFFFFFF;
const uint g_textcolor_default = 0x000000;
const int g_grid_margin = 0;
const int g_cell_margin = 0;
const int g_tagcolor_default = 0xFF0000;
const int g_bordercolor_default = 0xA0A0A0;
const int g_usergridouterspacing_default = 3;

enum { DS_GRID, DS_BLOBSHIER, DS_BLOBLINE };
enum { STYLE_BOLD=1, STYLE_ITALIC=2, STYLE_FIXED=4, STYLE_UNDERLINE=8, STYLE_STRIKETHRU=16 };
enum { A_EXPTEXT, A_EXPCSV, A_EXPXML, A_EXPHTMLT, A_EXPHTMLTI, A_EXPHTMLTE, A_EXPHTMLB, A_EXPHTMLO,
       A_CELLCOLOR, A_TEXTCOLOR, A_BORDCOLOR };
enum { TS_TEXT = 0, TS_GRID = 1, TS_BOTH = 2, TS_NEITHER = 3 };
const auto TS_SELECTION_MASK = 0x80;

static const std::map<char, pair<wxBitmapType, wxString>> imagetypes = {
    {'I', {wxBITMAP_TYPE_PNG, "image/png"}}, {'J', {wxBITMAP_TYPE_JPEG, "image/jpeg"}}};

// Mocks
wxString wxBase64Encode(const void* data, size_t len) { return ""; }
inline uint LightColor(uint c) { return c; }
inline void DrawRectangle(TSGraphics &dc, uint color, int x, int y, int xs, int ys, bool outline = false) {
    dc.DrawRectangle(x,y,xs,ys);
}
inline bool wxLaunchDefaultBrowser(const wxString& url) { return false; }
inline bool wxLaunchDefaultApplication(const wxString& url) { return false; }

struct wasm_treesheets {
    struct Cell;
    struct Grid;
    struct Text;
    struct Document;
    struct Selection;
    struct Image;
    struct System;

    static System* sys;

    struct TSFrame {
        wxBitmap line_nw, line_sw;
    };

    // Mocks needing Document/Selection
    static void HintIMELocation(Document* doc, int x, int y, int h, int style) {}

    // Stub Document minimal for Cell.h
    struct Document {
        Cell* currentdrawroot = nullptr;
        Cell* root = nullptr;
        int hierarchysize = 0;
        std::map<wxString, uint> tags;

        struct Canvas {
            void Refresh() {}
        } *canvas = new Canvas();

        int scrolly=0, maxy=0, scrollx=0, maxx=0;
        std::vector<Selection> drawpath;
        Selection hover;

        bool PickFont(TSGraphics &dc, int depth, int relsize, int stylebits) { return false; }
        uint Background() { return 0xFFFFFF; }
        void AddUndo(Cell* c) {}
        Cell* WalkPath(const std::vector<Selection>& path) { return root; }
    };

    // Selection stub
    struct Selection {
        Grid* grid = nullptr;
        int x=0, y=0, xs=0, ys=0;
        int cursor=0, cursorend=0;

        bool TextEdit() const { return false; }
        Cell* GetCell() const { return nullptr; }
        void EnterEdit(Document* doc, int c=0, int ce=0) {}
        bool Thin() const { return xs == 0 || ys == 0; }
        void ExitEdit(Document* doc) {}
    };

    // Image Stub
    struct Image {
        uint64_t hash;
        double display_scale;
        std::vector<uint8_t> data;
        char type;
        int pixel_width = 0;
        int savedindex = 0;
        int trefc = 0;

        wxBitmap& Display() { static wxBitmap b; return b; }
        wxString GetFileExtension() { return ""; }
    };

    // Core Includes
    #include "../text.h"
    #include "../grid.h"
    #include "../cell.h"

    // System definition
    struct System {
        int defaultmaxcolwidth = 80;
        int roundness = 3;
        bool casesensitivesearch = true;
        bool darkennonmatchingcells = false;
        bool fastrender = true;
        bool darkmode = false;
        uchar versionlastloaded = 0;
        wxString searchstring;
        wxString defaultfont = "Arial";
        wxString defaultfixedfont = "Courier";
        int sortcolumn = 0;
        int sortxs = 0;
        bool sortdescending = false;
        wxLongLong fakelasteditonload;
        uint cursorcolor = 0;

        TSFrame* frame = new TSFrame();

        struct MockPen {};
        MockPen pen_gridlines, pen_tinygridlines, pen_thinselect, pen_tinytext;

        void ImageSize(wxBitmap *bm, int &xs, int &ys) {
            if (!bm) return;
            xs = bm->GetWidth();
            ys = bm->GetHeight();
        }

        std::vector<int> loadimageids;
        std::vector<std::unique_ptr<Image>> imagelist;

        System(bool portable) {}
    };
};

wasm_treesheets::System* wasm_treesheets::sys = nullptr;

int main() {
    wasm_treesheets::sys = new wasm_treesheets::System(false);
    // TSWasmGraphics g;
    TSWebGraphics g;

    auto c = new wasm_treesheets::Cell();
    c->text.t = "WASM Ready";

    wasm_treesheets::Document doc;
    doc.currentdrawroot = c;

    // Layout
    c->Layout(&doc, g, 0, 80, false);

    std::cout << "Cell calculated width: " << c->sx << std::endl;

    return 0;
}
