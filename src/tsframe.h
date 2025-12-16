#include "ts_menu_wx.h"

struct TSFrame : wxFrame {
    TSApp *app;
    wxIcon icon;
    wxTaskBarIcon taskbaricon;
    TSMenu *editmenupopup = nullptr;
    wxFileHistory filehistory;
    wxFileHistory scripts {A_MAXACTION - A_SCRIPT, A_SCRIPT};
    wxFileSystemWatcher *watcher;
    wxAuiNotebook *notebook {nullptr};
    wxAuiManager aui {this};
    wxBitmap line_nw;
    wxBitmap line_sw;
    wxBitmap foldicon;
    bool fromclosebox {true};
    bool watcherwaitingforuser {false};
    wxToolBar *toolbar {nullptr};
    wxColour toolbarbackgroundcolor {0xD8C7BC};
    wxTextCtrl *filter {nullptr};
    wxTextCtrl *replaces {nullptr};
    ColorDropdown *cellcolordropdown {nullptr};
    ColorDropdown *textcolordropdown {nullptr};
    ColorDropdown *bordercolordropdown {nullptr};
    ImageDropdown *imagedropdown {nullptr};
    wxString imagepath;
    int refreshhack {0};
    int refreshhackinstances {0};
    std::map<wxString, wxString> menustrings;

    using TSPlatformMenu = TSWxMenu;
    using TSPlatformMenuBar = TSWxMenuBar;

    TSFrame(TSApp *_app)
        : wxFrame((wxFrame *)nullptr, wxID_ANY, L"TreeSheets", wxDefaultPosition, wxDefaultSize,
                  wxDEFAULT_FRAME_STYLE),
          app(_app) {
        sys->frame = this;

        class MyLog : public wxLog {
            void DoLogString(const wxChar *message, time_t timestamp) { DoLogText(*message); }
            void DoLogText(const wxString &message) {
                #ifdef WIN32
                OutputDebugString(message.c_str());
                OutputDebugString(L"\n");
                #else
                fputws(message.c_str(), stderr);
                fputws(L"\n", stderr);
                #endif
            }
        };

        wxLog::SetActiveTarget(new MyLog());

        wxLogMessage(L"%s", wxVERSION_STRING);

        wxInitAllImageHandlers();

        wxIconBundle icons;
        wxIcon iconbig;
        #ifdef WIN32
            int iconsmall = ::GetSystemMetrics(SM_CXSMICON);
            int iconlarge = ::GetSystemMetrics(SM_CXICON);
        #endif
        icon.LoadFile(app->GetDataPath(L"images/icon16.png"), wxBITMAP_TYPE_PNG
            #ifdef WIN32
                , iconsmall, iconsmall
            #endif
        );
        iconbig.LoadFile(app->GetDataPath(L"images/icon32.png"), wxBITMAP_TYPE_PNG
            #ifdef WIN32
                , iconlarge, iconlarge
            #endif
        );
        if (!icon.IsOk() || !iconbig.IsOk()) {
            wxMessageBox(_(L"Error loading core data file (TreeSheets not installed correctly?)"),
                         _(L"Initialization Error"), wxOK, this);
            exit(1);
        }
        icons.AddIcon(icon);
        icons.AddIcon(iconbig);
        SetIcons(icons);

        RenderFolderIcon();
        line_nw.LoadFile(app->GetDataPath(L"images/render/line_nw.png"), wxBITMAP_TYPE_PNG);
        line_sw.LoadFile(app->GetDataPath(L"images/render/line_sw.png"), wxBITMAP_TYPE_PNG);

        imagepath = app->GetDataPath("images/nuvola/dropdown/");

        if (sys->singletray)
            taskbaricon.Connect(wxID_ANY, wxEVT_TASKBAR_LEFT_UP,
                        wxTaskBarIconEventHandler(TSFrame::OnTBIDBLClick), nullptr, this);
        else
            taskbaricon.Connect(wxID_ANY, wxEVT_TASKBAR_LEFT_DCLICK,
                        wxTaskBarIconEventHandler(TSFrame::OnTBIDBLClick), nullptr, this);

        bool showtbar, showsbar, lefttabs;

        sys->cfg->Read(L"showtbar", &showtbar, true);
        sys->cfg->Read(L"showsbar", &showsbar, true);
        sys->cfg->Read(L"lefttabs", &lefttabs, true);

        filehistory.Load(*sys->cfg);
        auto oldpath = sys->cfg->GetPath();
        sys->cfg->SetPath("/scripts");
        scripts.Load(*sys->cfg);
        sys->cfg->SetPath(oldpath);

        #ifdef __WXMAC__
            #define CTRLORALT "CTRL"
        #else
            #define CTRLORALT "ALT"
        #endif

        #ifdef __WXMAC__
            #define ALTORCTRL "ALT"
        #else
            #define ALTORCTRL "CTRL"
        #endif

        CreateMenus(lefttabs);
        ConstructToolBar();

        auto sb = CreateStatusBar(5);
        SetStatusBarPane(0);
        SetDPIAwareStatusWidths();
        sb->Show(sys->showstatusbar);

        notebook =
            new wxAuiNotebook(this, wxID_ANY, wxDefaultPosition, wxDefaultSize,
                              wxAUI_NB_TAB_MOVE | wxAUI_NB_TAB_SPLIT | wxAUI_NB_SCROLL_BUTTONS |
                                  wxAUI_NB_WINDOWLIST_BUTTON | wxAUI_NB_CLOSE_ON_ALL_TABS |
                                  (lefttabs ? wxAUI_NB_BOTTOM : wxAUI_NB_TOP));

        int display_id = wxDisplay::GetFromWindow(this);
        wxRect disprect = wxDisplay(display_id == wxNOT_FOUND ? 0 : display_id).GetClientArea();
        const int boundary = 64;
        const int defx = disprect.width - 2 * boundary;
        const int defy = disprect.height - 2 * boundary;
        int resx, resy, posx, posy;
        sys->cfg->Read(L"resx", &resx, defx);
        sys->cfg->Read(L"resy", &resy, defy);
        sys->cfg->Read(L"posx", &posx, boundary + disprect.x);
        sys->cfg->Read(L"posy", &posy, boundary + disprect.y);
        #ifndef __WXGTK__
        // On X11, disprect only refers to the primary screen. Thus, for a multi-head display,
        // the conditions below might be fulfilled (e.g. large window spanning multiple screens
        // or being on the secondary screen), so just ignore them.
        if (resx > disprect.width || resy > disprect.height || posx < disprect.x ||
            posy < disprect.y || posx + resx > disprect.width + disprect.x ||
            posy + resy > disprect.height + disprect.y) {
            // Screen res has been resized since we last ran, set sizes to default to avoid being
            // off-screen.
            resx = defx;
            resy = defy;
            posx = posy = boundary;
            posx += disprect.x;
            posy += disprect.y;
        }
        #endif
        SetSize(resx, resy);
        Move(posx, posy);

        bool ismax;
        sys->cfg->Read(L"maximized", &ismax, true);

        aui.AddPane(notebook, wxCENTER);
        aui.Update();

        Show(!IsIconized());

        // needs to be after Show() to avoid scrollbars rendered in the wrong place?
        if (ismax && !IsIconized()) Maximize(true);

        SetFileAssoc(app->exename);

        wxSafeYield();
    }

    void FileHistoryUseMenu(TSMenu* menu) {
        filehistory.UseMenu(dynamic_cast<TSWxMenu*>(menu)->GetWxMenu());
    }
    void ScriptsUseMenu(TSMenu* menu) {
        scripts.UseMenu(dynamic_cast<TSWxMenu*>(menu)->GetWxMenu());
    }
    void SetMenuBar(TSMenuBar* menubar) {
        wxFrame::SetMenuBar(dynamic_cast<TSWxMenuBar*>(menubar)->GetWxMenuBar());
    }
    void SetDefaultAccelerators() {
        wxAcceleratorEntry entries[3];
        entries[0].Set(wxACCEL_SHIFT, WXK_DELETE, wxID_CUT);
        entries[1].Set(wxACCEL_SHIFT, WXK_INSERT, wxID_PASTE);
        entries[2].Set(wxACCEL_CTRL, WXK_INSERT, wxID_COPY);
        wxAcceleratorTable accel(3, entries);
        SetAcceleratorTable(accel);
    }

    #include "ts_menu_builder.h"
    #include "ts_toolbar_builder.h"

    void AppOnEventLoopEnter() {
        watcher = new wxFileSystemWatcher();
        watcher->SetOwner(this);
        Connect(wxEVT_FSWATCHER, wxFileSystemWatcherEventHandler(TSFrame::OnFileSystemEvent));
    }

    // event handling functions

    #include "ts_action_handler.h"

    void OnMenu(wxCommandEvent &ce) {
        HandleAction(ce.GetId(), ce.IsChecked());
    }

    void OnTabChange(wxAuiNotebookEvent &nbe) {
        auto canvas = static_cast<TSCanvas *>(notebook->GetPage(nbe.GetSelection()));
        SetStatus();
        sys->TabChange(canvas->doc);
        nbe.Skip();
    }

    void OnTabClose(wxAuiNotebookEvent &nbe) {
        auto canvas = static_cast<TSCanvas *>(notebook->GetPage(nbe.GetSelection()));
        if (notebook->GetPageCount() <= 1) {
            nbe.Veto();
            Close();
        } else if (canvas->doc->CloseDocument()) {
            nbe.Veto();
        } else {
            nbe.Skip();
        }
    }

    void OnUpdateStatusBarRequest(wxCommandEvent &ce) {
        TSCanvas *canvas = GetCurrentTab();
        if (canvas && canvas->doc->selected.grid) UpdateStatus(canvas->doc->selected);
    }

    void OnSearch(wxCommandEvent &ce) {
        auto searchstring = ce.GetString();
        sys->darkennonmatchingcells = searchstring.Len() != 0;
        sys->searchstring = sys->casesensitivesearch ? searchstring : searchstring.Lower();
        TSCanvas *canvas = GetCurrentTab();
        Document *doc = canvas->doc;
        if (doc->searchfilter) {
            doc->SetSearchFilter(sys->searchstring.Len() != 0);
            doc->searchfilter = true;
        }
        canvas->Refresh();
    }

    void OnSearchReplaceEnter(wxCommandEvent &ce) {
        auto canvas = GetCurrentTab();
        if (ce.GetId() == A_SEARCH && ce.GetString().IsEmpty())
            canvas->SetFocus();
        else
            canvas->doc->Action(ce.GetId() == A_SEARCH ? A_SEARCHNEXT : A_REPLACEONCEJ);
    }

    void OnChangeColor(wxCommandEvent &ce) {
        GetCurrentTab()->doc->ColorChange(ce.GetId(), ce.GetInt());
        ReFocus();
    }

    void OnDDImage(wxCommandEvent &ce) {
        GetCurrentTab()->doc->ImageChange(imagedropdown->filenames[ce.GetInt()], dd_icon_res_scale);
        ReFocus();
    }

    void OnActivate(wxActivateEvent &ae) {
        // This causes warnings in the debug log, but without it keyboard entry upon window select
        // doesn't work.
        ReFocus();
    }

    void OnSizing(wxSizeEvent &se) { se.Skip(); }

    void OnMaximize(wxMaximizeEvent &me) {
        ReFocus();
        me.Skip();
    }

    void OnIconize(wxIconizeEvent &me) {
        if (me.IsIconized()) {
            #ifndef __WXMAC__
            if (sys->totray) {
                taskbaricon.SetIcon(icon, L"TreeSheets");
                Show(false);
                Iconize();
            }
            #endif
        } else {
            #ifdef __WXGTK__
            if (sys->totray) {
                Show(true);
            }
            #endif
            if (TSCanvas *canvas = GetCurrentTab()) canvas->SetFocus();
        }
    }

    void OnTBIDBLClick(wxTaskBarIconEvent &e) { DeIconize(); }

    void OnClosing(wxCloseEvent &ce) {
        bool fcb = fromclosebox;
        fromclosebox = true;
        if (fcb && sys->minclose) {
            ce.Veto();
            Iconize();
            return;
        }
        sys->RememberOpenFiles();
        if (ce.CanVeto()) {
            // ask to save/discard all files before closing any
            loop(i, notebook->GetPageCount()) {
                auto canvas = static_cast<TSCanvas *>(notebook->GetPage(i));
                if (canvas->doc->modified) {
                    notebook->SetSelection(i);
                    if (canvas->doc->CheckForChanges()) {
                        ce.Veto();
                        return;
                    }
                }
            }
            // all files have been saved/discarded
            while (notebook->GetPageCount()) {
                GetCurrentTab()->doc->RemoveTmpFile();
                notebook->DeletePage(notebook->GetSelection());
            }
        }
        sys->every_second_timer.Stop();
        filehistory.Save(*sys->cfg);
        auto oldpath = sys->cfg->GetPath();
        sys->cfg->SetPath("/scripts");
        scripts.Save(*sys->cfg);
        sys->cfg->SetPath(oldpath);
        if (!IsIconized()) {
            sys->cfg->Write(L"maximized", IsMaximized());
            if (!IsMaximized()) {
                sys->cfg->Write(L"resx", GetSize().x);
                sys->cfg->Write(L"resy", GetSize().y);
                sys->cfg->Write(L"posx", GetPosition().x);
                sys->cfg->Write(L"posy", GetPosition().y);
            }
        }
        aui.ClearEventHashTable();
        aui.UnInit();
        if (editmenupopup) {
            delete static_cast<TSWxMenu*>(editmenupopup)->GetWxMenu();
            delete editmenupopup;
            editmenupopup = nullptr;
        }
        DELETEP(watcher);
        Destroy();
    }

    void OnFileSystemEvent(wxFileSystemWatcherEvent &event) {
        // 0xF == create/delete/rename/modify
        if ((event.GetChangeType() & 0xF) == 0 || watcherwaitingforuser || !notebook) return;
        const auto &modfile = event.GetPath().GetFullPath();
        loop(i, notebook->GetPageCount()) {
            Document *doc = static_cast<TSCanvas *>(notebook->GetPage(i))->doc;
            if (modfile == doc->filename) {
                auto modtime = wxFileName(modfile).GetModificationTime();
                // Compare with last modified to trigger multiple times.
                if (!modtime.IsValid() || !doc->lastmodificationtime.IsValid() ||
                    modtime == doc->lastmodificationtime) {
                    return;
                }
                if (doc->modified) {
                    // TODO: this dialog is problematic since it may be on an unattended
                    // computer and more of these events may fire. since the occurrence of this
                    // situation is rare, it may be better to just take the most
                    // recently changed version (which is the one that has just been modified
                    // on disk) this potentially throws away local changes, but this can only
                    // happen if the user left changes unsaved, then decided to go edit an older
                    // version on another computer.
                    // for now, we leave this code active, and guard it with
                    // watcherwaitingforuser
                    auto message = wxString::Format(
                        _(L"%s\nhas been modified on disk by another program / computer:\nWould you like to discard your changes and re-load from disk?"),
                        doc->filename);
                    watcherwaitingforuser = true;
                    int res = wxMessageBox(message, _(L"File modification conflict!"),
                                           wxYES_NO | wxICON_QUESTION, this);
                    watcherwaitingforuser = false;
                    if (res != wxYES) return;
                }
                auto message = sys->LoadDB(doc->filename, true);
                assert(message);
                if (*message) {
                    SetStatus(message);
                } else {
                    loop(j,
                         notebook->GetPageCount()) if (static_cast<TSCanvas *>(notebook->GetPage(j))
                                                           ->doc == doc) notebook->DeletePage(j);
                    ::wxRemoveFile(sys->TmpName(modfile));
                    SetStatus(
                        _(L"File has been re-loaded because of modifications of another program / computer"));
                }
                return;
            }
        }
    }

    void OnDPIChanged(wxDPIChangedEvent &dce) {
        // block all other events until we finished preparing
        wxEventBlocker blocker(this);
        wxBusyCursor wait;
        {
            ThreadPool pool(std::thread::hardware_concurrency());
            for (const auto &image : sys->imagelist) {
                pool.enqueue(
                    [](auto img) {
                        img->bm_display = wxNullBitmap;
                        img->Display();
                    },
                    image.get());
            }
        }  // wait until all tasks are finished
        RenderFolderIcon();
        dce.Skip();
    }

    void OnSysColourChanged(wxSysColourChangedEvent &se) {
        sys->darkmode = sys->followdarkmode && wxSystemSettings::GetAppearance().IsDark();
        DELETEP(toolbar);
        ConstructToolBar();
        se.Skip();
    }

    // helper functions

    void CycleTabs(int offset = 1) {
        auto numtabs = static_cast<int>(notebook->GetPageCount());
        offset = offset >= 0 ? 1 : numtabs - 1;  // normalize to non-negative wrt modulo
        notebook->SetSelection((notebook->GetSelection() + offset) % numtabs);
    }

    void DeIconize() {
        if (!IsIconized()) {
            RequestUserAttention();
            return;
        }
        Show(true);
        Iconize(false);
        taskbaricon.RemoveIcon();
    }

    TSCanvas *GetCurrentTab() {
        return notebook ? static_cast<TSCanvas *>(notebook->GetCurrentPage()) : nullptr;
    }

    TSCanvas *GetTabByFileName(const wxString &filename) {
        if (notebook) loop(i, notebook->GetPageCount()) {
                auto canvas = static_cast<TSCanvas *>(notebook->GetPage(i));
                if (canvas->doc->filename == filename) {
                    notebook->SetSelection(i);
                    return canvas;
                }
            }
        return nullptr;
    }

    TSCanvas *NewTab(Document *doc, bool append = false) {
        TSCanvas *canvas = new TSCanvas(this, notebook);
        canvas->doc = doc;
        doc->canvas = canvas;
        canvas->SetScrollRate(1, 1);
        if (append)
            notebook->AddPage(canvas, _(L"<unnamed>"), true, wxNullBitmap);
        else
            notebook->InsertPage(0, canvas, _(L"<unnamed>"), true, wxNullBitmap);
        canvas->SetDropTarget(new DropTarget(doc->dndobjc));
        canvas->SetFocus();
        return canvas;
    }

    void ReFocus() {
        if (TSCanvas *canvas = GetCurrentTab()) canvas->SetFocus();
    }

    void RenderFolderIcon() {
        wxImage foldiconi;
        foldiconi.LoadFile(app->GetDataPath(L"images/nuvola/fold.png"));
        foldicon = wxBitmap(foldiconi);
        ScaleBitmap(foldicon, FromDIP(1.0) / 3.0, foldicon);
    }

    void SetDPIAwareStatusWidths() {
        int statusbarfieldwidths[] = {-1, FromDIP(300), FromDIP(120), FromDIP(100), FromDIP(150)};
        SetStatusWidths(5, statusbarfieldwidths);
    }

    void SetFileAssoc(wxString &exename) {
        #ifdef WIN32
        SetRegistryKey(L"HKEY_CURRENT_USER\\Software\\Classes\\.cts", L"TreeSheets");
        SetRegistryKey(L"HKEY_CURRENT_USER\\Software\\Classes\\TreeSheets", L"TreeSheets file");
        SetRegistryKey(L"HKEY_CURRENT_USER\\Software\\Classes\\TreeSheets\\Shell\\Open\\Command",
                       wxString(L"\"") + exename + L"\" \"%1\"");
        SetRegistryKey(L"HKEY_CURRENT_USER\\Software\\Classes\\TreeSheets\\DefaultIcon",
                       wxString(L"\"") + exename + L"\",0");
        #else
        // TODO: do something similar for mac/kde/gnome?
        #endif
    }

    void SetPageTitle(const wxString &filename, wxString mods, int page = -1) {
        if (page < 0) page = notebook->GetSelection();
        if (page < 0) return;
        if (page == notebook->GetSelection()) SetTitle(L"TreeSheets - " + filename + mods);
        notebook->SetPageText(
            page,
            (filename.empty() ? wxString(_(L"<unnamed>")) : wxFileName(filename).GetName()) + mods);
    }

    #ifdef WIN32
    void SetRegistryKey(const wxChar *key, wxString value) {
        wxRegKey registrykey(key);
        registrykey.Create();
        registrykey.SetValue(L"", value);
    }
    #endif

    void SetStatus(const wxChar *message = nullptr) {
        if (GetStatusBar() && (!message || *message)) SetStatusText(message ? message : L"", 0);
    }

    void TabsReset() {
        if (notebook) loop(i, notebook->GetPageCount()) {
                auto canvas = static_cast<TSCanvas *>(notebook->GetPage(i));
                canvas->doc->root->ResetChildren();
            }
    }

    void UpdateStatus(const Selection &s) {
        if (GetStatusBar()) {
            if (Cell *c = s.GetCell(); c && s.xs) {
                SetStatusText(wxString::Format(_(L"Size %d"), -c->text.relsize), 3);
                SetStatusText(wxString::Format(_(L"Width %d"), s.grid->colwidths[s.x]), 2);
                SetStatusText(
                    wxString::Format(_(L"Edited %s %s"), c->text.lastedit.FormatDate().c_str(),
                                     c->text.lastedit.FormatTime().c_str()),
                    1);
            } else
                for (int field : {1, 2, 3}) SetStatusText("", field);
            SetStatusText(wxString::Format(_(L"%d cell(s)"), s.xs * s.ys), 4);
        }
    }

    DECLARE_EVENT_TABLE()
};
