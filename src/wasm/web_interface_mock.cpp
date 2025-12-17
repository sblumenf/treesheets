#include "web_interface.h"
#include <iostream>
#include <cstring>
#include <cstdlib>

extern "C" {
    void JS_DrawRectangle(int x, int y, int w, int h) { std::cout << "DrawRect " << x << "," << y << "\n"; }
    void JS_DrawRoundedRectangle(int x, int y, int w, int h, int radius) {}
    void JS_DrawLine(int x1, int y1, int x2, int y2) {}
    void JS_DrawText(const char* str, int x, int y) {}
    void JS_DrawBitmap(int imageIndex, int x, int y) {}
    int JS_GetCharHeight() { return 10; }
    int JS_GetTextWidth(const char* str) { return 0; }
    int JS_GetTextHeight(const char* str) { return 10; }
    void JS_SetBrushColor(unsigned int color) {}
    void JS_SetPenColor(unsigned int color) {}
    void JS_SetTextForeground(unsigned int color) {}
    void JS_SetTextBackground(unsigned int color) {}
    void JS_SetFont(int size, int stylebits) {}
    void JS_SetPen(int penType) {}
    void JS_SetBrush(int brushType) {}
    void JS_InitInput() {}
    void JS_TriggerUpload() {}
    void JS_DownloadFile(const char* filename, const uint8_t* data, int size) {}
    void JS_LaunchBrowser(const char* url) {}
    void JS_SetClipboardText(const char* text) {}

    // Menus
    void JS_Menu_Create(int id, const char* title) { std::cout << "JS_Menu_Create " << id << " " << title << "\n"; }
    void JS_Menu_Append(int parentId, int id, const char* text, const char* help, int type, bool checked) {
        std::cout << "JS_Menu_Append " << parentId << " " << text << "\n";
    }
    void JS_MenuBar_Append(int menuId, const char* title) { std::cout << "JS_MenuBar_Append " << menuId << " " << title << "\n"; }

    // Dialogs
    void JS_ShowMessage(const char* title, const char* msg) {}
    char* JS_AskText(const char* title, const char* msg, const char* def) {
        return strdup(def);
    }
    double JS_AskNumber(const char* title, const char* msg, double def, double min, double max) { return def; }
    int JS_SingleChoice(const char* title, const char* msg, const char* choices_json) { return 0; }

    // Toolbar
    void JS_Toolbar_Create() { std::cout << "JS_Toolbar_Create\n"; }
    void JS_Toolbar_AddTool(int id, const char* label, const char* iconpath) { std::cout << "JS_Toolbar_AddTool " << label << "\n"; }
    void JS_Toolbar_AddSeparator() { std::cout << "JS_Toolbar_AddSeparator\n"; }
    void JS_Toolbar_AddInput(int id, int width, const char* defaultVal) { std::cout << "JS_Toolbar_AddInput " << id << "\n"; }
    void JS_Toolbar_AddLabel(const char* label) { std::cout << "JS_Toolbar_AddLabel " << label << "\n"; }
    void JS_Toolbar_AddDropdown(int id, int width, const char* choices_json) { std::cout << "JS_Toolbar_AddDropdown " << id << "\n"; }
}
