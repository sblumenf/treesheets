# Trying the first slice on an Android phone

For the owner, with a phone and no other tools. It takes about half an hour. The screen is a
prototype: it looks plain on purpose. What matters is whether it does what you mean, loses
nothing you type, and what gets in your way.

## Get the app onto the phone

1. On GitHub, open the repository, then **Actions**, then the workflow **Wombat**, and the
   newest run of the branch with a green tick.
2. At the bottom of that run's page, under **Artifacts**, download the Android package (a
   zip with an `.apk` in it). You have to be signed in to GitHub to download it.
   *When this page was written the workflow built the Android package and did not yet keep
   it as an artifact; if there is nothing under Artifacts, ask for the job "App for
   Android" to upload `build/app/outputs/flutter-apk/app-debug.apk`, or build it as
   `ANDROID.md` says under "To install without building".*
3. Get the `.apk` onto the phone (unzip it first; send it to yourself, or copy it over
   USB) and tap it. Android asks whether to allow installing from this source: allow it for
   this once. The app is called **Wombat**. It is a debug build: slower than a real one,
   and Android may warn that it does not know who made it.

To start over at any time: uninstall Wombat (its documents go with it) and install it
again.

## What to try

Go through these in order. After each, note what happened if it was not what the line says.

**1. Open the example.** The app starts on a small document called "Example": two columns,
three rows, with two small grids drawn inside the cells "Shopping" and "Costs". The word
under the title says "Saved".

**2. Dive and come back.** Tap "Costs", then **Open ›** in the bar above the keyboard. The
grid of "Costs" fills the screen and its name is the title. Come back three ways, diving
again in between: the **‹** arrow at the bottom, the name "Example" beside it, and the
phone's own back button or back swipe. Back at the top, the phone's back leaves the app.

**3. Type, cancel, undo.**
- Tap "what to buy", type something, press **✓**. The cell has the new text. A moment
  later the word under the title goes from "Not saved yet" to "Saved".
- Tap it again, type something else, press **✕**. The cell has the text from before.
- Tap "Apples" (inside "Shopping"), change it, and press Return on the keyboard: the editor
  moves to "Bread". Change that too, Return, change "Cheese", **✓**.
- Press the undo arrow at the top three times: Cheese, Bread and Apples come back, one
  each time. The redo arrow beside it brings your texts back.

**4. Add rows and columns.**
- Tap the **+** under the last row: a new row, with the editor in its first cell. Type,
  then press **→** above the keyboard: the editor moves to the cell on the right.
- Tap the **+** at the right edge: a new column.
- Tap a cell in the middle, then **+Row** above the keyboard: a row appears below it, with
  the editor in it. The same with **+Col**.
- Tap the last cell of a column and press Return: a row is added and the editor is in it.
- Undo takes these back one at a time. (Nothing deletes a row yet; undo is the way.)

**5. Put the app away in the middle of typing.**
- Tap a cell, type half a sentence, and do not press ✓. Go to the phone's home screen,
  wait a few seconds, and come back. The editor should be as you left it. Press **✕**: the
  cell should have the text from before you started.
- Do it again, but this time swipe the app away in the list of recent apps and start it
  again. The half sentence should be in the cell.
- Pull the notification shade down and up while typing: nothing should change.

**6. Rotate.** Turn the phone on its side with the editor open, and back. The text you were
typing stays, and the cell you were typing in is still in sight above the keyboard.

**7. The large documents.** Tap **⋮** at the top right, and under the line choose "Six
levels (12×20)". The first time it takes a few seconds to make. Then:
- Scroll down and sideways. The first row and the first column stay in view.
- Find row 12, column 7 (the cell says "Level 0 cell 6.11 text": the numbers count from 0).
- Tap the small framed preview of a grid inside a cell (a few of its texts in small
  print, and its size, such as "4×5"): you go into that grid. Do it again, three levels
  down, and come back up with **‹**. Each level should be where you left it.
- In the same menu, "Large (100,000 cells)". Scroll it, go into a grid, type into a cell.
  Say if anything waits long enough to be annoying, and where.
- "Depth forms" shows the four ways a grid inside a cell is drawn (cell by cell, a preview
  of its first texts, a small picture, a count), and cells with long text, right-to-left
  text and colours. Say which of the four you can read and which you cannot.

**8. Line breaks.** Tap a cell and press **↵** above the keyboard (not Return): the text
gets a second line. Press ✓: the cell shows both lines.

**9. Aa.** With the editor open, tap **Aa**: Bold, Italic, Link and Tag appear. Select a
word in the editor and tap Bold; the cell above shows it. ✓ keeps it, ✕ takes it back.

**10. The two switches.** **⋮**, then "Prototype settings":
- "Tap text: select first, tap again to edit". Try a few minutes with it on, then off.
  Which did you mean more often?
- "Tap child grid: unfold in place". With it on, tapping the framed preview of a grid
  opens that grid inside its cell instead of going into it ("Fold" above it closes it
  again). The same question.
- "Keep the first row and column in view": off, if it was in your way.

## What to note down

- Anything you typed that was not there afterwards. **This matters most.** Say what you
  did just before.
- Every time you went into a grid when you meant to edit, or the other way round.
- Anything too small to hit, or covered by the keyboard.
- Where the keyboard's Return did not move to the cell below, or its key says something
  other than "next".
- What the phone's back did when the keyboard was up.
- Any red or yellow-and-black striped area on the screen, and any message that starts with
  "Error".
- Which phone, and which version of Android.

## The prototype log

The app writes down what you do: every dive, way up, edit, undo and so on, with the time
and the switch settings, so that accidental dives can be counted. It holds no text of any
cell. It stays on the phone until you send it:

1. **⋮**, "Prototype settings", scroll to the bottom.
2. Either "Copy the log", then paste it into a mail or a message to yourself; or "Export
   the log as a file", which says where the file is. On Android that is the folder
   `Android/data/io.github.sblumenf.burrow/files` on the phone's storage, which a computer
   shows when the phone is plugged in by USB (the phone's own file manager may hide it).
3. Send it along with your notes.

Nobody has run this app on a real phone before you. If it does not start at all, that is
the first thing to say.
