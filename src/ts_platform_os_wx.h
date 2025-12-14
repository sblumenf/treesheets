#pragma once
#include "ts_platform_os.h"
#include <wx/utils.h>
#include <wx/file.h>
#include <wx/clipbrd.h>

struct TSDesktopOS : public TSPlatformOS {
    bool WriteFile(const wxString& filename, const std::vector<uint8_t>& data) override {
        wxFile file;
        if (!file.Create(filename, true)) return false;
        return file.Write(data.data(), data.size()) == data.size();
    }

    std::vector<uint8_t> ReadFile(const wxString& filename) override {
        wxFile file;
        if (!file.Open(filename)) return {};
        wxFileOffset len = file.Length();
        std::vector<uint8_t> data(len);
        if (file.Read(data.data(), len) != len) return {};
        return data;
    }

    void LaunchBrowser(const wxString& url) override {
        wxLaunchDefaultBrowser(url);
    }

    void SetClipboardText(const wxString& text) override {
        if (wxTheClipboard->Open()) {
            wxTheClipboard->SetData(new wxTextDataObject(text));
            wxTheClipboard->Close();
        }
    }

    wxString GetClipboardText() override {
        wxString text;
        if (wxTheClipboard->Open()) {
            if (wxTheClipboard->IsSupported(wxDF_TEXT)) {
                wxTextDataObject data;
                wxTheClipboard->GetData(data);
                text = data.GetText();
            }
            wxTheClipboard->Close();
        }
        return text;
    }
};
