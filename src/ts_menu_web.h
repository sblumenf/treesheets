#pragma once

#include "ts_menu_interface.h"
#include <iostream>
#include <vector>

struct TSWebMenu : public TSMenu {
    struct Item {
        int id;
        wxString text;
        wxString help;
        bool is_separator = false;
        bool is_check = false;
        bool is_radio = false;
        bool checked = false;
        TSWebMenu* submenu = nullptr;
    };

    std::vector<Item> items;

    virtual void Append(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        items.push_back({id, text, help});
        // std::cout << "Menu Append: " << text << std::endl;
    }
    virtual void AppendSeparator() override {
        Item i; i.is_separator = true;
        items.push_back(i);
    }
    virtual void AppendSubMenu(TSMenu* submenu, const wxString& text, const wxString& help = wxEmptyString) override {
        Item i;
        i.text = text;
        i.help = help;
        i.submenu = dynamic_cast<TSWebMenu*>(submenu);
        items.push_back(i);
    }
    virtual void AppendCheckItem(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        Item i; i.id = id; i.text = text; i.help = help; i.is_check = true;
        items.push_back(i);
    }
    virtual void AppendRadioItem(int id, const wxString& text, const wxString& help = wxEmptyString) override {
        Item i; i.id = id; i.text = text; i.help = help; i.is_radio = true;
        items.push_back(i);
    }
    virtual void Check(int id, bool check) override {
        for (auto& item : items) {
            if (item.id == id) {
                item.checked = check;
                return;
            }
        }
    }
};

struct TSWebMenuBar : public TSMenuBar {
    struct MenuEntry {
        TSWebMenu* menu;
        wxString title;
    };
    std::vector<MenuEntry> menus;

    virtual void Append(TSMenu* menu, const wxString& title) override {
        menus.push_back({dynamic_cast<TSWebMenu*>(menu), title});
        // std::cout << "MenuBar Append: " << title << std::endl;
    }
};
