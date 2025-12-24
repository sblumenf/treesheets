#include "wx_shim.h"
#include "../tools.h"
#include "../ts_graphics.h"
#include "ts_graphics_web.h"
#include "../ts_platform_os.h"
#include "ts_platform_os_web.h"
#include "../ts_constants.h"
#include "ts_menu_web.h"
#include "../ts_dialog_web.h"
#include "web_interface.h"
#include "emscripten.h"

#include <map>
#include <vector>
#include <memory>
#include <climits>
#include <cassert>
#include <cstring>
#include <iostream>

using namespace std;

// Globals
int g_deftextsize = g_deftextsize_default;

// Canvas state
static int g_canvasWidth = 800;
static int g_canvasHeight = 600;
static bool g_needsRedraw = true;
static int g_scrollX = 0;
static int g_scrollY = 0;
static int g_mouseX = 0;
static int g_mouseY = 0;

static const std::map<char, pair<wxBitmapType, wxString>> imagetypes = {
    {'I', {wxBITMAP_TYPE_PNG, "image/png"}}, {'J', {wxBITMAP_TYPE_JPEG, "image/jpeg"}}};

// Mocks and implementations
wxString wxBase64Encode(const void* data, size_t len) { return ""; }
inline uint LightColor(uint c) { return c; }
inline void DrawRectangle(TSGraphics &dc, uint color, int x, int y, int xs, int ys, bool outline = false) {
    dc.DrawRectangle(x,y,xs,ys);
}
inline bool wxLaunchDefaultBrowser(const wxString& url) {
    JS_LaunchBrowser(url.c_str());
    return true;
}
inline bool wxLaunchDefaultApplication(const wxString& url) {
    // For web, treat as browser launch
    JS_LaunchBrowser(url.c_str());
    return true;
}

// Stubs for TSFrame members
struct TSApp {
    wxString GetDataPath(const wxString& s) { return s; }
    wxString exename = "treesheets";
};

struct wxIcon {
    void LoadFile(const wxString& f, wxBitmapType t) {}
    void LoadFile(const wxString& f, wxBitmapType t, int w, int h) {}
    bool IsOk() { return true; }
};
struct wxIconBundle {
    void AddIcon(const wxIcon& i) {}
};
struct wxTaskBarIcon {
    void Connect(int, int, void*, void*, void*) {}
    void SetIcon(const wxIcon& i, const wxString& s) {}
    void RemoveIcon() {}
};
using wxTaskBarIconEventHandler = void (*)(void*); // simplified
using wxFileSystemWatcherEventHandler = void (*)(void*);

struct wxFileHistory {
    void UseMenu(TSMenu* m) {} // overloaded for Web
    void AddFilesToMenu() {}
    void AddFileToHistory(const wxString& s) {}
    wxString GetHistoryFile(size_t i) { return ""; }
    size_t GetCount() { return 0; }
    void RemoveFileFromHistory(size_t i) {}
    void Load(struct MockConfig& c) {}
    void Save(struct MockConfig& c) {}
    void SetMenuPathStyle(int i) {}
};
const int wxFH_PATH_SHOW_NEVER = 0;

struct wxFileSystemWatcher {
    void SetOwner(void*) {}
    void Add(const wxString& p, int f) {}
};
struct wxAuiNotebook {
    void AddPage(void*, const wxString&, bool, const wxBitmap&) {}
    void InsertPage(int, void*, const wxString&, bool, const wxBitmap&) {}
    void DeletePage(int) {}
    int GetPageCount() { return 0; }
    int GetSelection() { return 0; }
    void SetSelection(int) {}
    void SetPageText(int, const wxString&) {}
    void* GetPage(int) { return nullptr; }
    void* GetCurrentPage() { return nullptr; }
};
const int wxAUI_NB_TAB_MOVE = 0;
const int wxAUI_NB_TAB_SPLIT = 0;
const int wxAUI_NB_SCROLL_BUTTONS = 0;
const int wxAUI_NB_WINDOWLIST_BUTTON = 0;
const int wxAUI_NB_CLOSE_ON_ALL_TABS = 0;
const int wxAUI_NB_BOTTOM = 0;
const int wxAUI_NB_TOP = 0;
const int wxCENTER = 0;

struct wxAuiManager {
    wxAuiManager(void* w = nullptr) {}
    void AddPane(void*, int) {}
    void Update() {}
    void UnInit() {}
    void ClearEventHashTable() {}
};

struct wxTextCtrl {
    int id; wxString value; int width;
    wxTextCtrl(void* p, int _id, const wxString& s, const wxPoint& pos, const wxSize& sz, int style = 0)
        : id(_id), value(s), width(sz.x) {}
    void SetFocus() {}
    void SetSelection(long, long) {}
    void GetSelection(long*, long*) {}
    void SetInsertionPoint(long) {}
    long GetLineLength(long) { return 0; }
    void Remove(long, long) {}
    void Clear() {}
    wxString GetValue() { return ""; }
    void SetValue(const wxString&) {}
};
const int wxWANTS_CHARS = 0;
const int wxTE_PROCESS_ENTER = 0;

struct ColorDropdown {
    int id;
    ColorDropdown(void* p, int _id, int val) : id(_id) {}
    void ShowPopup() {}
};
struct ImageDropdown {
    int id;
    ImageDropdown(void* p, const wxString& path) : id(A_DDIMAGE) {}
    std::vector<wxString> filenames;
    void ShowPopup() {}
};
struct wxStaticText {
    wxString label;
    wxStaticText(void* p, int id, const wxString& _label) : label(_label) {}
};

struct wxBitmapBundle {
    static wxBitmap FromSVGFile(const wxString& f, const wxSize& s) { return wxBitmap(); }
};

struct wxToolBar {
    wxToolBar() { JS_Toolbar_Create(); }
    void SetOwnBackgroundColour(const wxColour&) {}
    void AddTool(int id, const wxString& name, const wxBitmap&, const wxString&, int) {
        JS_Toolbar_AddTool(id, name.c_str(), "");
    }
    void AddControl(wxStaticText* c) { JS_Toolbar_AddLabel(c->label.c_str()); }
    void AddControl(wxTextCtrl* c) { JS_Toolbar_AddInput(c->id, c->width, c->value.c_str()); }
    void AddControl(ColorDropdown* c) { JS_Toolbar_AddDropdown(c->id, 44, "[\"Colors\"]"); }
    void AddControl(ImageDropdown* c) { JS_Toolbar_AddDropdown(c->id, 44, "[\"Images\"]"); }
    void AddSeparator() { JS_Toolbar_AddSeparator(); }
    void Realize() {}
    void Show(bool) {}
};
const int wxBORDER_NONE = 0;
const int wxTB_HORIZONTAL = 0;
const int wxTB_FLAT = 0;
const int wxTB_NODIVIDER = 0;
const int wxITEM_NORMAL = 0;

struct wxAcceleratorEntry {
    void Set(int, int, int) {}
};
struct wxAcceleratorTable {
    wxAcceleratorTable(int, wxAcceleratorEntry*) {}
};
const int wxACCEL_SHIFT = 0;
const int wxACCEL_CTRL = 0;

struct wxSystemSettings {
    struct Appearance { bool IsDark() { return JS_IsDarkMode() != 0; } };
    static Appearance GetAppearance() { return Appearance(); }
};

struct MockConfig {
    bool Read(const wxString& key, bool* val, bool def = false) { *val = def; return true; }
    wxString Read(const wxString& key, const wxString& def) { return def; }
    long Read(const wxString& key, long def) { return def; }
    void Read(const wxString& key, int* val, int def) { *val = def; }
    void Write(const wxString& key, bool val) {}
    void Write(const wxString& key, long val) {}
    void Write(const wxString& key, const wxString& val) {}
    void SetPath(const wxString& p) {}
    wxString GetPath() { return ""; }
};

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
        wxBitmap foldicon;

        TSApp* app;
        wxIcon icon;
        wxTaskBarIcon taskbaricon;
        TSMenu *editmenupopup = nullptr;
        wxFileHistory filehistory;
        wxFileHistory scripts; // No need for constructor args if stub
        wxFileSystemWatcher *watcher;
        wxAuiNotebook *notebook {nullptr};
        wxAuiManager aui {this};
        bool fromclosebox {true};
        bool watcherwaitingforuser {false};
        wxToolBar *toolbar {nullptr};
        wxColour toolbarbackgroundcolor {0xD8C7BC};
        wxTextCtrl *filter {nullptr};
        wxTextCtrl *replaces {nullptr};
        ColorDropdown *cellcolordropdown {nullptr};
        ColorDropdown *textcolordropdown {nullptr};
        ColorDropdown *bordercolordropdown {nullptr};
        ImageDropdown *imagedropdown {nullptr};
        wxString imagepath;
        int refreshhack {0};
        int refreshhackinstances {0};
        std::map<wxString, wxString> menustrings;

        using TSPlatformMenu = TSWebMenu;
        using TSPlatformMenuBar = TSWebMenuBar;

        // Stub methods for CreateMenus
        void FileHistoryUseMenu(TSMenu* menu) {
            filehistory.UseMenu(menu);
        }
        void ScriptsUseMenu(TSMenu* menu) {
            scripts.UseMenu(menu);
        }
        void SetMenuBar(TSMenuBar* menubar) {
            // TODO: Store menubar for rendering
        }
        void SetDefaultAccelerators() {
            // TODO
        }

        wxSize FromDIP(const wxSize& s) { return s; }
        int FromDIP(int i) { return i; }

        void SetStatus(const wxChar* msg = nullptr) {}
        void SetFileAssoc(wxString& e) {}
        void SetIcons(const wxIconBundle& i) {}
        void SetSize(int w, int h) {}
        void Move(int x, int y) {}
        void Show(bool b) {}
        void Maximize(bool b) {}
        bool IsIconized() { return false; }
        bool IsFullScreen() { return false; }
        void ShowFullScreen(bool b) {}
        void Iconize(bool b = true) {}
        void Close() {}
        void Destroy() {}
        void RequestUserAttention() {}
        void SetTitle(const wxString&) {}
        void SetPageText(int, const wxString&) {}
        wxSize GetSize() { return wxSize(); }
        wxPoint GetPosition() { return wxPoint(); }
        bool IsMaximized() { return true; }
        bool IsActive() { return true; }

        wxToolBar* CreateToolBar(long style = -1, int id = -1, const wxString& name = wxEmptyString) {
            toolbar = new wxToolBar();
            return toolbar;
        }

        // Stubs for wxFrame
        void Connect(int, int, void*, void*, void*) {}

        // Methods called by CreateMenus
        #include "../ts_menu_builder.h"
        #include "../ts_toolbar_builder.h"

        void DeIconize() {}
        static void OnTBIDBLClick(void*) {}
        static void OnFileSystemEvent(void*) {}

        // Stub for canvas access - returns nullptr for now
        struct CanvasStub {
            Document* doc = nullptr;
            void SetFocus() {}
            void Refresh() { g_needsRedraw = true; }
            void CursorScroll(int dx, int dy) {}
        };
        CanvasStub* GetCurrentTab() {
            static CanvasStub stub;
            return &stub;
        }
        void Refresh() { g_needsRedraw = true; }

        TSFrame() {} // Default constructor for stub
    };

    // Selection stub
    struct Selection {
        Grid* grid = nullptr;
        int x=0, y=0, xs=0, ys=0;
        int cursor=0, cursorend=0;

        Selection() {}
        Selection(Grid* g, int _x, int _y, int _xs, int _ys) : grid(g), x(_x), y(_y), xs(_xs), ys(_ys) {}

        bool TextEdit() const { return false; }
        Cell* GetCell() const { return nullptr; }
        void EnterEdit(Document* doc, int c=0, int ce=0) {}
        bool Thin() const { return xs == 0 || ys == 0; }
        void ExitEdit(Document* doc) {}
    };

    // Mocks needing Document/Selection
    static void HintIMELocation(Document* doc, int x, int y, int h, int style) {}

    // Document - holds the cell tree and view state
    struct Document {
        Cell* currentdrawroot = nullptr;
        Cell* root = nullptr;
        int hierarchysize = 0;
        std::map<wxString, uint> tags;

        struct Canvas {
            void Refresh() { g_needsRedraw = true; }
            void GetClientSize(int* w, int* h) { *w = g_canvasWidth; *h = g_canvasHeight; }
        } *canvas = new Canvas();

        int scrolly = 0, maxy = 0, scrollx = 0, maxx = 0;
        int centerx = 0, centery = 0;
        int layoutxs = 0, layoutys = 0;
        std::vector<Selection> drawpath;
        Selection hover;
        Selection selected;
        double currentviewscale = 1.0;
        wxString filename;

        bool PickFont(TSGraphics &dc, int depth, int relsize, int stylebits) {
            int size = g_deftextsize + relsize;
            if (size < 6) size = 6;
            if (size > 100) size = 100;
            dc.SetFont(size, stylebits);
            return true;
        }

        uint Background() { return root ? root->cellcolor : 0xFFFFFF; }
        void AddUndo(Cell* c) {}

        Cell* WalkPath(const std::vector<Selection>& path) {
            if (!root) return nullptr;
            Cell* c = root;
            for (auto& sel : path) {
                if (!c->grid) return c;
                c = c->grid->C(sel.x, sel.y);
            }
            return c;
        }

        void Layout(TSGraphics &dc) {
            if (!root) return;
            ResetFont();
            root->ResetLayout();
            root->LazyLayout(this, dc, 0, false);
            layoutxs = root->sx;
            layoutys = root->sy;
        }

        void ResetFont() {
            // Reset to default font state
        }

        void Render(TSGraphics &dc) {
            if (!root) return;
            currentdrawroot = root;
            Cell* drawroot = WalkPath(drawpath);
            if (!drawroot) drawroot = root;

            // Clear with background color
            dc.SetBrushColor(Background());
            dc.SetPenColor(Background());
            dc.DrawRectangle(0, 0, g_canvasWidth, g_canvasHeight);

            // Layout if needed
            drawroot->ResetLayout();
            drawroot->LazyLayout(this, dc, 0, false);

            // Calculate centering
            if (drawroot->sx < g_canvasWidth) {
                centerx = (g_canvasWidth - drawroot->sx) / 2;
            } else {
                centerx = hierarchysize;
            }
            if (drawroot->sy < g_canvasHeight) {
                centery = (g_canvasHeight - drawroot->sy) / 2;
            } else {
                centery = hierarchysize;
            }

            // Draw the cells
            drawroot->Render(this, centerx - scrollx, centery - scrolly, dc, 0, 0, 0, 0, 0);
        }
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
        std::unique_ptr<TSPlatformOS> os;
        std::unique_ptr<MockConfig> cfg = std::make_unique<MockConfig>(); // Mock Config
        std::unique_ptr<TSDialogs> dialogs;

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

        // Members needed for CreateMenus
        bool showtoolbar = true;
        bool showstatusbar = true;
        bool lefttabs = false;
        bool totray = false;
        bool minclose = false;
        bool singletray = false;
        bool zoomscroll = false;
        bool thinselc = true;
        bool makebaks = true;
        bool autosave = true;
        bool fswatch = true;
        bool centered = true;
        bool followdarkmode = false;
        int autohtmlexport = 0;
        int customcolor = 0xFFFFFF;

        struct Timer { void Stop() {} } every_second_timer;

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

        // Current document being displayed
        std::unique_ptr<Document> currentDoc;

        System(bool portable) {
            frame->app = new TSApp();
        }

        int AddImageToList(double scale, std::vector<uint8_t>&& data, char type) {
            auto img = std::make_unique<Image>();
            img->display_scale = scale;
            img->data = std::move(data);
            img->type = type;
            img->hash = imagelist.size(); // Simple hash for now
            imagelist.push_back(std::move(img));
            return imagelist.size() - 1;
        }

        const wxChar* LoadDBFromStream(wxInputStream& fis, const wxString& filename) {
            wxDataInputStream dis(fis);
            Cell* ics = nullptr;
            char buf[4];

            // Read and verify magic
            fis.Read(buf, 4);
            if (strncmp(buf, "TSFF", 4)) return L"Not a TreeSheets file.";

            // Read version
            fis.Read(&versionlastloaded, 1);
            if (versionlastloaded > TS_VERSION) return L"File of newer version.";

            std::cout << "Loading file version: " << (int)versionlastloaded << std::endl;

            // Read selection info
            auto xs = versionlastloaded >= 21 ? dis.Read8() : 1;
            auto ys = versionlastloaded >= 21 ? dis.Read8() : 1;
            auto zoomlevel = versionlastloaded >= 23 ? dis.Read8() : 0;
            fakelasteditonload = 0;

            loadimageids.clear();

            // Read images and document data
            for (;;) {
                fis.Read(buf, 1);
                switch (*buf) {
                    case 'I':
                    case 'J': {
                        char iti = *buf;
                        if (versionlastloaded < 9) dis.ReadString();
                        auto sc = versionlastloaded >= 19 ? dis.ReadDouble() : 1.0;
                        std::vector<uint8_t> image_data;
                        if (versionlastloaded >= 22) {
                            auto imagelen = (size_t)dis.Read64();
                            image_data.resize(imagelen);
                            fis.Read(image_data.data(), imagelen);
                        }
                        loadimageids.push_back(AddImageToList(sc, std::move(image_data), iti));
                        std::cout << "Loaded image " << loadimageids.size() << std::endl;
                        break;
                    }

                    case 'D': {
                        wxZlibInputStream zis(fis);
                        if (!zis.IsOk()) return L"Cannot decompress file.";
                        wxDataInputStream zdis(zis);
                        int numcells = 0, textbytes = 0;
                        auto root = Cell::LoadWhich(zdis, nullptr, numcells, textbytes, ics);
                        if (!root) return L"File corrupted!";

                        std::cout << "Loaded " << numcells << " cells, " << textbytes << " text bytes" << std::endl;

                        // Create new document
                        currentDoc = std::make_unique<Document>();
                        currentDoc->root = root;
                        currentDoc->filename = filename;

                        // Read tags
                        if (versionlastloaded >= 11) {
                            for (;;) {
                                auto tag = zdis.ReadString();
                                if (tag.empty()) break;
                                currentDoc->tags[tag] = versionlastloaded >= 24 ? zdis.Read32() : g_tagcolor_default;
                            }
                        }

                        // Set initial selection
                        if (root->grid) {
                            currentDoc->selected = Selection(root->grid, 0, 0, xs, ys);
                        }

                        return nullptr; // Success
                    }

                    default:
                        return L"Corrupt block header.";
                }
            }
        }

        const wxChar *Open(const wxString &filename) { return L""; }
        void TabChange(Document *doc) {}
        void RememberOpenFiles() {}
        wxString TmpName(const wxString &f) { return f + ".tmp"; }
        const wxChar *LoadDB(const wxString &filename, bool fromreload = false) { return L""; }
    };
};

wasm_treesheets::System* wasm_treesheets::sys = nullptr;

// Helper to avoid link error in ts_menu_builder.h if it uses global functions not in struct
// But CreateMenus is member function.
// However, GetFilesFromUser etc are global functions used in CreateMenus/Action handlers.
// CreateMenus does NOT call GetFilesFromUser. Action handlers do.
// ts_menu_builder.h only contains CreateMenus and MyAppend.
// Wait, ts_menu_builder.h contains Action handlers?
// No, TSFrame::CreateMenus calls `MyAppend` which binds `Action(A_*)`.
// The handlers are in `TSFrame::Action`.
// `ts_menu_builder.h` does NOT contain `TSFrame::Action`.
// It only builds the menu.
// So I don't need `GetFilesFromUser` stub for `CreateMenus`.

// Forward declaration for render
static void RenderDocument();

void Iterate() {
    if (g_needsRedraw) {
        RenderDocument();
        g_needsRedraw = false;
    }
}

// Render the current document/demo content
static void RenderDocument() {
    TSWebGraphics g;

    // If we have a loaded document, render it
    if (wasm_treesheets::sys && wasm_treesheets::sys->currentDoc &&
        wasm_treesheets::sys->currentDoc->root) {
        wasm_treesheets::sys->currentDoc->Render(g);
        return;
    }

    // Otherwise render the welcome screen
    // Clear background
    g.SetBrushColor(0xFFFFFF);  // White
    g.SetPenColor(0xFFFFFF);
    g.DrawRectangle(0, 0, g_canvasWidth, g_canvasHeight);

    // Draw header bar
    g.SetBrushColor(0xE0E0E0);  // Light gray
    g.SetPenColor(0xCCCCCC);
    g.DrawRectangle(0, 0, g_canvasWidth, 40);

    // Draw title
    g.SetFont(16, 0);
    g.SetTextForeground(0x333333);
    g.DrawText("TreeSheets Web", 20, 10);

    // Draw welcome content
    g.SetBrushColor(0xF8F8F8);
    g.SetPenColor(0xDDDDDD);
    g.DrawRoundedRectangle(40, 80, g_canvasWidth - 80, 200, 10);

    g.SetFont(14, STYLE_BOLD);
    g.SetTextForeground(0x333333);
    g.DrawText("Welcome to TreeSheets Web!", 60, 100);

    g.SetFont(12, 0);
    g.SetTextForeground(0x666666);
    g.DrawText("A hierarchical spreadsheet that runs in your browser.", 60, 130);

    g.SetFont(11, 0);
    g.DrawText("To get started:", 60, 170);
    g.DrawText("  - Use File > Open to load a .cts file", 60, 190);
    g.DrawText("  - Or use File > New to create a new document", 60, 210);
    g.DrawText("  - Menus and toolbar are functional", 60, 230);

    // Draw canvas size info
    g.SetFont(10, 0);
    g.SetTextForeground(0x999999);
    wxString sizeInfo = wxString::Format("Canvas: %dx%d", g_canvasWidth, g_canvasHeight);
    g.DrawText(sizeInfo.c_str(), 20, g_canvasHeight - 20);
}

extern "C" {
    void WASM_FileLoaded(const char* filename, const uint8_t* data, int size) {
        std::cout << "File Loaded: " << filename << " (" << size << " bytes)" << std::endl;

        if (!wasm_treesheets::sys) {
            std::cout << "Error: System not initialized" << std::endl;
            return;
        }

        wxMemoryInputStream mis(data, size);
        const wxChar* error = wasm_treesheets::sys->LoadDBFromStream(mis, filename);

        if (error) {
            std::cout << "Error loading file: ";
            // Convert wchar_t to char for cout
            const wchar_t* werr = error;
            while (*werr) {
                std::cout << (char)*werr;
                werr++;
            }
            std::cout << std::endl;
            JS_ShowMessage("Error", "Failed to load file. It may be corrupted or an unsupported version.");
        } else {
            std::cout << "File loaded successfully!" << std::endl;
            // Reset scroll position
            g_scrollX = 0;
            g_scrollY = 0;
        }

        g_needsRedraw = true;
    }

    void WASM_Mouse(int type, int x, int y, int modifiers) {
        // type: 0=move, 1=down, 2=up, 3=wheel, 4=wheel (delta in y)
        g_mouseX = x;
        g_mouseY = y;

        auto& doc = wasm_treesheets::sys->currentDoc;

        switch (type) {
            case 0: // Move
                break;

            case 1: // Mouse down
                if (doc && doc->root) {
                    // Could implement cell selection here
                    g_needsRedraw = true;
                }
                break;

            case 2: // Mouse up
                break;

            case 3: // Wheel (y contains delta)
            case 4:
                if (doc && doc->root) {
                    // Scroll the document
                    int delta = y * 30; // y is wheel delta
                    if (modifiers & 1) { // Ctrl - horizontal scroll
                        doc->scrollx += delta;
                        if (doc->scrollx < 0) doc->scrollx = 0;
                    } else {
                        doc->scrolly += delta;
                        if (doc->scrolly < 0) doc->scrolly = 0;
                    }
                    g_needsRedraw = true;
                }
                break;
        }
    }

    void WASM_Key(int type, int key, int modifiers) {
        // type: 0=down, 1=up
        // modifiers: bit0=ctrl, bit1=shift, bit2=alt
        if (type == 0) {
            g_needsRedraw = true;
        }
    }

    void WASM_Resize(int w, int h) {
        g_canvasWidth = w;
        g_canvasHeight = h;
        g_needsRedraw = true;
        std::cout << "Resized to: " << w << "x" << h << std::endl;
    }

    void WASM_Action(int id) {
        std::cout << "Action triggered: " << id << std::endl;

        auto& sys = wasm_treesheets::sys;
        auto& doc = sys->currentDoc;

        // Handle actions
        switch (id) {
            case wxID_NEW: {
                std::cout << "New document requested" << std::endl;
                // Create a new document with a single cell
                sys->currentDoc = std::make_unique<wasm_treesheets::Document>();
                auto root = new wasm_treesheets::Cell(nullptr, nullptr, CT_DATA);
                root->cellcolor = 0xFFFFFF;
                root->textcolor = 0x000000;

                // Create a 3x3 grid
                auto grid = new wasm_treesheets::Grid(3, 3);
                grid->cell = root;
                root->grid = grid;

                // Add some default text
                grid->C(0, 0)->text.t = "New Document";

                sys->currentDoc->root = root;
                sys->currentDoc->filename = "untitled.cts";
                sys->currentDoc->selected = wasm_treesheets::Selection(grid, 0, 0, 1, 1);
                g_scrollX = 0;
                g_scrollY = 0;
                break;
            }

            case wxID_OPEN:
                std::cout << "Open file requested" << std::endl;
                JS_TriggerUpload();
                break;

            case wxID_SAVE:
                std::cout << "Save requested" << std::endl;
                if (doc && doc->root) {
                    JS_ShowMessage("Save", "Save functionality coming soon.\nUse browser's save feature for now.");
                }
                break;

            case wxID_ABOUT:
                JS_ShowMessage("About TreeSheets",
                    "TreeSheets Web Port (Proof of Concept)\n\n"
                    "A hierarchical spreadsheet application.\n"
                    "https://strlen.com/treesheets/");
                break;

            case wxID_EXIT:
                std::cout << "Exit requested (ignored in web)" << std::endl;
                break;

            case A_ZOOMIN:
                if (doc) {
                    g_deftextsize = std::min(g_deftextsize + 1, 40);
                }
                break;

            case A_ZOOMOUT:
                if (doc) {
                    g_deftextsize = std::max(g_deftextsize - 1, 6);
                }
                break;

            default:
                // For other actions, just log them
                std::cout << "Unhandled action: " << id << std::endl;
                break;
        }

        g_needsRedraw = true;
    }
}

int main() {
    wasm_treesheets::sys = new wasm_treesheets::System(false);
    wasm_treesheets::sys->os.reset(new TSWebOS());
    wasm_treesheets::sys->dialogs.reset(new TSDialogsWeb());

    // Create Menus
    wasm_treesheets::sys->frame->CreateMenus(false);

    // Create Toolbar
    wasm_treesheets::sys->frame->ConstructToolBar();

    // Test Layout
    TSWebGraphics g;
    auto c = new wasm_treesheets::Cell();
    c->text.t = "WASM Ready";
    wasm_treesheets::Document doc;
    doc.currentdrawroot = c;
    c->Layout(&doc, g, 0, 80, false);
    std::cout << "Cell calculated width: " << c->sx << std::endl;

    // Init JS hooks
    JS_InitInput();

    // Start Loop
    emscripten_set_main_loop(Iterate, 0, 1);

    return 0;
}
