    void ConstructToolBar() {
        toolbar = CreateToolBar(wxBORDER_NONE | wxTB_HORIZONTAL | wxTB_FLAT | wxTB_NODIVIDER);
        toolbar->SetOwnBackgroundColour(toolbarbackgroundcolor);

        #ifdef __WXMAC__
        #define SEPARATOR
        #else
        #define SEPARATOR toolbar->AddSeparator()
        #endif

        auto iconpath = app->GetDataPath(L"images/material/toolbar/");

        auto AddToolbarIcon = [&](const wxString& name, int action, wxString iconpath,
                                  wxString lighticon, wxString darkicon) {
            toolbar->AddTool(
                action, name,
                wxBitmapBundle::FromSVGFile(
                    iconpath + (wxSystemSettings::GetAppearance().IsDark() ? darkicon : lighticon),
                    wxSize(24, 24)),
                name, wxITEM_NORMAL);
        };

        AddToolbarIcon(_(L"New (CTRL+n)"), wxID_NEW, iconpath, L"filenew.svg", L"filenew_dark.svg");
        AddToolbarIcon(_(L"Open (CTRL+o)"), wxID_OPEN, iconpath, L"fileopen.svg",
                       L"fileopen_dark.svg");
        AddToolbarIcon(_(L"Save (CTRL+s)"), wxID_SAVE, iconpath, L"filesave.svg",
                       L"filesave_dark.svg");
        AddToolbarIcon(_(L"Save as..."), wxID_SAVEAS, iconpath, L"filesaveas.svg",
                       L"filesaveas_dark.svg");
        SEPARATOR;
        AddToolbarIcon(_(L"Undo (CTRL+z)"), wxID_UNDO, iconpath, L"undo.svg", L"undo_dark.svg");
        AddToolbarIcon(_(L"Copy (CTRL+c)"), wxID_COPY, iconpath, L"editcopy.svg",
                       L"editcopy_dark.svg");
        AddToolbarIcon(_(L"Paste (CTRL+v)"), wxID_PASTE, iconpath, L"editpaste.svg",
                       L"editpaste_dark.svg");
        SEPARATOR;
        AddToolbarIcon(_(L"Zoom In (CTRL+mousewheel)"), A_ZOOMIN, iconpath, L"zoomin.svg",
                       L"zoomin_dark.svg");
        AddToolbarIcon(_(L"Zoom Out (CTRL+mousewheel)"), A_ZOOMOUT, iconpath, L"zoomout.svg",
                       L"zoomout_dark.svg");
        SEPARATOR;
        AddToolbarIcon(_(L"New Grid (INS)"), A_NEWGRID, iconpath, L"newgrid.svg",
                       L"newgrid_dark.svg");
        AddToolbarIcon(_(L"Add Image"), A_IMAGE, iconpath, L"image.svg", L"image_dark.svg");
        SEPARATOR;
        AddToolbarIcon(_(L"Run"), wxID_EXECUTE, iconpath, L"run.svg", L"run_dark.svg");
        toolbar->AddSeparator();
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Search ")));
        toolbar->AddControl(filter = new wxTextCtrl(toolbar, A_SEARCH, "", wxDefaultPosition,
                                                    FromDIP(wxSize(80, 22)),
                                                    wxWANTS_CHARS | wxTE_PROCESS_ENTER));
        AddToolbarIcon(_(L"Clear search"), A_CLEARSEARCH, iconpath, L"cancel.svg",
                       L"cancel_dark.svg");
        AddToolbarIcon(_(L"Go to Next Search Result"), A_SEARCHNEXT, iconpath, L"search.svg",
                       L"search_dark.svg");
        SEPARATOR;
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Replace ")));
        toolbar->AddControl(replaces = new wxTextCtrl(toolbar, A_REPLACE, "", wxDefaultPosition,
                                                      FromDIP(wxSize(80, 22)),
                                                      wxWANTS_CHARS | wxTE_PROCESS_ENTER));
        AddToolbarIcon(_(L"Clear replace"), A_CLEARREPLACE, iconpath, L"cancel.svg",
                       L"cancel_dark.svg");
        AddToolbarIcon(_(L"Replace in selection"), A_REPLACEONCE, iconpath, L"replace.svg",
                       L"replace_dark.svg");
        AddToolbarIcon(_(L"Replace All"), A_REPLACEALL, iconpath, L"replaceall.svg",
                       L"replaceall_dark.svg");
        toolbar->AddSeparator();
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Cell ")));
        cellcolordropdown = new ColorDropdown(toolbar, A_CELLCOLOR, 1);
        toolbar->AddControl(cellcolordropdown);
        SEPARATOR;
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Text ")));
        textcolordropdown = new ColorDropdown(toolbar, A_TEXTCOLOR, 2);
        toolbar->AddControl(textcolordropdown);
        SEPARATOR;
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Border ")));
        bordercolordropdown = new ColorDropdown(toolbar, A_BORDCOLOR, 7);
        toolbar->AddControl(bordercolordropdown);
        toolbar->AddSeparator();
        toolbar->AddControl(new wxStaticText(toolbar, wxID_ANY, _(L"Image ")));
        imagedropdown = new ImageDropdown(toolbar, imagepath);
        toolbar->AddControl(imagedropdown);
        toolbar->Realize();
        toolbar->Show(sys->showtoolbar);

        #undef SEPARATOR
    }
