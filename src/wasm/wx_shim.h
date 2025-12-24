#pragma once

#include <string>
#include <vector>
#include <iostream>
#include <algorithm>
#include <cstdint>
#include <sstream>
#include <cstdarg>
#include <cstring> // memcpy
#include <zlib.h>  // Emscripten provides zlib with -sUSE_ZLIB=1

// Basic Types
using wxUint8 = uint8_t;
using wxUint32 = uint32_t;
using wxInt64 = int64_t;
using uchar = unsigned char;
using uint = unsigned int;
using wxChar = wchar_t;

// UTF-8 conversion helpers
namespace utf8_utils {
    // Convert a single wchar_t codepoint to UTF-8 bytes, append to string
    inline void append_wchar_as_utf8(std::string& s, wchar_t wc) {
        uint32_t cp = static_cast<uint32_t>(wc);
        if (cp < 0x80) {
            s.push_back(static_cast<char>(cp));
        } else if (cp < 0x800) {
            s.push_back(static_cast<char>(0xC0 | (cp >> 6)));
            s.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
        } else if (cp < 0x10000) {
            s.push_back(static_cast<char>(0xE0 | (cp >> 12)));
            s.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
            s.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
        } else if (cp < 0x110000) {
            s.push_back(static_cast<char>(0xF0 | (cp >> 18)));
            s.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3F)));
            s.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
            s.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
        }
        // Invalid codepoints are silently ignored
    }

    // Convert wchar_t string to UTF-8 string
    inline std::string wchar_to_utf8(const wchar_t* s) {
        std::string result;
        if (s) {
            while (*s) {
                append_wchar_as_utf8(result, *s);
                s++;
            }
        }
        return result;
    }

    // Convert std::wstring to UTF-8 string
    inline std::string wstring_to_utf8(const std::wstring& s) {
        std::string result;
        for (wchar_t wc : s) {
            append_wchar_as_utf8(result, wc);
        }
        return result;
    }
}

// wxString Shim - stores UTF-8 internally
class wxString : public std::string {
public:
    wxString() : std::string() {}
    wxString(const char* s) : std::string(s ? s : "") {}
    wxString(const std::string& s) : std::string(s) {}
    wxString(const wchar_t* s) : std::string(utf8_utils::wchar_to_utf8(s)) {}

    static wxString Format(const char* fmt, ...) {
        char buf[1024];
        va_list args;
        va_start(args, fmt);
        vsnprintf(buf, sizeof(buf), fmt, args);
        va_end(args);
        return wxString(buf);
    }

    static wxString Format(const wchar_t* fmt, ...) {
        // Convert wchar_t format to UTF-8, then format
        // This is a simplified version - for full printf-style formatting
        // we'd need more complex handling
        return wxString(fmt);  // At minimum, preserve the format string
    }

    size_t Len() const { return length(); }
    size_t Length() const { return length(); }
    bool IsEmpty() const { return empty(); }
    void Clear() { std::string::clear(); }

    wxString Left(size_t n) const { return substr(0, n); }
    wxString Mid(size_t start, size_t count = npos) const {
        if(start >= length()) return "";
        return substr(start, count);
    }
    void Truncate(size_t len) { if(len < length()) resize(len); }
    wxString Trim(bool fromRight = true) {
        wxString result = *this;
        if (fromRight) {
            // Trim from right
            size_t end = result.find_last_not_of(" \t\n\r\f\v");
            if (end != std::string::npos) {
                result.erase(end + 1);
            } else {
                result.clear();
            }
        } else {
            // Trim from left
            size_t start = result.find_first_not_of(" \t\n\r\f\v");
            if (start != std::string::npos) {
                result.erase(0, start);
            } else {
                result.clear();
            }
        }
        return result;
    }

    int Find(const wxString& sub) const { return (int)find(sub); }
    int Find(char c) const { return (int)find(c); }

    void Replace(const wxString& old, const wxString& newStr) {
        size_t pos = 0;
        while ((pos = find(old, pos)) != std::string::npos) {
            replace(pos, old.length(), newStr);
            pos += newStr.length();
        }
    }

    void Remove(size_t pos, size_t count) { erase(pos, count); }

    void Append(char c, size_t count) { append(count, c); }
    void Append(const wxString& s) { append(s); }
    void Prepend(const wxString& s) { insert(0, s); }

    bool EndsWith(const wxString& suffix) const {
        if (length() < suffix.length()) return false;
        return compare(length() - suffix.length(), suffix.length(), suffix) == 0;
    }

    wxString Lower() const {
        wxString s = *this;
        std::transform(s.begin(), s.end(), s.begin(), ::tolower);
        return s;
    }

    // Conversion
    const char* wx_str() const { return c_str(); }
    const wchar_t* wc_str() const { return L""; } // Dummy
    operator const wchar_t*() const { return wc_str(); }

    std::wstring ToStdWstring() const {
        return std::wstring(begin(), end());
    }
    static wxString FromUTF8(const char* s, size_t len) { return std::string(s, len); }
    std::string utf8_string() const { return *this; }

    // Operators
    wxString& operator<<(int i) { append(std::to_string(i)); return *this; }
    wxString& operator<<(const char* s) { append(s); return *this; }

    void Append(char c) { push_back(c); }

    void Pad(size_t count, char padChar = ' ', bool right = true) {
        if (count > length()) {
            if (right) append(count - length(), padChar);
            else insert(0, count - length(), padChar);
        }
    }
    void Pad(size_t count, wchar_t padChar, bool right = true) {
        Pad(count, (char)padChar, right);
    }

    wxString& operator+=(const wchar_t* s) {
        if (s) {
            while (*s) {
                utf8_utils::append_wchar_as_utf8(*this, *s);
                s++;
            }
        }
        return *this;
    }

    using std::string::operator+=;

    wxString& operator=(const std::wstring& s) {
        assign(utf8_utils::wstring_to_utf8(s));
        return *this;
    }

    wxString& operator=(const wchar_t* s) {
        assign(utf8_utils::wchar_to_utf8(s));
        return *this;
    }

    wxString& operator=(const char* s) {
        assign(s);
        return *this;
    }

    int CmpNoCase(const wxString& s) const {
        std::string s1 = Lower();
        std::string s2 = s.Lower();
        return s1.compare(s2);
    }

    friend wxString operator+(const wxString& lhs, const char* rhs) { return std::string(lhs) + rhs; }
    friend wxString operator+(const char* lhs, const wxString& rhs) { return lhs + std::string(rhs); }

    friend wxString operator+(const wxString& lhs, const wchar_t* rhs) {
        wxString s = lhs;
        s += rhs;
        return s;
    }
    friend wxString operator+(const wchar_t* lhs, const wxString& rhs) {
        wxString s(lhs);
        s += rhs;
        return s;
    }

    friend wxString operator+(wxString&& lhs, const wchar_t* rhs) {
        lhs += rhs;
        return std::move(lhs);
    }
    friend wxString operator+(const wchar_t* lhs, wxString&& rhs) {
        wxString s(lhs);
        s += rhs;
        return s;
    }

    friend wxString operator+(const wxString& lhs, wchar_t rhs) {
        wxString s = lhs;
        utf8_utils::append_wchar_as_utf8(s, rhs);
        return s;
    }
    friend wxString operator+(wchar_t lhs, const wxString& rhs) {
        wxString s;
        utf8_utils::append_wchar_as_utf8(s, lhs);
        s += rhs;
        return s;
    }

    friend wxString operator+(const wxString& lhs, const wxString& rhs) { return std::string(lhs) + std::string(rhs); }
    friend wxString operator+(wxString&& lhs, const wxString& rhs) { return std::string(lhs) + std::string(rhs); }
    friend wxString operator+(const wxString& lhs, wxString&& rhs) { return std::string(lhs) + std::string(rhs); }
    friend wxString operator+(wxString&& lhs, wxString&& rhs) { return std::string(lhs) + std::string(rhs); }

    friend wxString operator+(wxString&& lhs, const char* rhs) { return std::string(lhs) + rhs; }
    friend wxString operator+(const char* lhs, wxString&& rhs) { return lhs + std::string(rhs); }
};

// Global empty string
const wxString wxEmptyString = "";

// wxArrayString
class wxArrayString : public std::vector<wxString> {
public:
    void Add(const wxString& s) { push_back(s); }
    size_t GetCount() const { return size(); }
};

// wxLongLong
class wxLongLong {
    long long val;
public:
    wxLongLong(long long v = 0) : val(v) {}
    long long GetValue() const { return val; }
    wxLongLong& operator--() { val--; return *this; }
    wxLongLong operator--(int) { wxLongLong temp = *this; val--; return temp; }
};

// wxDateTime
class wxDateTime {
    long long ticks;
public:
    wxDateTime() : ticks(0) {}
    wxDateTime(long long t) : ticks(t) {}
    wxDateTime(const wxLongLong& t) : ticks(t.GetValue()) {}
    static wxDateTime Now() { return wxDateTime(1000); }
    long long GetValue() const { return ticks; }
    bool IsBetween(const wxDateTime& t1, const wxDateTime& t2) const { return ticks >= t1.ticks && ticks <= t2.ticks; }
};

class wxOutputStream {
public:
    virtual ~wxOutputStream() {}
    virtual void Write(const void *buffer, size_t size) = 0;
    void PutC(char c) { Write(&c, 1); }
};

class wxStreamBuffer {
    std::vector<uint8_t>* buf;
public:
    wxStreamBuffer(std::vector<uint8_t>& b) : buf(&b) {}
    void* GetBufferStart() { return buf->data(); }
    void* GetBufferEnd() { return buf->data() + buf->size(); }
};

class wxMemoryOutputStream : public wxOutputStream {
    std::vector<uint8_t> buffer;
    wxStreamBuffer streamBuf{buffer};
public:
    void Write(const void *p, size_t size) override {
        const uint8_t* up = (const uint8_t*)p;
        buffer.insert(buffer.end(), up, up + size);
    }
    wxStreamBuffer* GetOutputStreamBuffer() {
        streamBuf = wxStreamBuffer(buffer); // Update pointer
        return &streamBuf;
    }
};

class wxZlibOutputStream : public wxOutputStream {
    wxOutputStream& parent;
public:
    wxZlibOutputStream(wxOutputStream& os, int level=9) : parent(os) {}
    bool IsOk() { return true; }
    void Write(const void *buffer, size_t size) override {
        parent.Write(buffer, size); // No actual compression in shim
    }
};

class wxInputStream {
public:
    virtual ~wxInputStream() {}
    virtual bool IsOk() { return true; }
    virtual size_t Read(void *buffer, size_t size) = 0;
    virtual char GetC() { char c; Read(&c, 1); return c; }
    virtual off_t TellI() { return 0; }
    virtual void SeekI(off_t pos, int mode = 0) {}
};
// wxFromCurrent
const int wxFromCurrent = 1;

class wxMemoryInputStream : public wxInputStream {
    std::vector<uint8_t> internal_buf;
    size_t pos = 0;
public:
    wxMemoryInputStream(const void* data, size_t len) {
        const uint8_t* up = (const uint8_t*)data;
        internal_buf.assign(up, up+len);
    }
    size_t Read(void *buffer, size_t size) override {
        size_t copy = std::min(size, internal_buf.size() - pos);
        memcpy(buffer, internal_buf.data() + pos, copy);
        pos += copy;
        return copy;
    }
};

class wxZlibInputStream : public wxInputStream {
    std::vector<uint8_t> decompressed;
    size_t pos = 0;
    bool ok = false;
public:
    wxZlibInputStream(wxInputStream& is) {
        // Read all remaining data from parent stream
        std::vector<uint8_t> compressed;
        uint8_t buf[4096];
        size_t n;
        while ((n = is.Read(buf, sizeof(buf))) > 0) {
            compressed.insert(compressed.end(), buf, buf + n);
        }

        if (compressed.empty()) {
            ok = false;
            return;
        }

        // Initialize zlib for decompression (zlib format, not raw deflate)
        z_stream strm = {};
        strm.next_in = compressed.data();
        strm.avail_in = compressed.size();

        // Use inflateInit for zlib format (with header)
        if (inflateInit(&strm) != Z_OK) {
            ok = false;
            return;
        }

        // Decompress in chunks
        uint8_t outbuf[8192];
        int ret;
        do {
            strm.next_out = outbuf;
            strm.avail_out = sizeof(outbuf);
            ret = inflate(&strm, Z_NO_FLUSH);
            if (ret == Z_STREAM_ERROR || ret == Z_DATA_ERROR || ret == Z_MEM_ERROR) {
                inflateEnd(&strm);
                ok = false;
                return;
            }
            size_t have = sizeof(outbuf) - strm.avail_out;
            decompressed.insert(decompressed.end(), outbuf, outbuf + have);
        } while (ret != Z_STREAM_END);

        inflateEnd(&strm);
        ok = true;
    }

    bool IsOk() const { return ok; }

    size_t Read(void *buffer, size_t size) override {
        if (!ok || pos >= decompressed.size()) return 0;
        size_t avail = decompressed.size() - pos;
        size_t toRead = std::min(size, avail);
        memcpy(buffer, decompressed.data() + pos, toRead);
        pos += toRead;
        return toRead;
    }

    size_t TellI() const { return pos; }

    void SeekI(size_t newpos) {
        if (newpos <= decompressed.size()) pos = newpos;
    }
};

// Streams - actual implementations for file I/O
class wxDataOutputStream {
    wxOutputStream* os = nullptr;
public:
    wxDataOutputStream(wxOutputStream& s) : os(&s) {}

    void Write8(wxUint8 val) {
        if (os) os->Write(&val, 1);
    }

    void Write32(uint32_t val) {
        // Big-endian (network byte order) for cross-platform compatibility
        wxUint8 buf[4];
        buf[0] = (val >> 24) & 0xFF;
        buf[1] = (val >> 16) & 0xFF;
        buf[2] = (val >> 8) & 0xFF;
        buf[3] = val & 0xFF;
        if (os) os->Write(buf, 4);
    }

    void Write64(wxInt64 val) {
        wxUint8 buf[8];
        buf[0] = (val >> 56) & 0xFF;
        buf[1] = (val >> 48) & 0xFF;
        buf[2] = (val >> 40) & 0xFF;
        buf[3] = (val >> 32) & 0xFF;
        buf[4] = (val >> 24) & 0xFF;
        buf[5] = (val >> 16) & 0xFF;
        buf[6] = (val >> 8) & 0xFF;
        buf[7] = val & 0xFF;
        if (os) os->Write(buf, 8);
    }

    void Write64(const wxLongLong& val) {
        Write64(val.GetValue());
    }

    void Write64(const wxLongLong* val, size_t count) {
        for (size_t i = 0; i < count; i++) {
            Write64(val[i]);
        }
    }

    void WriteString(const wxString& s) {
        // Write length as 32-bit, then string data (no null terminator)
        Write32((uint32_t)s.length());
        if (os && !s.empty()) os->Write(s.c_str(), s.length());
    }

    void WriteDouble(double d) {
        // Write as 64-bit IEEE 754 (just copy bytes, assuming same endianness)
        if (os) os->Write(&d, sizeof(double));
    }
};

class wxDataInputStream {
    wxInputStream* is = nullptr;
    bool bigEndian = true;
public:
    wxDataInputStream(wxInputStream& s) : is(&s) {}

    void BigEndianOrdered(bool be) { bigEndian = be; }

    wxUint8 Read8() {
        wxUint8 val = 0;
        if (is) is->Read(&val, 1);
        return val;
    }

    uint32_t Read32() {
        wxUint8 buf[4] = {0};
        if (is) is->Read(buf, 4);
        if (bigEndian) {
            return ((uint32_t)buf[0] << 24) | ((uint32_t)buf[1] << 16) |
                   ((uint32_t)buf[2] << 8) | (uint32_t)buf[3];
        } else {
            return ((uint32_t)buf[3] << 24) | ((uint32_t)buf[2] << 16) |
                   ((uint32_t)buf[1] << 8) | (uint32_t)buf[0];
        }
    }

    wxInt64 Read64() {
        wxUint8 buf[8] = {0};
        if (is) is->Read(buf, 8);
        if (bigEndian) {
            return ((wxInt64)buf[0] << 56) | ((wxInt64)buf[1] << 48) |
                   ((wxInt64)buf[2] << 40) | ((wxInt64)buf[3] << 32) |
                   ((wxInt64)buf[4] << 24) | ((wxInt64)buf[5] << 16) |
                   ((wxInt64)buf[6] << 8) | (wxInt64)buf[7];
        } else {
            return ((wxInt64)buf[7] << 56) | ((wxInt64)buf[6] << 48) |
                   ((wxInt64)buf[5] << 40) | ((wxInt64)buf[4] << 32) |
                   ((wxInt64)buf[3] << 24) | ((wxInt64)buf[2] << 16) |
                   ((wxInt64)buf[1] << 8) | (wxInt64)buf[0];
        }
    }

    void Read64(wxLongLong* val, size_t count) {
        for (size_t i = 0; i < count; i++) {
            val[i] = wxLongLong(Read64());
        }
    }

    wxString ReadString() {
        uint32_t len = Read32();
        if (len == 0 || !is) return "";
        std::vector<char> buf(len);
        is->Read(buf.data(), len);
        return wxString(std::string(buf.data(), len));
    }

    double ReadDouble() {
        double d = 0.0;
        if (is) is->Read(&d, sizeof(double));
        return d;
    }
};

// Graphics types used in core
struct wxPoint { int x, y; wxPoint(int _x=0, int _y=0) : x(_x), y(_y) {} };
struct wxSize { int x, y; wxSize(int w=0, int h=0) : x(w), y(h) {} };
const wxPoint wxDefaultPosition = wxPoint(-1, -1);
const wxSize wxDefaultSize = wxSize(-1, -1);
struct wxRect {
    int x, y, width, height;
    wxRect(int _x=0, int _y=0, int _w=0, int _h=0) : x(_x), y(_y), width(_w), height(_h) {}
};
struct wxColour {
    uint32_t col;
    wxColour(uint32_t c) : col(c) {}
    wxColour(int r, int g, int b) { col = (r << 16) | (g << 8) | b; }
};

// Defines
#define wxUSE_UNICODE 1

// Helper functions (mocks)
inline bool wxIsalnum(char c) { return isalnum(c); }
inline bool wxIsspace(char c) { return isspace(c); }
inline bool wxIspunct(char c) { return ispunct(c); }
inline wxString _(const wchar_t* s) { return wxString(s); }
inline wxString _(const char* s) { return wxString(s); }
inline const wchar_t* wxStrchr(const wchar_t* s, wchar_t c) {
    while(s && *s) { if(*s == c) return s; s++; }
    return c == 0 ? s : nullptr;
}

enum wxBitmapType { wxBITMAP_TYPE_PNG, wxBITMAP_TYPE_JPEG };
class wxBitmap {
public:
    wxBitmap() {}
    wxBitmap(int w, int h, int d=0) {}
    int GetWidth() const { return 0; }
    int GetHeight() const { return 0; }
};

enum {
    wxID_ANY = -1,
    wxID_NEW = 5000, wxID_OPEN, wxID_CLOSE, wxID_SAVE, wxID_SAVEAS,
    wxID_PREVIEW, wxID_PRINT, wxID_EXIT, wxID_SELECTALL,
    wxID_BOLD, wxID_ITALIC, wxID_UNDERLINE, wxID_STRIKETHROUGH,
    wxID_CUT, wxID_COPY, wxID_PASTE, wxID_UNDO, wxID_REDO,
    wxID_FIND, wxID_REPLACE, wxID_SELECT_FONT, wxID_EXECUTE,
    wxID_ABOUT, wxID_HELP
};

inline wxString wxFindFirstFile(const wxString& s, int f = 0) { return ""; }
inline wxString wxFindNextFile() { return ""; }
class wxFileName {
public:
    wxFileName(const wxString& s) {}
    static wxFileName FileName(const wxString& s) { return wxFileName(s); }
    wxString GetFullName() { return ""; }
};
