#pragma once

#include <wx/string.h>
#include <vector>
#include <cstdint>

class TSPlatformOS {
public:
    virtual ~TSPlatformOS() {}

    // File I/O
    // Desktop: Writes to disk. WASM: Triggers download.
    virtual bool WriteFile(const wxString& filename, const std::vector<uint8_t>& data) = 0;

    // Desktop: Reads from disk. WASM: Not used directly (browser pushes data).
    // But we might need it for config loading?
    virtual std::vector<uint8_t> ReadFile(const wxString& filename) = 0;

    // System interaction
    virtual void LaunchBrowser(const wxString& url) = 0;

    // Clipboard
    virtual void SetClipboardText(const wxString& text) = 0;
    virtual wxString GetClipboardText() = 0;
};
