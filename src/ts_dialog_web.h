#include "../ts_dialog_interface.h"
#include "wasm/web_interface.h"
#include <iostream>
#include <stdlib.h> // for free

struct TSDialogsWeb : public TSDialogs {
    void ShowMessage(const wxString& msg, const wxString& title) override {
        JS_ShowMessage(title.c_str(), msg.c_str());
    }
    int ThreeChoice(const wxString& title, const wxString& msg, const wxString& ch1, const wxString& ch2, const wxString& ch3) override {
        // Fallback to simpler prompt? Or confirm?
        // Since prompt is blocking, use it.
        wxString txt = msg + L"\n1: " + ch1 + L"\n2: " + ch2 + L"\n3: " + ch3;
        double res = JS_AskNumber(title.c_str(), txt.c_str(), 1, 1, 3);
        return (int)res - 1;
    }
    bool DateTimeRange(wxDateTime& begin, wxDateTime& end) override {
        return false;
    }
    wxString AskText(const wxString& msg, const wxString& title, const wxString& defaultVal) override {
        char* res = JS_AskText(title.c_str(), msg.c_str(), defaultVal.c_str());
        wxString s(res);
        free(res);
        return s;
    }
    double AskNumber(const wxString& msg, const wxString& title, double defaultVal, double min, double max) override {
        return JS_AskNumber(title.c_str(), msg.c_str(), defaultVal, min, max);
    }
    void GetFilesFromUser(wxArrayString& filenames, const wxString& title, const wxString& filter) override {
        // Async upload not supported in blocking flow easily.
        // But we can trigger it.
        JS_TriggerUpload();
        // Return immediately. File loaded callback will handle it later.
    }
    bool SelectFont(wxString& fontName, int& fontSize) override {
        return false;
    }
    uint PickColor(uint defaultColor) override {
        // JS prompt for hex?
        char* res = JS_AskText("Pick Color", "Enter Hex Color (RRGGBB):", "FFFFFF");
        wxString s(res);
        free(res);
        // Parse hex
        // ... simple parser
        return defaultColor; // Mock
    }
    wxString FileSelector(const wxString& message, const wxString& default_path, const wxString& default_filename, const wxString& default_extension, const wxString& wildcard, int flags) override {
        // For Save: return a filename to trigger browser download?
        // JS_DownloadFile needs data.
        // System::SaveDB calls FileSelector then WriteFile.
        // On Web, WriteFile should trigger Download?
        // If WriteFile is implemented to download, then FileSelector just needs to return a name.
        char* res = JS_AskText("Save File", message.c_str(), default_filename.c_str());
        wxString s(res);
        free(res);
        return s;
    }
    int SingleChoice(const wxString& title, const wxString& msg, const wxArrayString& choices) override {
        // Need to pass array to JS. JSON?
        wxString json = "[";
        for(size_t i=0; i<choices.size(); i++) {
            json += "\"";
            json += choices[i];
            json += "\"";
            if(i < choices.size()-1) json += ",";
        }
        json += "]";
        return JS_SingleChoice(title.c_str(), msg.c_str(), json.c_str());
    }
    void ShowAbout(const wxString& title, const wxString& version, const wxString& desc) override {
        JS_ShowMessage(title.c_str(), (version + "\n" + desc).c_str());
    }
    void PageSetup(Document* doc) override {}
    void Print(Document* doc) override {}
    void PrintPreview(Document* doc) override {}
};
