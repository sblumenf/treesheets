#pragma once
#include "../ts_platform_os.h"
#include "web_interface.h"

struct TSWebOS : public TSPlatformOS {
    bool WriteFile(const wxString& filename, const std::vector<uint8_t>& data) override {
        JS_DownloadFile(filename.c_str(), data.data(), data.size());
        return true;
    }

    std::vector<uint8_t> ReadFile(const wxString& filename) override {
        return {};
    }

    void LaunchBrowser(const wxString& url) override {
        JS_LaunchBrowser(url.c_str());
    }

    void SetClipboardText(const wxString& text) override {
        JS_SetClipboardText(text.c_str());
    }

    wxString GetClipboardText() override {
        return "";
    }
};
