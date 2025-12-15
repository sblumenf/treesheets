#pragma once

#include <wx/string.h>

struct TSMenu {
    virtual ~TSMenu() {}
    virtual void Append(int id, const wxString& text, const wxString& help = wxEmptyString) = 0;
    virtual void AppendSeparator() = 0;
    virtual void AppendSubMenu(TSMenu* submenu, const wxString& text, const wxString& help = wxEmptyString) = 0;
    virtual void AppendCheckItem(int id, const wxString& text, const wxString& help = wxEmptyString) = 0;
    virtual void AppendRadioItem(int id, const wxString& text, const wxString& help = wxEmptyString) = 0;
    virtual void Check(int id, bool check) = 0;
};

struct TSMenuBar {
    virtual ~TSMenuBar() {}
    virtual void Append(TSMenu* menu, const wxString& title) = 0;
};
