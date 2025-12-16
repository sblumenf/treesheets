    void HandleAction(int id, bool checked = false) {
        wxTextCtrl *tc;
        auto canvas = GetCurrentTab();
        #ifdef __WXWIDGETS__
        if ((tc = filter) && filter == wxWindow::FindFocus() ||
            (tc = replaces) && replaces == wxWindow::FindFocus()) {
            long from, to;
            tc->GetSelection(&from, &to);
            switch (id) {
                #if defined(__WXMSW__) || defined(__WXMAC__)
                case A_MLEFT:
                case A_LEFT:
                    if (from != to)
                        tc->SetInsertionPoint(from);
                    else if (from)
                        tc->SetInsertionPoint(from - 1);
                    return;
                case A_MRIGHT:
                case A_RIGHT:
                    if (from != to)
                        tc->SetInsertionPoint(to);
                    else if (to < tc->GetLineLength(0))
                        tc->SetInsertionPoint(to + 1);
                    return;

                case A_SHOME: tc->SetSelection(0, to); return;
                case A_SEND: tc->SetSelection(from, 1000); return;

                case A_SCLEFT:
                case A_SLEFT:
                    if (from) tc->SetSelection(from - 1, to);
                    return;
                case A_SCRIGHT:
                case A_SRIGHT:
                    if (to < tc->GetLineLength(0)) tc->SetSelection(from, to + 1);
                    return;

                case A_BACKSPACE: tc->Remove(from - (from == to), to); return;
                case A_DELETE: tc->Remove(from, to + (from == to)); return;
                case A_HOME: tc->SetSelection(0, 0); return;
                case A_END: tc->SetSelection(1000, 1000); return;
                case wxID_SELECTALL: tc->SetSelection(0, 1000); return;
                #endif
                #ifdef __WXMSW__
                case A_ENTERCELL: {
                    if (tc == filter) {
                        if (sys->searchstring.IsEmpty()) {
                            canvas->SetFocus();
                        } else {
                            canvas->doc->Action(A_SEARCHNEXT);
                        }
                    } else if (tc == replaces) {
                        canvas->doc->Action(A_REPLACEONCEJ);
                    }
                    return;
                }
                #endif
                case A_CANCELEDIT:
                    tc->Clear();
                    canvas->SetFocus();
                    return;
            }
        }
        #endif

        auto Check = [&](const wxChar *cfg) {
            sys->cfg->Write(cfg, checked);
            SetStatus(_(L"change will take effect next run of TreeSheets"));
        };
        switch (id) {
            case A_NOP: break;

            case A_ALEFT: canvas->CursorScroll(-g_scrollratecursor, 0); break;
            case A_ARIGHT: canvas->CursorScroll(g_scrollratecursor, 0); break;
            case A_AUP: canvas->CursorScroll(0, -g_scrollratecursor); break;
            case A_ADOWN: canvas->CursorScroll(0, g_scrollratecursor); break;

            case A_SHOWSBAR:
                if (!IsFullScreen()) {
                    sys->cfg->Write(L"showstatusbar", sys->showstatusbar = checked);
                    // auto wsb = GetStatusBar(); // TSDialogs abstraction needed? No, TSFrame has GetStatusBar
                    // wsb->Show(sys->showstatusbar);
                    // SendSizeEvent();
                    // Refresh();
                    // wsb->Refresh();
                    // Web stub usually doesn't have StatusBar window to show/hide
                    #ifdef __WXWIDGETS__
                    auto wsb = GetStatusBar();
                    wsb->Show(sys->showstatusbar);
                    SendSizeEvent();
                    Refresh();
                    wsb->Refresh();
                    #endif
                }
                break;
            case A_SHOWTBAR:
                if (!IsFullScreen()) {
                    sys->cfg->Write(L"showtoolbar", sys->showtoolbar = checked);
                    #ifdef __WXWIDGETS__
                    auto wtb = GetToolBar();
                    wtb->Show(sys->showtoolbar);
                    SendSizeEvent();
                    Refresh();
                    wtb->Refresh();
                    #endif
                }
                break;
            case A_CUSTCOL: {
                if (auto color = sys->dialogs->PickColor(sys->customcolor); color != (uint)-1)
                    sys->cfg->Write(L"customcolor", sys->customcolor = color);
                break;
            }

            case A_ADDSCRIPT: {
                wxArrayString filenames;
                sys->dialogs->GetFilesFromUser(filenames, _(L"Please select Lobster script file(s):"),
                                 _(L"Lobster Files (*.lobster)|*.lobster|All Files (*.*)|*.*"));
                for (auto &filename : filenames) scripts.AddFileToHistory(filename);
                break;
            }

            case A_DETSCRIPT: {
                wxArrayString filenames;
                for (int i = 0, n = scripts.GetCount(); i < n; i++) {
                    filenames.Add(scripts.GetHistoryFile(i));
                }
                int sel = sys->dialogs->SingleChoice(_(L"Remove script from list..."), _(L"Please select the script you want to remove from the list:"), filenames);
                if (sel >= 0) scripts.RemoveFileFromHistory(sel);
                break;
            }

            case A_DEFAULTMAXCOLWIDTH: {
                int w = (int)sys->dialogs->AskNumber(_(L"Please enter the default column width:"), _(L"Default column width"), sys->defaultmaxcolwidth, 1, 1000);
                if (w > 0) sys->cfg->Write(L"defaultmaxcolwidth", sys->defaultmaxcolwidth = w);
                break;
            }

            case A_LEFTTABS: Check(L"lefttabs"); break;
            case A_SINGLETRAY: Check(L"singletray"); break;
            case A_MAKEBAKS: sys->cfg->Write(L"makebaks", sys->makebaks = checked); break;
            case A_TOTRAY: sys->cfg->Write(L"totray", sys->totray = checked); break;
            case A_MINCLOSE: sys->cfg->Write(L"minclose", sys->minclose = checked); break;
            case A_ZOOMSCR: sys->cfg->Write(L"zoomscroll", sys->zoomscroll = checked); break;
            case A_THINSELC: sys->cfg->Write(L"thinselc", sys->thinselc = checked); break;
            case A_AUTOSAVE: sys->cfg->Write(L"autosave", sys->autosave = checked); break;
            case A_CENTERED:
                sys->cfg->Write(L"centered", sys->centered = checked);
                Refresh();
                break;
            case A_FSWATCH:
                Check(L"fswatch");
                sys->fswatch = checked;
                break;
            case A_AUTOEXPORT_HTML_NONE:
            case A_AUTOEXPORT_HTML_WITH_IMAGES:
            case A_AUTOEXPORT_HTML_WITHOUT_IMAGES:
                sys->cfg->Write(
                    L"autohtmlexport",
                    static_cast<long>(sys->autohtmlexport = id - A_AUTOEXPORT_HTML_NONE));
                break;
            case A_FASTRENDER:
                sys->cfg->Write(L"fastrender", sys->fastrender = checked);
                Refresh();
                break;
            case A_INVERTRENDER:
                sys->cfg->Write(L"followdarkmode", sys->followdarkmode = checked);
                sys->darkmode = sys->followdarkmode && wxSystemSettings::GetAppearance().IsDark();
                Refresh();
                break;
            case A_FULLSCREEN:
                ShowFullScreen(!IsFullScreen());
                if (IsFullScreen()) SetStatus(_(L"Press F11 to exit fullscreen mode."));
                break;
            case wxID_FIND:
                if (filter) {
                    filter->SetFocus();
                    filter->SetSelection(0, 1000);
                } else {
                    SetStatus(_(L"Please enable (Options -> Show Toolbar) to use search."));
                }
                break;
            case wxID_REPLACE:
                if (replaces) {
                    replaces->SetFocus();
                    replaces->SetSelection(0, 1000);
                } else {
                    SetStatus(_(L"Please enable (Options -> Show Toolbar) to use replace."));
                }
                break;
            #ifdef __WXMAC__
                case wxID_OSX_HIDE: Iconize(true); break;
                case wxID_OSX_HIDEOTHERS: SetStatus(L"NOT IMPLEMENTED"); break;
                case wxID_OSX_SHOWALL: Iconize(false); break;
                case wxID_ABOUT: canvas->doc->Action(wxID_ABOUT); break;
                case wxID_PREFERENCES: canvas->doc->Action(wxID_SELECT_FONT); break;
            #endif
            case wxID_EXIT:
                fromclosebox = false;
                Close();
                break;
            case wxID_CLOSE:
                canvas->doc->Action(id);
                break;  // canvas dangling pointer on return
            default:
                if (id >= wxID_FILE1 && id <= wxID_FILE9) {
                    wxString filename(filehistory.GetHistoryFile(id - wxID_FILE1));
                    SetStatus(sys->Open(filename));
                } else if (id >= A_TAGSET && id < A_SCRIPT) {
                    SetStatus(canvas->doc->TagSet(id - A_TAGSET));
                } else if (id >= A_SCRIPT && id < A_MAXACTION) {
                    auto message =
                        tssi.ScriptRun(scripts.GetHistoryFile(id - A_SCRIPT).c_str());
                    message.erase(std::remove(message.begin(), message.end(), '\n'), message.end());
                    SetStatus(wxString(message));
                } else {
                    SetStatus(canvas->doc->Action(id));
                    break;
                }
        }
    }
