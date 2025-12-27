#pragma once
#include "../ts_platform_os.h"
#include "web_interface.h"

struct TSWebOS : public TSPlatformOS {
    bool WriteFile(const wxString& filename, const std::vector<uint8_t>& data) override {
        JS_DownloadFile(filename.c_str(), data.data(), data.size());
        return true;
    }

    std::vector<uint8_t> ReadFile(const wxString& filename) override {
        uint8_t* ptr = JS_ReadFile(filename.c_str());
        if (!ptr) return {};

        int size = JS_GetLastFileSize();
        std::vector<uint8_t> data(ptr, ptr + size);
        free(ptr);
        return data;
    }

    void LaunchBrowser(const wxString& url) override {
        JS_LaunchBrowser(url.c_str());
    }

    void SetClipboardText(const wxString& text) override {
        JS_SetClipboardText(text.c_str());
    }

    wxString GetClipboardText() override {
        char* ptr = JS_GetClipboardText();
        if (!ptr) return "";
        wxString result(ptr);
        free(ptr);
        return result;
    }
};
