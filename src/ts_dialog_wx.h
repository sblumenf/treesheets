struct TSDialogsWx : public TSDialogs {
    void ShowMessage(const wxString& msg, const wxString& title) override {
        wxMessageBox(msg, title, wxOK, sys->frame);
    }
    int ThreeChoice(const wxString& title, const wxString& msg, const wxString& ch1, const wxString& ch2, const wxString& ch3) override {
        ThreeChoiceDialog tcd(sys->frame, title, msg, ch1, ch2, ch3);
        return tcd.Run();
    }
    bool DateTimeRange(wxDateTime& begin, wxDateTime& end) override {
        DateTimeRangeDialog rd(sys->frame);
        if(rd.Run() == wxID_OK) {
            begin = rd.begin;
            end = rd.end;
            return true;
        }
        return false;
    }
    wxString AskText(const wxString& msg, const wxString& title, const wxString& defaultVal) override {
        return wxGetTextFromUser(msg, title, defaultVal, sys->frame);
    }
    double AskNumber(const wxString& msg, const wxString& title, double defaultVal, double min, double max) override {
        return wxGetNumberFromUser(msg, title, title, (long)defaultVal, (long)min, (long)max, sys->frame);
    }
    void GetFilesFromUser(wxArrayString& filenames, const wxString& title, const wxString& filter) override {
        treesheets::GetFilesFromUser(filenames, sys->frame, title, filter);
    }
    bool SelectFont(wxString& fontName, int& fontSize) override {
        wxFontData fdat;
        fdat.SetInitialFont(wxFont(fontSize, wxFONTFAMILY_DEFAULT, wxFONTSTYLE_NORMAL, wxFONTWEIGHT_NORMAL, false, fontName));
        wxFontDialog fd(sys->frame, fdat);
        if (fd.ShowModal() == wxID_OK) {
            wxFont font = fd.GetFontData().GetChosenFont();
            fontName = font.GetFaceName();
            fontSize = font.GetPointSize();
            return true;
        }
        return false;
    }
    uint PickColor(uint defaultColor) override {
        return treesheets::PickColor(sys->frame, defaultColor);
    }
    wxString FileSelector(const wxString& message, const wxString& default_path, const wxString& default_filename, const wxString& default_extension, const wxString& wildcard, int flags) override {
        return ::wxFileSelector(message, default_path, default_filename, default_extension, wildcard, flags, sys->frame);
    }
    int SingleChoice(const wxString& title, const wxString& msg, const wxArrayString& choices) override {
        wxSingleChoiceDialog choice(sys->frame, msg, title, choices);
        choice.SetSize(wxSize(500, 700));
        choice.Centre();
        if (choice.ShowModal() == wxID_OK) return choice.GetSelection();
        return -1;
    }
    void ShowAbout(const wxString& title, const wxString& version, const wxString& desc) override {
        wxAboutDialogInfo info;
        info.SetName(title);
        info.SetVersion(version);
        info.SetDescription(desc);
        wxAboutBox(info);
    }
    void PageSetup(Document* doc) override {
        doc->pageSetupData = doc->printData;
        wxPageSetupDialog pageSetupDialog(sys->frame, &doc->pageSetupData);
        pageSetupDialog.ShowModal();
        doc->printData = pageSetupDialog.GetPageSetupDialogData().GetPrintData();
        doc->pageSetupData = pageSetupDialog.GetPageSetupDialogData();
    }
    void Print(Document* doc) override {
        wxPrintDialogData printDialogData(doc->printData);
        wxPrinter printer(&printDialogData);
        Document::Printout printout(doc);
        if (printer.Print(sys->frame, &printout, true)) {
            doc->printData = printer.GetPrintDialogData().GetPrintData();
        }
    }
    void PrintPreview(Document* doc) override {
        wxPrintDialogData printDialogData(doc->printData);
        auto preview = new wxPrintPreview(new Document::Printout(doc), new Document::Printout(doc), &printDialogData);
        auto pframe = new wxPreviewFrame(preview, sys->frame, _(L"Print Preview"), wxPoint(100, 100), wxSize(600, 650));
        pframe->Centre(wxBOTH);
        pframe->Initialize();
        pframe->Show(true);
    }
};
