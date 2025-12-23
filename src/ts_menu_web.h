#pragma once
#include "ts_menu_interface.h"
#include "wasm/web_interface.h"
#include <iostream>
#include <vector>

struct TSWebMenu : public TSMenu {
    inline static int nextId = 1;
    int id;

    TSWebMenu() : id(nextId++) {
        JS_Menu_Create(id, ""); // Title set when added to bar or submenu?
        // Actually wxMenu doesn't have title until appended?
        // JS_Menu_Create(id, title) expects title.
        // We can update title later? No API for it.
        // wxMenu constructor takes title.
        // But TSFrame uses `new TSPlatformMenu()`. No args.
    }

    virtual void Append(int itemId, const wxString& text, const wxString& help = wxEmptyString) override {
        JS_Menu_Append(id, itemId, text.c_str(), help.c_str(), 0, false);
    }
    virtual void AppendSeparator() override {
        JS_Menu_Append(id, 0, "", "", 3, false);
    }
    virtual void AppendSubMenu(TSMenu* submenu, const wxString& text, const wxString& help = wxEmptyString) override {
        TSWebMenu* webSub = dynamic_cast<TSWebMenu*>(submenu);
        if(webSub) {
            JS_Menu_AppendSubMenu(id, webSub->id, text.c_str(), help.c_str());
        }
    }
    virtual void AppendCheckItem(int itemId, const wxString& text, const wxString& help = wxEmptyString) override {
        JS_Menu_Append(id, itemId, text.c_str(), help.c_str(), 1, false);
    }
    virtual void AppendRadioItem(int itemId, const wxString& text, const wxString& help = wxEmptyString) override {
        JS_Menu_Append(id, itemId, text.c_str(), help.c_str(), 2, false);
    }
    virtual void Check(int itemId, bool check) override {
        JS_Menu_Check(id, itemId, check);
    }
};

struct TSWebMenuBar : public TSMenuBar {
    virtual void Append(TSMenu* menu, const wxString& title) override {
        TSWebMenu* webMenu = dynamic_cast<TSWebMenu*>(menu);
        if(webMenu) {
            // Update title of menu? JS_Menu_Create used empty string.
            // JS_MenuBar_Append uses title.
            JS_MenuBar_Append(webMenu->id, title.c_str());
        }
    }
};
