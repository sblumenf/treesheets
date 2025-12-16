#include "stdafx.h"
#include "ts_constants.h"

// Defined in ts_constants.h
// static const auto TS_VERSION = 24;
// ...

int g_deftextsize = g_deftextsize_default;

static const std::array<uint, 42> celltextcolors = {
    0xFFFFFF,  // CUSTOM COLOR!
    0xFFFFFF, 0x000000, 0x202020, 0x404040, 0x606060, 0x808080, 0xA0A0A0, 0xC0C0C0, 0xD0D0D0,
    0xE0E0E0, 0xE8E8E8, 0x000080, 0x0000FF, 0x8080FF, 0xC0C0FF, 0xC0C0E0, 0x008000, 0x00FF00,
    0x80FF80, 0xC0FFC0, 0xC0E0C0, 0x800000, 0xFF0000, 0xFF8080, 0xFFC0C0, 0xE0C0C0, 0x800080,
    0xFF00FF, 0xFF80FF, 0xFFC0FF, 0xE0C0E0, 0x008080, 0x00FFFF, 0x80FFFF, 0xC0FFFF, 0xC0E0E0,
    0x808000, 0xFFFF00, 0xFFFF80, 0xFFFFC0, 0xE0E0C0,
};

static const std::map<char, pair<wxBitmapType, wxString>> imagetypes = {
    {'I', {wxBITMAP_TYPE_PNG, "image/png"}}, {'J', {wxBITMAP_TYPE_JPEG, "image/jpeg"}}};

// script_interface.h is both used by TreeSheets and lobster-impl
// and uses data types that are already defined by lobster.

// Define these data types separately on the TreeSheets side here
// to avoid redefinitions.

struct string_view_nt {
    string_view sv;
    string_view_nt(const string &s) : sv(s) {}
    explicit string_view_nt(const char *s) : sv(s) {}
    explicit string_view_nt(string_view osv) : sv(osv) { check_null_terminated(); }
    void check_null_terminated() const { assert(!sv.data()[sv.size()]); }
    size_t size() const { return sv.size(); }
    const char *data() const { return sv.data(); }
    const char *c_str() const {
        check_null_terminated();  // Catch appends to parent buffer since construction.
        return sv.data();
    }
};

using FileLoader = int64_t (*)(string_view_nt absfilename, std::string *dest, int64_t start,
                               int64_t len);

#include "script_interface.h"

using namespace script;

wxDEFINE_EVENT(UPDATE_STATUSBAR_REQUEST, wxCommandEvent);
wxDEFINE_EVENT(SCROLLTOSELECTION_REQUEST, wxCommandEvent);

struct treesheets {
    struct TreeSheetsScriptImpl;

    struct Image;
    struct Text;
    struct Cell;
    struct Grid;
    struct Selection;
    struct Document;
    struct Evaluator;

    struct System;
    struct TSCanvas;
    struct TSFrame;
    struct TSApp;

    static System *sys;

    #include "treesheets_impl.h"

    #include "ts_graphics.h"
    #include "ts_graphics_wx.h"

    #include "ts_platform_os.h"
    #include "ts_platform_os_wx.h"

    #include "image.h"
    #include "text.h"
    #include "cell.h"
    #include "grid.h"
    #include "selection.h"
    #include "document.h"
    #include "evaluator.h"

    #include "system.h"

    #include "wxtools.h"
    #include "tscanvas.h"
    #include "tsframe.h"
    #include "ts_dialog_wx.h"
    #include "tsapp.h"
};

treesheets::System *treesheets::sys = nullptr;
treesheets::TreeSheetsScriptImpl treesheets::tssi;

IMPLEMENT_APP(treesheets::TSApp)

#include "events.h"
