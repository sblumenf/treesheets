#pragma once

#include "ts_menu_interface.h"
#include <wx/menu.h>

struct TSWxMenu : public TSMenu {
    wxMenu* menu;

    TSWxMenu() : menu(new wxMenu()) {}
    TSWxMenu(wxMenu* m) : menu(m) {} // For wrapping existing menus like filehistory

    wxMenu* GetWxMenu() { return menu; }

    virtual void Append(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        menu->Append(id, text, help);
    }
    virtual void AppendSeparator() override {
        menu->AppendSeparator();
    }
    virtual void AppendSubMenu(TSMenu* submenu, const wxString& text, const wxString& help = wxEmptyString) override {
        TSWxMenu* wxSub = dynamic_cast<TSWxMenu*>(submenu);
        if (wxSub) {
            menu->AppendSubMenu(wxSub->menu, text, help);
        }
    }
    virtual void AppendCheckItem(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        menu->AppendCheckItem(id, text, help);
    }
    virtual void AppendRadioItem(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        menu->AppendRadioItem(id, text, help);
    }
    virtual void Check(int id, bool check) override {
        menu->Check(id, check);
    }
};

struct TSWxMenuBar : public TSMenuBar {
    wxMenuBar* menubar;

    TSWxMenuBar() : menubar(new wxMenuBar()) {}

    wxMenuBar* GetWxMenuBar() { return menubar; }

    virtual void Append(TSMenu* menu, const wxString& title) override {
        TSWxMenu* wxMenu = dynamic_cast<TSWxMenu*>(menu);
        if (wxMenu) {
            menubar->Append(wxMenu->menu, title);
        }
    }
};
