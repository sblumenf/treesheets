#pragma once

#ifndef __WXWIDGETS__
#include "wasm/wx_shim.h"
#else
#include <wx/string.h>
#include <wx/datetime.h>
#include <wx/arrstr.h>
#endif

struct Document;

struct TSDialogs {
    virtual ~TSDialogs() {}
    virtual void ShowMessage(const wxString& msg, const wxString& title) = 0;
    virtual int ThreeChoice(const wxString& title, const wxString& msg, const wxString& ch1, const wxString& ch2, const wxString& ch3) = 0;
    virtual bool DateTimeRange(wxDateTime& begin, wxDateTime& end) = 0;
    virtual wxString AskText(const wxString& msg, const wxString& title, const wxString& defaultVal = "") = 0;
    virtual double AskNumber(const wxString& msg, const wxString& title, double defaultVal, double min, double max) = 0;
    virtual void GetFilesFromUser(wxArrayString& filenames, const wxString& title, const wxString& filter) = 0;
    virtual bool SelectFont(wxString& fontName, int& fontSize) = 0;
    virtual uint PickColor(uint defaultColor) = 0;
    virtual wxString FileSelector(const wxString& message, const wxString& default_path, const wxString& default_filename, const wxString& default_extension, const wxString& wildcard, int flags) = 0;
    virtual int SingleChoice(const wxString& title, const wxString& msg, const wxArrayString& choices) = 0;
    virtual void ShowAbout(const wxString& title, const wxString& version, const wxString& desc) = 0;
    virtual void PageSetup(Document* doc) = 0;
    virtual void Print(Document* doc) = 0;
    virtual void PrintPreview(Document* doc) = 0;
};
