#pragma once

#include <string>
#include <vector>
#include <iostream>
#include <algorithm>
#include <cstdint>
#include <sstream>
#include <cstdarg>
#include <cstring> // memcpy

// Basic Types
using wxUint8 = uint8_t;
using wxUint32 = uint32_t;
using wxInt64 = int64_t;
using uchar = unsigned char;
using uint = unsigned int;
using wxChar = wchar_t;

// wxString Shim
class wxString : public std::string {
public:
    wxString() : std::string() {}
    wxString(const char* s) : std::string(s ? s : "") {}
    wxString(const std::string& s) : std::string(s) {}
    wxString(const wchar_t* s) {
        if(s) while(*s) { push_back((char)*s); s++; }
    }

    static wxString Format(const char* fmt, ...) {
        char buf[1024];
        va_list args;
        va_start(args, fmt);
        vsnprintf(buf, sizeof(buf), fmt, args);
        va_end(args);
        return wxString(buf);
    }

    static wxString Format(const wchar_t* fmt, ...) {
        return "FormatW";
    }

    size_t Len() const { return length(); }
    bool IsEmpty() const { return empty(); }
    void Clear() { std::string::clear(); }

    wxString Left(size_t n) const { return substr(0, n); }
    wxString Mid(size_t start, size_t count = npos) const {
        if(start >= length()) return "";
        return substr(start, count);
    }
    void Truncate(size_t len) { if(len < length()) resize(len); }
    wxString Trim(bool fromRight = true) { return *this; } // Mock

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
        while(s && *s) push_back((char)*s++);
        return *this;
    }

    using std::string::operator+=;

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
        s.push_back((char)rhs);
        return s;
    }
    friend wxString operator+(wchar_t lhs, const wxString& rhs) {
        wxString s;
        s.push_back((char)lhs);
        s += rhs;
        return s;
    }
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
    wxInputStream& parent;
public:
    wxZlibInputStream(wxInputStream& is) : parent(is) {}
    bool IsOk() { return true; }
    size_t Read(void *buffer, size_t size) override {
        return parent.Read(buffer, size);
    }
};

// Streams
class wxDataOutputStream {
public:
    wxDataOutputStream(std::ostream& s) {}
    wxDataOutputStream(wxOutputStream& s) {}
    void Write8(wxUint8 val) {}
    void Write32(uint32_t val) {}
    void Write64(const wxLongLong& val) {}
    void Write64(wxInt64 val) {}
    void Write64(const wxLongLong* val, size_t count) {}
    void WriteString(const wxString& s) {}
    void WriteDouble(double d) {}
};

class wxDataInputStream {
public:
    wxDataInputStream(std::istream& s) {}
    wxDataInputStream(wxInputStream& s) {}
    void BigEndianOrdered(bool be) {}
    wxUint8 Read8() { return 0; }
    uint32_t Read32() { return 0; }
    wxInt64 Read64() { return 0; }
    void Read64(wxLongLong* val, size_t count) {}
    wxString ReadString() { return ""; }
    double ReadDouble() { return 0.0; }
};

// Graphics types used in core
struct wxPoint { int x, y; };
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
inline const char* _(const wchar_t* s) { return "translated"; }
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
