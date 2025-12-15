#pragma once

// #include <wx/defs.h> // Relies on stdafx.h or wx_shim.h being included first
// Or just basic types.

#ifndef __WXWIDGETS__
// Ensure types are defined if not using wxWidgets headers
// But main.cpp (Desktop) defines them via stdafx.h
// And wasm_main.cpp (Web) defines them via wx_shim.h
// So we assume they exist.
// If standalone compilation fails, we can add guards.
#endif

// Constants used across the application
static const auto TS_VERSION = 24;
static const auto g_grid_margin = 1;
static const auto g_cell_margin = 2;
static const auto g_margin_extra = 2;
static const auto g_usergridouterspacing_default = 3;
static const auto g_line_width = 1;
static const auto g_selmargin = 2;
static const auto g_scrollratecursor = 240;
static const auto g_scrollratewheel = 2;
static const auto g_max_launches = 20;
static const auto g_deftextsize_default = 12;
static const auto g_mintextsize_delta = 8;
static const auto g_maxtextsize_delta = 32;
static const auto BLINK_TIME = 400;
static const auto CUSTOMCOLORIDX = 0;
static const auto TS_SELECTION_MASK = 0x80;
static const auto g_bordercolor_default = 0xA0A0A0;
static const auto g_cellcolor_default = 0xFFFFFFU;
static const auto g_textcolor_default = 0x000000U;
static const auto g_tagcolor_default = 0xFF0000;

// Mutable global state
extern int g_deftextsize;

static int g_mintextsize() { return g_deftextsize - g_mintextsize_delta; }
static int g_maxtextsize() { return g_deftextsize + g_maxtextsize_delta; }

enum { TS_TEXT = 0, TS_GRID = 1, TS_BOTH = 2, TS_NEITHER = 3 };

enum {
    A_SAVEALL = 500,
    A_COLLAPSE,
    A_NEWGRID,
    A_CLRVIEW,
    A_MARKDATA,
    A_MARKVIEWH,
    A_MARKVIEWV,
    A_MARKCODE,
    A_IMAGE,
    A_EXPIMAGE,
    A_EXPXML,
    A_EXPHTMLT,
    A_EXPHTMLTI,
    A_EXPHTMLTE,
    A_EXPHTMLO,
    A_EXPHTMLB,
    A_EXPTEXT,
    A_ZOOMIN,
    A_ZOOMOUT,
    A_TRANSPOSE,
    A_DELETE,
    A_BACKSPACE,
    A_DELETE_WORD,
    A_BACKSPACE_WORD,
    A_LEFT,
    A_RIGHT,
    A_UP,
    A_DOWN,
    A_MLEFT,
    A_MRIGHT,
    A_MUP,
    A_MDOWN,
    A_SLEFT,
    A_SRIGHT,
    A_SUP,
    A_SDOWN,
    A_ALEFT,
    A_ARIGHT,
    A_AUP,
    A_ADOWN,
    A_SCLEFT,
    A_SCRIGHT,
    A_SCUP,
    A_SCDOWN,
    A_IMPXML,
    A_IMPXMLA,
    A_IMPTXTI,
    A_IMPTXTC,
    A_IMPTXTS,
    A_IMPTXTT,
    A_TUTORIALWEBPAGE,
    A_SCRIPTREFERENCE,
    A_MARKVARD,
    A_MARKVARU,
    A_SHOWSBAR,
    A_SHOWTBAR,
    A_LEFTTABS,
    A_TRADSCROLL,
    A_HOME,
    A_END,
    A_CHOME,
    A_CEND,
    A_PAGESETUP,
    A_PRINTSCALE,
    A_NEXT,
    A_PREV,
    A_TT,
    A_SEARCH,
    A_CASESENSITIVESEARCH,
    A_CLEARSEARCH,
    A_CLEARREPLACE,
    A_REPLACE,
    A_REPLACEONCE,
    A_REPLACEONCEJ,
    A_REPLACEALL,
    A_CANCELEDIT,
    A_BROWSE,
    A_ENTERCELL,
    A_ENTERCELL_JUMPTOSTART,
    A_ENTERCELL_JUMPTOEND,
    A_PROGRESSCELL,  // see
                     // https://github.com/aardappel/treesheets/issues/139#issuecomment-544167524
    A_CELLCOLOR,
    A_TEXTCOLOR,
    A_BORDCOLOR,
    A_INCSIZE,
    A_DECSIZE,
    A_INCWIDTH,
    A_DECWIDTH,
    A_ENTERGRID,
    A_LINK,
    A_LINKREV,
    A_LINKIMG,
    A_LINKIMGREV,
    A_SEARCHNEXT,
    A_SEARCHPREV,
    A_CUSTCOL,
    A_COLCELL,
    A_SORT,
    A_MAKEBAKS,
    A_TOTRAY,
    A_AUTOSAVE,
    A_FULLSCREEN,
    A_SCALED,
    A_SCOLS,
    A_SROWS,
    A_SHOME,
    A_SEND,
    A_BORD0,
    A_BORD1,
    A_BORD2,
    A_BORD3,
    A_BORD4,
    A_BORD5,
    A_HSWAP,
    A_TEXTGRID,
    A_TAGADD,
    A_TAGREMOVE,
    A_WRAP,
    A_HIFY,
    A_FLATTEN,
    A_BROWSEF,
    A_ROUND0,
    A_ROUND1,
    A_ROUND2,
    A_ROUND3,
    A_ROUND4,
    A_ROUND5,
    A_ROUND6,
    A_FILTER5,
    A_FILTER10,
    A_FILTER20,
    A_FILTER50,
    A_FILTERM,
    A_FILTERL,
    A_FILTERS,
    A_FILTEROFF,
    A_FILTERBYCELLBG,
    A_FILTERMATCHNEXT,
    A_FILTERRANGE,
    A_FILTERDIALOG,
    A_FASTRENDER,
    A_INVERTRENDER,
    A_EXPCSV,
    A_PASTESTYLE,
    A_PREVFILE,
    A_NEXTFILE,
    A_IMAGER,
    A_INCWIDTHNH,
    A_DECWIDTHNH,
    A_ZOOMSCR,
    A_V_GS,
    A_V_BS,
    A_V_LS,
    A_H_GS,
    A_H_BS,
    A_H_LS,
    A_GS,
    A_BS,
    A_LS,
    A_RESETSIZE,
    A_RESETWIDTH,
    A_RESETSTYLE,
    A_RESETCOLOR,
    A_LASTCELLCOLOR,
    A_LASTTEXTCOLOR,
    A_LASTBORDCOLOR,
    A_LASTIMAGE,
    A_OPENCELLCOLOR,
    A_OPENTEXTCOLOR,
    A_OPENBORDCOLOR,
    A_OPENIMGDROPDOWN,
    A_DDIMAGE,
    A_MINCLOSE,
    A_SINGLETRAY,
    A_CENTERED,
    A_SORTD,
    A_FOLD,
    A_FOLDALL,
    A_UNFOLDALL,
    A_IMAGESCP,
    A_IMAGESCW,
    A_IMAGESCF,
    A_IMAGESCN,
    A_IMAGESVA,
    A_SAVE_AS_JPEG,
    A_SAVE_AS_PNG,
    A_HELP_OP_REF,
    A_FSWATCH,
    A_DEFBGCOL,
    A_DEFCURCOL,
    A_THINSELC,
    A_COPYCT,
    A_COPYBM,
    A_COPYWI,
    A_MINISIZE,
    A_CUSTKEY,
    A_SETLANG,
    A_AUTOEXPORT_HTML_NONE,
    A_AUTOEXPORT_HTML_WITH_IMAGES,
    A_AUTOEXPORT_HTML_WITHOUT_IMAGES,
    A_DRAGANDDROP,
    A_DEFAULTMAXCOLWIDTH,
    A_ADDSCRIPT,
    A_DETSCRIPT,
    A_SET_FIXED_FONT,
    A_NOP,
    A_TAGSET = 1000,  // and all values from here on
    A_SCRIPT = 2000,  // and all values from here on
    A_MAXACTION = 3000
};

enum {
    STYLE_BOLD = 1,
    STYLE_ITALIC = 2,
    STYLE_FIXED = 4,
    STYLE_UNDERLINE = 8,
    STYLE_STRIKETHRU = 16
};

enum { TEXT_SPACE = 3, TEXT_SEP = 2, TEXT_CHAR = 1 };

/* The evaluation types for a cell.
CT_DATA: "Data"
CT_CODE: "Operation"
CT_VARD: "Variable Assign"
CT_VARU: "Variable Read"
CT_VIEWH: "Horizontal View"
CT_VIEWV: "Vertical View"
*/
enum { CT_DATA = 0, CT_CODE, CT_VARD, CT_VIEWH, CT_VARU, CT_VIEWV };

/* The drawstyles for a cell:

*/
enum { DS_GRID, DS_BLOBSHIER, DS_BLOBLINE };
