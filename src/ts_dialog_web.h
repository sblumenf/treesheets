#include "../ts_dialog_interface.h"

struct TSDialogsWeb : public TSDialogs {
    void ShowMessage(const wxString& msg, const wxString& title) override {
        std::cout << "ShowMessage: " << title << ": " << msg << std::endl;
    }
    int ThreeChoice(const wxString& title, const wxString& msg, const wxString& ch1, const wxString& ch2, const wxString& ch3) override {
        std::cout << "ThreeChoice: " << title << ": " << msg << " [" << ch1 << "/" << ch2 << "/" << ch3 << "]" << std::endl;
        return 0;
    }
    bool DateTimeRange(wxDateTime& begin, wxDateTime& end) override {
        std::cout << "DateTimeRange requested" << std::endl;
        return false;
    }
    wxString AskText(const wxString& msg, const wxString& title, const wxString& defaultVal) override {
        std::cout << "AskText: " << title << ": " << msg << " (default: " << defaultVal << ")" << std::endl;
        return defaultVal;
    }
    double AskNumber(const wxString& msg, const wxString& title, double defaultVal, double min, double max) override {
        std::cout << "AskNumber: " << title << ": " << msg << " (val: " << defaultVal << ")" << std::endl;
        return defaultVal;
    }
    void GetFilesFromUser(wxArrayString& filenames, const wxString& title, const wxString& filter) override {
        std::cout << "GetFilesFromUser: " << title << std::endl;
    }
    bool SelectFont(wxString& fontName, int& fontSize) override {
        std::cout << "SelectFont requested" << std::endl;
        return false;
    }
    uint PickColor(uint defaultColor) override {
        std::cout << "PickColor requested" << std::endl;
        return defaultColor;
    }
    wxString FileSelector(const wxString& message, const wxString& default_path, const wxString& default_filename, const wxString& default_extension, const wxString& wildcard, int flags) override {
        std::cout << "FileSelector: " << message << std::endl;
        return "";
    }
    int SingleChoice(const wxString& title, const wxString& msg, const wxArrayString& choices) override {
        std::cout << "SingleChoice: " << title << ": " << msg << std::endl;
        return -1;
    }
    void ShowAbout(const wxString& title, const wxString& version, const wxString& desc) override {
        std::cout << "ShowAbout: " << title << std::endl;
    }
    void PageSetup(Document* doc) override {
        std::cout << "PageSetup" << std::endl;
    }
    void Print(Document* doc) override {
        std::cout << "Print" << std::endl;
    }
    void PrintPreview(Document* doc) override {
        std::cout << "PrintPreview" << std::endl;
    }
};
