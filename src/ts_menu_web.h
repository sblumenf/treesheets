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
             // Link submenu. JS side needs to handle submenu type (4)
             // Or maybe flattening?
             // Simple JS menu implementation supports flat list.
             // Nested is harder.
             // I'll skip nested for now or treat as item that opens another menu?
             // Standard wxMenu is nested.
             // I'll append as type 4?
             // But JS logic needs to know the submenu ID.
             // I'll encode submenu ID in itemId? No itemId is 0 for submenu usually?
             // Or passed as help?
             // Let's just log it for now as "Submenu not fully supported in JS basic implementation".
             // Or append items of submenu to this menu? (Flattening).
             // Flattening is easier for Phase 4 proof of concept.
             // "Submenu: Title" -> items.
             // But TSFrame uses submenus extensively.

             // I'll append a separator with title "--- Submenu: Title ---"
             JS_Menu_Append(id, 0, (wxString("--- ") + text + " ---").c_str(), help.c_str(), 3, false);
             // Then append items from submenu?
             // But submenu items are already added to submenu.id.
             // I need to copy them?
             // Or JS side handles hierarchy.
             // Let's stick to flat or just log.
             // Reverting to console log for submenu to avoid breaking JS.
             std::cout << "Submenu appended: " << text << " (ID: " << webSub->id << ")" << std::endl;
        }
    }
    virtual void AppendCheckItem(int itemId, const wxString& text, const wxString& help = wxEmptyString) override {
        JS_Menu_Append(id, itemId, text.c_str(), help.c_str(), 1, false);
    }
    virtual void AppendRadioItem(int itemId, const wxString& text, const wxString& help = wxEmptyString) override {
        JS_Menu_Append(id, itemId, text.c_str(), help.c_str(), 2, false);
    }
    virtual void Check(int itemId, bool check) override {
        // JS_Menu_Check(id, itemId, check); // Todo
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
