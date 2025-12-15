
    void MyAppend(TSMenu *menu, int tag, const wxString &contents, const wxString &help = wxEmptyString) {
        auto item = contents;
        wxString key = L"";
        if (int pos = contents.Find("\t"); pos >= 0) {
            item = contents.Mid(0, pos);
            key = contents.Mid(pos + 1);
        }
        key = sys->cfg->Read(item, key);
        auto newcontents = item;
        if (key.Length()) newcontents += "\t" + key;
        menu->Append(tag, newcontents, help);
        menustrings[item] = key;
    }

    void CreateMenus(bool lefttabs) {
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

        auto expmenu = new TSPlatformMenu();
        MyAppend(expmenu, A_EXPXML, _(L"&XML..."),
                 _(L"Export the current view as XML (which can also be reimported without losing structure)"));
        MyAppend(expmenu, A_EXPHTMLT, _(L"&HTML (Tables+Styling)..."),
                 _(L"Export the current view as HTML using nested tables, that will look somewhat like the TreeSheet"));
        MyAppend(expmenu, A_EXPHTMLTE, _(L"&HTML (Tables+Styling+Images)..."),
                 _(L"Export the curent view as HTML using nested tables and exported images"));
        MyAppend(expmenu, A_EXPHTMLB, _(L"HTML (&Bullet points)..."),
                 _(L"Export the current view as HTML as nested bullet points."));
        MyAppend(expmenu, A_EXPHTMLO, _(L"HTML (&Outline)..."),
                 _(L"Export the current view as HTML as nested headers, suitable for importing into Word's outline mode"));
        MyAppend(
            expmenu, A_EXPTEXT, _(L"Indented &Text..."),
            _(L"Export the current view as tree structured text, using spaces for each indentation level. Suitable for importing into mindmanagers and general text programs"));
        MyAppend(
            expmenu, A_EXPCSV, _(L"&Comma delimited text (CSV)..."),
            _(L"Export the current view as CSV. Good for spreadsheets and databases. Only works on grids with no sub-grids (use the Flatten operation first if need be)"));
        MyAppend(expmenu, A_EXPIMAGE, _(L"&Image..."),
                 _(L"Export the current view as an image. Useful for faithful renderings of the TreeSheet, and programs that don't accept any of the above options"));

        auto impmenu = new TSPlatformMenu();
        MyAppend(impmenu, A_IMPXML, _(L"XML..."));
        MyAppend(impmenu, A_IMPXMLA, _(L"XML (attributes too, for OPML etc)..."));
        MyAppend(impmenu, A_IMPTXTI, _(L"Indented text..."));
        MyAppend(impmenu, A_IMPTXTC, _(L"Comma delimited text (CSV)..."));
        MyAppend(impmenu, A_IMPTXTS, _(L"Semi-Colon delimited text (CSV)..."));
        MyAppend(impmenu, A_IMPTXTT, _(L"Tab delimited text..."));

        auto recentmenu = new TSPlatformMenu();
        FileHistoryUseMenu(recentmenu);
        filehistory.AddFilesToMenu();

        auto filemenu = new TSPlatformMenu();
        MyAppend(filemenu, wxID_NEW, _(L"&New") + L"\tCTRL+N", _(L"Create a new document"));
        MyAppend(filemenu, wxID_OPEN, _(L"&Open...") + L"\tCTRL+O",
                 _(L"Open an existing document"));
        MyAppend(filemenu, wxID_CLOSE, _(L"&Close") + L"\tCTRL+W", _(L"Close current document"));
        filemenu->AppendSubMenu(recentmenu, _(L"&Recent files"));
        MyAppend(filemenu, wxID_SAVE, _(L"&Save") + L"\tCTRL+S", _(L"Save current document"));
        MyAppend(filemenu, wxID_SAVEAS, _(L"Save &As..."),
                 _(L"Save current document with a different filename"));
        MyAppend(filemenu, A_SAVEALL, _(L"Save All"));
        filemenu->AppendSeparator();
        MyAppend(filemenu, A_PAGESETUP, _(L"Page Setup..."));
        MyAppend(filemenu, A_PRINTSCALE, _(L"Set Print Scale..."));
        MyAppend(filemenu, wxID_PREVIEW, _(L"Print preview..."));
        MyAppend(filemenu, wxID_PRINT, _(L"&Print...") + L"\tCTRL+P");
        filemenu->AppendSeparator();
        filemenu->AppendSubMenu(expmenu, _(L"Export &view as"));
        filemenu->AppendSubMenu(impmenu, _(L"Import from"));
        filemenu->AppendSeparator();
        MyAppend(filemenu, wxID_EXIT, _(L"&Exit") + L"\tCTRL+Q", _(L"Quit this program"));

        TSMenu *editmenu;
        loop(twoeditmenus, 2) {
            auto sizemenu = new TSPlatformMenu();
            MyAppend(sizemenu, A_INCSIZE,
                     _(L"&Increase text size (SHIFT+mousewheel)") + L"\tSHIFT+PGUP");
            MyAppend(sizemenu, A_DECSIZE,
                     _(L"&Decrease text size (SHIFT+mousewheel)") + L"\tSHIFT+PGDN");
            MyAppend(sizemenu, A_RESETSIZE, _(L"&Reset text sizes") + L"\tCTRL+SHIFT+S");
            MyAppend(sizemenu, A_MINISIZE, _(L"&Shrink text of all sub-grids") + L"\tCTRL+SHIFT+M");
            sizemenu->AppendSeparator();
            MyAppend(sizemenu, A_INCWIDTH,
                     _(L"Increase column width (ALT+mousewheel)") + L"\tALT+PGUP");
            MyAppend(sizemenu, A_DECWIDTH,
                     _(L"Decrease column width (ALT+mousewheel)") + L"\tALT+PGDN");
            MyAppend(sizemenu, A_INCWIDTHNH,
                     _(L"Increase column width (no sub grids)") + L"\tCTRL+ALT+PGUP");
            MyAppend(sizemenu, A_DECWIDTHNH,
                     _(L"Decrease column width (no sub grids)") + L"\tCTRL+ALT+PGDN");
            MyAppend(sizemenu, A_RESETWIDTH, _(L"Reset column widths") + L"\tCTRL+R",
                     _(L"Reset the column widths in the selection to the default column width"));

            auto bordmenu = new TSPlatformMenu();
            MyAppend(bordmenu, A_BORD0, _(L"Border &0") + L"\tCTRL+SHIFT+9");
            MyAppend(bordmenu, A_BORD1, _(L"Border &1") + L"\tCTRL+SHIFT+1");
            MyAppend(bordmenu, A_BORD2, _(L"Border &2") + L"\tCTRL+SHIFT+2");
            MyAppend(bordmenu, A_BORD3, _(L"Border &3") + L"\tCTRL+SHIFT+3");
            MyAppend(bordmenu, A_BORD4, _(L"Border &4") + L"\tCTRL+SHIFT+4");
            MyAppend(bordmenu, A_BORD5, _(L"Border &5") + L"\tCTRL+SHIFT+5");

            auto selmenu = new TSPlatformMenu();
            MyAppend(selmenu, A_NEXT,
                #ifdef __WXGTK__
                    _(L"Move to next cell (TAB)")
                #else
                    _(L"Move to next cell") + L"\tTAB"
                #endif
            );
            MyAppend(selmenu, A_PREV,
                #ifdef __WXGTK__
                    _(L"Move to previous cell (SHIFT+TAB)")
                #else
                    _(L"Move to previous cell") + L"\tSHIFT+TAB"
                #endif
            );
            selmenu->AppendSeparator();
            MyAppend(selmenu, wxID_SELECTALL, _(L"Select &all in current grid/cell") + L"\tCTRL+A");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_LEFT,
                #ifdef __WXGTK__
                    _(L"Move Selection Left (LEFT)")
                #else
                    _(L"Move Selection Left") + L"\tLEFT"
                #endif
            );
            MyAppend(selmenu, A_RIGHT,
                #ifdef __WXGTK__
                    _(L"Move Selection Right (RIGHT)")
                #else
                    _(L"Move Selection Right") + L"\tRIGHT"
                #endif
            );
            MyAppend(selmenu, A_UP,
                #ifdef __WXGTK__
                    _(L"Move Selection Up (UP)")
                #else
                    _(L"Move Selection Up") + L"\tUP"
                #endif
            );
            MyAppend(selmenu, A_DOWN,
                #ifdef __WXGTK__
                    _(L"Move Selection Down (DOWN)")
                #else
                    _(L"Move Selection Down") + L"\tDOWN"
                #endif
            );
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_MLEFT, _(L"Move Cells Left") + L"\tCTRL+LEFT");
            MyAppend(selmenu, A_MRIGHT, _(L"Move Cells Right") + L"\tCTRL+RIGHT");
            MyAppend(selmenu, A_MUP, _(L"Move Cells Up") + L"\tCTRL+UP");
            MyAppend(selmenu, A_MDOWN, _(L"Move Cells Down") + L"\tCTRL+DOWN");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_SLEFT, _(L"Extend Selection Left") + L"\tSHIFT+LEFT");
            MyAppend(selmenu, A_SRIGHT, _(L"Extend Selection Right") + L"\tSHIFT+RIGHT");
            MyAppend(selmenu, A_SUP, _(L"Extend Selection Up") + L"\tSHIFT+UP");
            MyAppend(selmenu, A_SDOWN, _(L"Extend Selection Down") + L"\tSHIFT+DOWN");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_SROWS, _(L"Extend Selection Full Rows"));
            MyAppend(selmenu, A_SCLEFT, _(L"Extend Selection Rows Left") + L"\tCTRL+SHIFT+LEFT");
            MyAppend(selmenu, A_SCRIGHT, _(L"Extend Selection Rows Right") + L"\tCTRL+SHIFT+RIGHT");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_SCOLS, _(L"Extend Selection Full Columns") + L"\tCTRL+SHIFT+a");
            MyAppend(selmenu, A_SCUP, _(L"Extend Selection Columns Up") + L"\tCTRL+SHIFT+UP");
            MyAppend(selmenu, A_SCDOWN, _(L"Extend Selection Columns Down") + L"\tCTRL+SHIFT+DOWN");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_CANCELEDIT, _(L"Select &Parent") + L"\tESC");
            MyAppend(selmenu, A_ENTERGRID, _(L"Select First &Child") + L"\tSHIFT+ENTER");
            selmenu->AppendSeparator();
            MyAppend(selmenu, A_LINK, _(L"Go To &Matching Cell (Text)") + L"\tF6");
            MyAppend(selmenu, A_LINKREV, _(L"Go To Matching Cell (Text, Reverse)") + L"\tSHIFT+F6");
            MyAppend(selmenu, A_LINKIMG, _(L"Go To Matching Cell (Image)") + L"\tF7");
            MyAppend(selmenu, A_LINKIMGREV,
                     _(L"Go To Matching Cell (Image, Reverse)") + L"\tSHIFT+F7");

            auto temenu = new TSPlatformMenu();
            MyAppend(temenu, A_LEFT, _(L"Cursor Left") + L"\tLEFT");
            MyAppend(temenu, A_RIGHT, _(L"Cursor Right") + L"\tRIGHT");
            MyAppend(temenu, A_MLEFT, _(L"Word Left") + L"\tCTRL+LEFT");
            MyAppend(temenu, A_MRIGHT, _(L"Word Right") + L"\tCTRL+RIGHT");
            temenu->AppendSeparator();
            MyAppend(temenu, A_SLEFT, _(L"Extend Selection Left") + L"\tSHIFT+LEFT");
            MyAppend(temenu, A_SRIGHT, _(L"Extend Selection Right") + L"\tSHIFT+RIGHT");
            MyAppend(temenu, A_SCLEFT, _(L"Extend Selection Word Left") + L"\tCTRL+SHIFT+LEFT");
            MyAppend(temenu, A_SCRIGHT, _(L"Extend Selection Word Right") + L"\tCTRL+SHIFT+RIGHT");
            MyAppend(temenu, A_SHOME, _(L"Extend Selection to Start") + L"\tSHIFT+HOME");
            MyAppend(temenu, A_SEND, _(L"Extend Selection to End") + L"\tSHIFT+END");
            temenu->AppendSeparator();
            MyAppend(temenu, A_HOME, _(L"Start of line of text") + L"\tHOME");
            MyAppend(temenu, A_END, _(L"End of line of text") + L"\tEND");
            MyAppend(temenu, A_CHOME, _(L"Start of text") + L"\tCTRL+HOME");
            MyAppend(temenu, A_CEND, _(L"End of text") + L"\tCTRL+END");
            temenu->AppendSeparator();
            MyAppend(temenu, A_ENTERCELL, _(L"Enter/exit text edit mode") + L"\tENTER");
            MyAppend(temenu, A_ENTERCELL_JUMPTOEND,
                     _(L"...and jump to the end of the text") + L"\tF2");
            MyAppend(
                temenu, A_ENTERCELL_JUMPTOSTART,
                _(L"...and progress to the first cell in the new row") + L"\t" ALTORCTRL "+ENTER");
            MyAppend(
                temenu, A_PROGRESSCELL,
                _(L"...and progress to the next cell on the right") + L"\t" CTRLORALT "+ENTER");
            MyAppend(temenu, A_CANCELEDIT, _(L"Cancel text edits") + "\tESC");

            auto stmenu = new TSPlatformMenu();
            MyAppend(stmenu, wxID_BOLD, _(L"Toggle cell &BOLD") + L"\tCTRL+B");
            MyAppend(stmenu, wxID_ITALIC, _(L"Toggle cell &ITALIC") + L"\tCTRL+I");
            MyAppend(stmenu, A_TT, _(L"Toggle cell &typewriter") + L"\tCTRL+ALT+T");
            MyAppend(stmenu, wxID_UNDERLINE, _(L"Toggle cell &underlined") + L"\tCTRL+U");
            MyAppend(stmenu, wxID_STRIKETHROUGH, _(L"Toggle cell &strikethrough") + L"\tCTRL+T");
            stmenu->AppendSeparator();
            MyAppend(stmenu, A_RESETSTYLE, _(L"&Reset text styles") + L"\tCTRL+SHIFT+R");
            MyAppend(stmenu, A_RESETCOLOR, _(L"Reset &colors") + L"\tCTRL+SHIFT+C");
            stmenu->AppendSeparator();
            MyAppend(stmenu, A_LASTCELLCOLOR, _(L"Apply last cell color") + L"\tSHIFT+ALT+C");
            MyAppend(stmenu, A_LASTTEXTCOLOR, _(L"Apply last text color") + L"\tSHIFT+ALT+T");
            MyAppend(stmenu, A_LASTBORDCOLOR, _(L"Apply last border color") + L"\tSHIFT+ALT+B");
            MyAppend(stmenu, A_OPENCELLCOLOR, _(L"Open cell colors") + L"\tSHIFT+ALT+F9");
            MyAppend(stmenu, A_OPENTEXTCOLOR, _(L"Open text colors") + L"\tSHIFT+ALT+F10");
            MyAppend(stmenu, A_OPENBORDCOLOR, _(L"Open border colors") + L"\tSHIFT+ALT+F11");
            MyAppend(stmenu, A_OPENIMGDROPDOWN, _(L"Open image dropdown") + L"\tSHIFT+ALT+F12");

            auto tagmenu = new TSPlatformMenu();
            MyAppend(tagmenu, A_TAGADD, _(L"&Add Cell Text as Tag"));
            MyAppend(tagmenu, A_TAGREMOVE, _(L"&Remove Cell Text from Tags"));
            MyAppend(tagmenu, A_NOP, _(L"&Set Cell Text to tag (use CTRL+RMB)"),
                     _(L"Hold CTRL while pressing right mouse button to quickly set a tag for the current cell using a popup menu"));

            auto orgmenu = new TSPlatformMenu();
            MyAppend(orgmenu, A_TRANSPOSE, _(L"&Transpose") + L"\tCTRL+SHIFT+T",
                     _(L"changes the orientation of a grid"));
            MyAppend(orgmenu, A_SORT, _(L"Sort &Ascending"),
                     _(L"Make a 1xN selection to indicate which column to sort on, and which rows to affect"));
            MyAppend(orgmenu, A_SORTD, _(L"Sort &Descending"),
                     _(L"Make a 1xN selection to indicate which column to sort on, and which rows to affect"));
            MyAppend(orgmenu, A_HSWAP, _(L"Hierarchy &Swap") + L"\tF8",
                     _(L"Swap all cells with this text at this level (or above) with the parent"));
            MyAppend(orgmenu, A_HIFY, _(L"&Hierarchify"),
                     _(L"Convert an NxN grid with repeating elements per column into an 1xN grid with hierarchy, useful to convert data from spreadsheets"));
            MyAppend(orgmenu, A_FLATTEN, _(L"&Flatten"),
                     _(L"Takes a hierarchy (nested 1xN or Nx1 grids) and converts it into a flat NxN grid, useful for export to spreadsheets"));

            auto imgmenu = new TSPlatformMenu();
            MyAppend(imgmenu, A_IMAGE, _(L"&Add..."), _(L"Add an image to the selected cell"));
            MyAppend(imgmenu, A_IMAGESVA, _(L"&Save as..."),
                     _(L"Save image(s) from selected cell(s) to disk. Multiple images will be saved with a counter appended to each file name."));
            imgmenu->AppendSeparator();
            MyAppend(
                imgmenu, A_IMAGESCP, _(L"Scale (re-sa&mple pixels, by %)"),
                _(L"Change the image(s) size if it is too big, by reducing the amount of pixels"));
            MyAppend(
                imgmenu, A_IMAGESCW, _(L"Scale (re-sample pixels, by &width)"),
                _(L"Change the image(s) size if it is too big, by reducing the amount of pixels"));
            MyAppend(imgmenu, A_IMAGESCF, _(L"Scale (&display only)"),
                     _(L"Change the image(s) size if it is too big or too small, by changing the size shown on screen. Applies to all uses of this image."));
            MyAppend(imgmenu, A_IMAGESCN, _(L"&Reset Scale (display only)"),
                     _(L"Change the image(s) scale to match DPI of the current display. Applies to all uses of this image."));
            imgmenu->AppendSeparator();
            MyAppend(
                imgmenu, A_SAVE_AS_JPEG, _(L"Embed as &JPEG"),
                _(L"Embed the image(s) in the selected cells in JPEG format (reduces data size)"));
            MyAppend(imgmenu, A_SAVE_AS_PNG, _(L"Embed as &PNG"),
                     _(L"Embed the image(s) in the selected cells in PNG format (default)"));
            imgmenu->AppendSeparator();
            MyAppend(imgmenu, A_LASTIMAGE, _(L"Insert last image") + L"\tSHIFT+ALT+i",
                     _(L"Insert the last image that has been inserted before in TreeSheets."));
            MyAppend(imgmenu, A_IMAGER, _(L"Remo&ve"),
                     _(L"Remove image(s) from the selected cells"));

            auto navmenu = new TSPlatformMenu();
            MyAppend(navmenu, A_BROWSE, _(L"Open link in &browser") + L"\tF5",
                     _(L"Opens up the text from the selected cell in browser (should start be a valid URL)"));
            MyAppend(navmenu, A_BROWSEF, _(L"Open &file") + "\tF4",
                     _(L"Opens up the text from the selected cell in default application for the file type"));

            auto laymenu = new TSPlatformMenu();
            MyAppend(laymenu, A_V_GS,
                     _(L"Vertical Layout with Grid Style Rendering") + L"\t" CTRLORALT "+1");
            MyAppend(laymenu, A_V_BS,
                     _(L"Vertical Layout with Bubble Style Rendering") + L"\t" CTRLORALT "+2");
            MyAppend(laymenu, A_V_LS,
                     _(L"Vertical Layout with Line Style Rendering") + L"\t" CTRLORALT "+3");
            laymenu->AppendSeparator();
            MyAppend(laymenu, A_H_GS,
                     _(L"Horizontal Layout with Grid Style Rendering") + L"\t" CTRLORALT "+4");
            MyAppend(laymenu, A_H_BS,
                     _(L"Horizontal Layout with Bubble Style Rendering") + L"\t" CTRLORALT "+5");
            MyAppend(laymenu, A_H_LS,
                     _(L"Horizontal Layout with Line Style Rendering") + L"\t" CTRLORALT "+6");
            laymenu->AppendSeparator();
            MyAppend(laymenu, A_GS, _(L"Grid Style Rendering") + L"\t" CTRLORALT "+7");
            MyAppend(laymenu, A_BS, _(L"Bubble Style Rendering") + L"\t" CTRLORALT "+8");
            MyAppend(laymenu, A_LS, _(L"Line Style Rendering") + L"\t" CTRLORALT "+9");
            laymenu->AppendSeparator();
            MyAppend(laymenu, A_TEXTGRID, _(L"Toggle Vertical Layout") + L"\t" CTRLORALT "+0",
                     _(L"Make a hierarchy layout more vertical (default) or more horizontal"));

            editmenu = new TSPlatformMenu();
            MyAppend(editmenu, wxID_CUT, _(L"Cu&t") + L"\tCTRL+X", _(L"Cut selection"));
            MyAppend(editmenu, wxID_COPY, _(L"&Copy") + L"\tCTRL+C", _(L"Copy selection"));
            editmenu->AppendSeparator();
            MyAppend(editmenu, A_COPYWI, _(L"Copy with &Images") + L"\tCTRL+ALT+C");
            MyAppend(editmenu, A_COPYBM, _(L"&Copy as Bitmap"));
            MyAppend(editmenu, A_COPYCT, _(L"Copy As Continuous Text"));
            editmenu->AppendSeparator();
            MyAppend(editmenu, wxID_PASTE, _(L"&Paste") + L"\tCTRL+V",
                     _(L"Paste clipboard contents"));
            MyAppend(editmenu, A_PASTESTYLE, _(L"Paste Style Only") + L"\tCTRL+SHIFT+V",
                     _(L"only sets the colors and style of the copied cell, and keeps the text"));
            MyAppend(editmenu, A_COLLAPSE, _(L"Collapse Ce&lls") + L"\tCTRL+L");
            editmenu->AppendSeparator();
            MyAppend(editmenu, wxID_UNDO, _(L"&Undo") + L"\tCTRL+Z",
                     _(L"revert the changes, one step at a time"));
            MyAppend(editmenu, wxID_REDO, _(L"&Redo") + L"\tCTRL+Y",
                     _(L"redo any undo steps, if you haven't made changes since"));
            editmenu->AppendSeparator();
            MyAppend(
                editmenu, A_DELETE, _(L"&Delete After") + L"\tDEL",
                _(L"Deletes the column of cells after the selected grid line, or the row below"));
            MyAppend(
                editmenu, A_BACKSPACE, _(L"Delete Before") + L"\tBACK",
                _(L"Deletes the column of cells before the selected grid line, or the row above"));
            MyAppend(editmenu, A_DELETE_WORD, _(L"Delete Word After") + L"\tCTRL+DEL",
                     _(L"Deletes the entire word after the cursor"));
            MyAppend(editmenu, A_BACKSPACE_WORD, _(L"Delete Word Before") + L"\tCTRL+BACK",
                     _(L"Deletes the entire word before the cursor"));
            editmenu->AppendSeparator();
            MyAppend(editmenu, A_NEWGRID,
                     #ifdef __WXMAC__
                     _(L"&Insert New Grid") + L"\tCTRL+G",
                     #else
                     _(L"&Insert New Grid") + L"\tINS",
                     #endif
                     _(L"Adds a grid to the selected cell"));
            MyAppend(editmenu, A_WRAP, _(L"&Wrap in new parent") + L"\tF9",
                     _(L"Creates a new level of hierarchy around the current selection"));
            editmenu->AppendSeparator();
            // F10 is tied to the OS on both Ubuntu and OS X, and SHIFT+F10 is now right
            // click on all platforms?
            MyAppend(editmenu, A_FOLD,
                     #ifndef WIN32
                     _(L"Toggle Fold") + L"\tCTRL+F10",
                     #else
                     _(L"Toggle Fold") + L"\tF10",
                     #endif
                    _("Toggles showing the grid of the selected cell(s)"));
            MyAppend(editmenu, A_FOLDALL, _(L"Fold All") + L"\tCTRL+SHIFT+F10",
                _(L"Folds the grid of the selected cell(s) recursively"));
            MyAppend(editmenu, A_UNFOLDALL, _(L"Unfold All") + L"\tCTRL+ALT+F10",
                _(L"Unfolds the grid of the selected cell(s) recursively"));
            editmenu->AppendSeparator();
            editmenu->AppendSubMenu(selmenu, _(L"&Selection"));
            editmenu->AppendSubMenu(orgmenu, _(L"&Grid Reorganization"));
            editmenu->AppendSubMenu(laymenu, _(L"&Layout && Render Style"));
            editmenu->AppendSubMenu(imgmenu, _(L"&Images"));
            editmenu->AppendSubMenu(navmenu, _(L"&Browsing"));
            editmenu->AppendSubMenu(temenu, _(L"Text &Editing"));
            editmenu->AppendSubMenu(sizemenu, _(L"Text Sizing"));
            editmenu->AppendSubMenu(stmenu, _(L"Text Style"));
            editmenu->AppendSubMenu(bordmenu, _(L"Set Grid Border Width"));
            editmenu->AppendSubMenu(tagmenu, _(L"Tag"));

            if (!twoeditmenus) editmenupopup = editmenu;
        }

        auto semenu = new TSPlatformMenu();
        MyAppend(semenu, wxID_FIND, _(L"&Search") + L"\tCTRL+F", _(L"Find in document"));
        semenu->AppendCheckItem(A_CASESENSITIVESEARCH, _(L"Case-sensitive search"));
        semenu->Check(A_CASESENSITIVESEARCH, sys->casesensitivesearch);
        semenu->AppendSeparator();
        MyAppend(semenu, A_SEARCHNEXT, _(L"&Next Match") + L"\tF3", _(L"Go to next search match"));
        MyAppend(semenu, A_SEARCHPREV, _(L"&Previous Match") + L"\tSHIFT+F3",
                 _(L"Go to previous search match"));
        semenu->AppendSeparator();
        MyAppend(semenu, wxID_REPLACE, _(L"&Replace") + L"\tCTRL+H",
                 _(L"Find and replace in document"));
        MyAppend(semenu, A_REPLACEONCE, _(L"Replace in Current &Selection") + L"\tCTRL+K");
        MyAppend(semenu, A_REPLACEONCEJ,
                 _(L"Replace in Current Selection && &Jump Next") + L"\tCTRL+J");
        MyAppend(semenu, A_REPLACEALL, _(L"Replace &All"));

        auto scrollmenu = new TSPlatformMenu();
        MyAppend(scrollmenu, A_AUP, _(L"Scroll Up (mousewheel)") + L"\tPGUP");
        MyAppend(scrollmenu, A_AUP, _(L"Scroll Up (mousewheel)") + L"\tALT+UP");
        MyAppend(scrollmenu, A_ADOWN, _(L"Scroll Down (mousewheel)") + L"\tPGDN");
        MyAppend(scrollmenu, A_ADOWN, _(L"Scroll Down (mousewheel)") + L"\tALT+DOWN");
        MyAppend(scrollmenu, A_ALEFT, _(L"Scroll Left") + L"\tALT+LEFT");
        MyAppend(scrollmenu, A_ARIGHT, _(L"Scroll Right") + L"\tALT+RIGHT");

        auto filtermenu = new TSPlatformMenu();
        MyAppend(filtermenu, A_FILTEROFF, _(L"Turn filter &off") + L"\tCTRL+SHIFT+F");
        MyAppend(filtermenu, A_FILTERS, _(L"Show only cells in current search"));
        MyAppend(filtermenu, A_FILTERRANGE, _(L"Show last edits in specific date range"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTER5, _(L"Show 5% of last edits"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTER10, _(L"Show 10% of last edits"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTER20, _(L"Show 20% of last edits"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTER50, _(L"Show 50% of last edits"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTERM, _(L"Show 1% more than the last filter"));
        // xgettext:no-c-format
        MyAppend(filtermenu, A_FILTERL, _(L"Show 1% less than the last filter"));
        MyAppend(filtermenu, A_FILTERBYCELLBG, _(L"Filter by the same cell color"));
        MyAppend(filtermenu, A_FILTERMATCHNEXT, _(L"Go to next filter match") + L"\tCTRL+F3");

        auto viewmenu = new TSPlatformMenu();
        MyAppend(viewmenu, A_ZOOMIN, _(L"Zoom &In (CTRL+mousewheel)") + L"\tCTRL+PGUP");
        MyAppend(viewmenu, A_ZOOMOUT, _(L"Zoom &Out (CTRL+mousewheel)") + L"\tCTRL+PGDN");
        viewmenu->AppendSeparator();
        MyAppend(
            viewmenu, A_NEXTFILE,
            _(L"&Next tab")
                 #ifndef __WXGTK__
                    // On Linux, this conflicts with CTRL+I, see Document::Key()
                    // CTRL+SHIFT+TAB below still works, so that will have to be used to switch tabs.
                     + L"\tCTRL+TAB"
                 #endif
            ,
            _(L"Go to the document in the next tab"));
        MyAppend(viewmenu, A_PREVFILE, _(L"Previous tab") + L"\tCTRL+SHIFT+TAB",
                 _(L"Go to the document in the previous tab"));
        viewmenu->AppendSeparator();
        MyAppend(viewmenu, A_FULLSCREEN,
                 #ifdef __WXMAC__
                 _(L"Toggle &Fullscreen View") + L"\tCTRL+F11");
                 #else
                 _(L"Toggle &Fullscreen View") + L"\tF11");
                 #endif
        MyAppend(viewmenu, A_SCALED,
                 #ifdef __WXMAC__
                 _(L"Toggle &Scaled Presentation View") + L"\tCTRL+F12");
                 #else
                 _(L"Toggle &Scaled Presentation View") + L"\tF12");
                 #endif
        viewmenu->AppendSeparator();
        viewmenu->AppendSubMenu(scrollmenu, _(L"Scroll Sheet"));
        viewmenu->AppendSubMenu(filtermenu, _(L"Filter"));

        auto roundmenu = new TSPlatformMenu();
        roundmenu->AppendRadioItem(A_ROUND0, _(L"Radius &0"));
        roundmenu->AppendRadioItem(A_ROUND1, _(L"Radius &1"));
        roundmenu->AppendRadioItem(A_ROUND2, _(L"Radius &2"));
        roundmenu->AppendRadioItem(A_ROUND3, _(L"Radius &3"));
        roundmenu->AppendRadioItem(A_ROUND4, _(L"Radius &4"));
        roundmenu->AppendRadioItem(A_ROUND5, _(L"Radius &5"));
        roundmenu->AppendRadioItem(A_ROUND6, _(L"Radius &6"));
        roundmenu->Check(sys->roundness + A_ROUND0, true);

        auto autoexportmenu = new TSPlatformMenu();
        autoexportmenu->AppendRadioItem(A_AUTOEXPORT_HTML_NONE, _(L"No autoexport"));
        autoexportmenu->AppendRadioItem(A_AUTOEXPORT_HTML_WITH_IMAGES, _(L"Export with images"),
                                        _(L"Export to a HTML file with exported images alongside "
                                          L"the original TreeSheets file when document is saved"));
        autoexportmenu->AppendRadioItem(A_AUTOEXPORT_HTML_WITHOUT_IMAGES,
                                        _(L"Export without images"),
                                        _(L"Export to a HTML file alongside the original "
                                          L"TreeSheets file when document is saved"));
        autoexportmenu->Check(sys->autohtmlexport + A_AUTOEXPORT_HTML_NONE, true);

        auto optmenu = new TSPlatformMenu();
        MyAppend(optmenu, wxID_SELECT_FONT, _(L"Font..."),
                 _(L"Set the font the document text is displayed with"));
        MyAppend(optmenu, A_SET_FIXED_FONT, _(L"Typewriter font..."),
                 _(L"Set the font the typewriter text is displayed with."));
        MyAppend(optmenu, A_CUSTKEY, _(L"Key bindings..."),
                 _(L"Change the key binding of a menu item"));
        MyAppend(optmenu, A_SETLANG, _(L"Change language..."), _(L"Change interface language"));
        MyAppend(optmenu, A_DEFAULTMAXCOLWIDTH, _(L"Default column width..."),
                 _(L"Set the default column width for a new grid"));
        optmenu->AppendSeparator();
        MyAppend(optmenu, A_CUSTCOL, _(L"Custom &color..."),
                 _(L"Set a custom color for the color dropdowns"));
        MyAppend(
            optmenu, A_COLCELL, _(L"&Set custom color from cell background"),
            _(L"Set a custom color for the color dropdowns from the selected cell background"));
        MyAppend(optmenu, A_DEFBGCOL, _(L"Background color..."),
                 _(L"Set the color for the document background"));
        MyAppend(optmenu, A_DEFCURCOL, _(L"Cu&rsor color..."),
                 _(L"Set the color for the text cursor"));
        optmenu->AppendSeparator();
        optmenu->AppendCheckItem(
            A_SHOWTBAR, _(L"Toolbar"),
            _(L"Toggle whether toolbar is shown between menu bar and documents"));
        optmenu->Check(A_SHOWTBAR, sys->showtoolbar);
        optmenu->AppendCheckItem(A_SHOWSBAR, _(L"Statusbar"),
                                 _(L"Toggle whether statusbar is shown below the documents"));
        optmenu->Check(A_SHOWSBAR, sys->showstatusbar);
        optmenu->AppendCheckItem(
            A_LEFTTABS, _(L"File Tabs on the bottom"),
            _(L"Toggle whether file tabs are shown on top or on bottom of the documents"));
        optmenu->Check(A_LEFTTABS, lefttabs);
        optmenu->AppendCheckItem(A_TOTRAY, _(L"Minimize to tray"),
                                 _(L"Toogle whether window is minimized to system tray"));
        optmenu->Check(A_TOTRAY, sys->totray);
        optmenu->AppendCheckItem(A_MINCLOSE, _(L"Minimize on close"),
                                 _(L"Toggle whether the window is minimized instead of closed"));
        optmenu->Check(A_MINCLOSE, sys->minclose);
        optmenu->AppendCheckItem(
            A_SINGLETRAY, _(L"Single click maximize from tray"),
            _(L"Toggle whether only one click is required to maximize from system tray"));
        optmenu->Check(A_SINGLETRAY, sys->singletray);
        optmenu->AppendSeparator();
        optmenu->AppendCheckItem(A_ZOOMSCR, _(L"Swap mousewheel scrolling and zooming"));
        optmenu->Check(A_ZOOMSCR, sys->zoomscroll);
        optmenu->AppendCheckItem(A_THINSELC, _(L"Navigate in between cells with cursor keys"),
                                 _(L"Toggle whether the cursor keys are used for navigation in addition to text editing"));
        optmenu->Check(A_THINSELC, sys->thinselc);
        optmenu->AppendSeparator();
        optmenu->AppendCheckItem(A_MAKEBAKS, _(L"Backup files"),
                                 _(L"Create backup file before document is saved to file"));
        optmenu->Check(A_MAKEBAKS, sys->makebaks);
        optmenu->AppendCheckItem(A_AUTOSAVE, _(L"Autosave"),
                                 _(L"Save open documents periodically to temporary files"));
        optmenu->Check(A_AUTOSAVE, sys->autosave);
        optmenu->AppendCheckItem(
            A_FSWATCH, _(L"Autoreload documents"),
            _(L"Reload when another computer has changed a file (if you have made changes, asks)"));
        optmenu->Check(A_FSWATCH, sys->fswatch);
        optmenu->AppendSubMenu(autoexportmenu, _(L"Autoexport to HTML"));
        optmenu->AppendSeparator();
        optmenu->AppendCheckItem(
            A_CENTERED, _(L"Render document centered"),
            _(L"Toggle whether documents are rendered centered or left aligned"));
        optmenu->Check(A_CENTERED, sys->centered);
        optmenu->AppendCheckItem(
            A_FASTRENDER, _(L"Faster line rendering"),
            _(L"Toggle whether lines are drawn solid (faster rendering) or dashed"));
        optmenu->Check(A_FASTRENDER, sys->fastrender);
        optmenu->AppendCheckItem(A_INVERTRENDER, _(L"Invert in dark mode"),
                                 _(L"Invert the document in dark mode"));
        optmenu->Check(A_INVERTRENDER, sys->followdarkmode);
        optmenu->AppendSubMenu(roundmenu, _(L"&Roundness of grid borders"));

        auto scriptmenu = new TSPlatformMenu();
        MyAppend(scriptmenu, A_ADDSCRIPT, _(L"Add...") + L"\tCTRL+ALT+L",
                 _(L"Add Lobster scripts to the menu"));
        MyAppend(scriptmenu, A_DETSCRIPT, _(L"Remove...") + L"\tCTRL+SHIFT+ALT+L",
                 _(L"Remove script from list in the menu"));
        ScriptsUseMenu(scriptmenu);
        scripts.SetMenuPathStyle(wxFH_PATH_SHOW_NEVER);
        scripts.AddFilesToMenu();

        auto scriptpath = app->GetDataPath("scripts/");
        auto sf = wxFindFirstFile(scriptpath + L"*.lobster");
        int sidx = 0;
        while (!sf.empty()) {
            auto fn = wxFileName::FileName(sf).GetFullName();
            scripts.AddFileToHistory(fn);
            sf = wxFindNextFile();
        }

        auto markmenu = new TSPlatformMenu();
        MyAppend(markmenu, A_MARKDATA, _(L"&Data") + L"\tCTRL+ALT+D");
        MyAppend(markmenu, A_MARKCODE, _(L"&Operation") + L"\tCTRL+ALT+O");
        MyAppend(markmenu, A_MARKVARD, _(L"Variable &Assign") + L"\tCTRL+ALT+A");
        MyAppend(markmenu, A_MARKVARU, _(L"Variable &Read") + L"\tCTRL+ALT+R");
        MyAppend(markmenu, A_MARKVIEWH, _(L"&Horizontal View") + L"\tCTRL+ALT+.");
        MyAppend(markmenu, A_MARKVIEWV, _(L"&Vertical View") + L"\tCTRL+ALT+,");

        auto langmenu = new TSPlatformMenu();
        MyAppend(langmenu, wxID_EXECUTE, _(L"&Run") + L"\tCTRL+ALT+F5");
        langmenu->AppendSubMenu(markmenu, _(L"&Mark as"));
        MyAppend(langmenu, A_CLRVIEW, _(L"&Clear Views"));

        auto helpmenu = new TSPlatformMenu();
        MyAppend(helpmenu, wxID_ABOUT, _(L"&About..."), _(L"Show About dialog"));
        helpmenu->AppendSeparator();
        MyAppend(helpmenu, wxID_HELP, _(L"Interactive &tutorial") + L"\tF1",
                 _(L"Load an interactive tutorial in TreeSheets"));
        MyAppend(helpmenu, A_HELP_OP_REF, _(L"Operation reference") + L"\tCTRL+ALT+F1",
                 _(L"Load an interactive program operation reference in TreeSheets"));
        helpmenu->AppendSeparator();
        MyAppend(helpmenu, A_TUTORIALWEBPAGE, _(L"Tutorial &web page"),
                 _(L"Open the tutorial web page in browser"));
        MyAppend(helpmenu, A_SCRIPTREFERENCE, _(L"&Script reference"),
                 _(L"Open the Lobster script reference in browser"));

        SetDefaultAccelerators();

        auto menubar = new TSPlatformMenuBar();
        menubar->Append(filemenu, _(L"&File"));
        menubar->Append(editmenu, _(L"&Edit"));
        menubar->Append(semenu, _(L"&Search"));
        menubar->Append(viewmenu, _(L"&View"));
        menubar->Append(optmenu, _(L"&Options"));
        menubar->Append(scriptmenu, _(L"S&cript"));
        menubar->Append(langmenu, _(L"&Program"));
        menubar->Append(helpmenu,
                        #ifdef __WXMAC__
                        wxApp::s_macHelpMenuTitleName  // so merges with osx provided help
                        #else
                        _(L"&Help")
                        #endif
                        );
        #ifdef __WXMAC__
        // these don't seem to work anymore in the newer wxWidgets, handled in the menu event
        // handler below instead
        wxApp::s_macAboutMenuItemId = wxID_ABOUT;
        wxApp::s_macExitMenuItemId = wxID_EXIT;
        wxApp::s_macPreferencesMenuItemId =
            wxID_SELECT_FONT;  // we have no prefs, so for now just select the font
        #endif
        SetMenuBar(menubar);

        #undef CTRLORALT
        #undef ALTORCTRL
    }
