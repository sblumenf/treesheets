#include "wx_shim.h"
#include "../tools.h"
#include "../ts_graphics.h"
#include "ts_graphics_wasm.h"
#include "ts_graphics_web.h"
#include "../ts_platform_os.h"
#include "ts_platform_os_web.h"
#include "../ts_constants.h"
#include "ts_menu_web.h"
#include "../ts_dialog_web.h" // Added
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

struct wxToolBar {
    void SetOwnBackgroundColour(const wxColour&) {}
    void AddTool(int id, const wxString& name, const wxBitmap&, const wxString&, int) {
        std::cout << "Toolbar AddTool: " << name << std::endl;
    }
    void AddControl(void*) {
        std::cout << "Toolbar AddControl" << std::endl;
    }
    void AddSeparator() {}
    void Realize() {}
    void Show(bool) {}
};
const int wxBORDER_NONE = 0;
const int wxTB_HORIZONTAL = 0;
const int wxTB_FLAT = 0;
const int wxTB_NODIVIDER = 0;
const int wxITEM_NORMAL = 0;

struct wxTextCtrl {
    wxTextCtrl(void* p, int id, const wxString& s, const wxPoint& pos, const wxSize& sz, int style = 0) {}
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
    ColorDropdown(void* p, int id, int val) {}
    void ShowPopup() {}
};
struct ImageDropdown {
    ImageDropdown(void* p, const wxString& path) {}
    std::vector<wxString> filenames;
    void ShowPopup() {}
};
struct wxStaticText {
    wxStaticText(void* p, int id, const wxString& label) {}
};

struct wxBitmapBundle {
    static wxBitmap FromSVGFile(const wxString& f, const wxSize& s) { return wxBitmap(); }
};

struct wxAcceleratorEntry {
    void Set(int, int, int) {}
};
struct wxAcceleratorTable {
    wxAcceleratorTable(int, wxAcceleratorEntry*) {}
};
const int wxACCEL_SHIFT = 0;
const int wxACCEL_CTRL = 0;

struct wxSystemSettings {
    struct Appearance { bool IsDark() { return false; } };
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
        std::unique_ptr<TSDialogs> dialogs; // Added

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

        System(bool portable) {
            frame->app = new TSApp(); // minimal app
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

void Iterate() {
    // std::cout << "Loop Frame" << std::endl;
}

extern "C" {
    void WASM_FileLoaded(const char* filename, const uint8_t* data, int size) {
        std::cout << "File Loaded Callback: " << filename << " (" << size << " bytes)" << std::endl;
        wxMemoryInputStream mis(data, size);
        // In real impl: sys->LoadDBFromStream(mis, ...);
    }

    void WASM_Mouse(int type, int x, int y, int modifiers) {
        std::cout << "Mouse: " << type << " at " << x << "," << y << std::endl;
    }

    void WASM_Key(int type, int key, int modifiers) {
        std::cout << "Key: " << type << " " << key << std::endl;
    }

    void WASM_Resize(int w, int h) {
        std::cout << "Resize: " << w << "x" << h << std::endl;
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
