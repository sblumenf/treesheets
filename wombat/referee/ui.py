"""Triggers menu actions of the real TreeSheets with key presses, for what its script API lacks.

Sort, transpose, hierarchify, flatten, hierarchy swap, wrap and the like are menu actions
(`Document::Action` in src/document.h) that no Lobster function reaches. This module presses
their keyboard shortcuts with `xdotool` on the private Xvfb display that referee.py made, and
never on any other display. Actions without a shortcut get one through TreeSheets' own custom key
bindings: in portable mode (-p) it reads them from TreeSheets.ini in its working directory, where
the key is the menu label (see `MyAppend` in src/tsframe.h). Nothing in TreeSheets is changed.

A case file (`*.ui`) is a list of lines; places and numbers are written as for `wombat edit`
(columns and rows from 1, `/` the root cell):

    # comment
    select CELL COLUMN ROW COLUMNS ROWS   select a block of the grid of CELL
    action NAME [unchanged]               press the key of a menu action (see ACTIONS); the
                                          document must change, or with `unchanged` must not
    cursor CELL POSITION                  edit the text of CELL with the cursor at POSITION
    keys KEY...                           raw xdotool key names
    lobster CODE ; CODE ...               lines of Lobster (the current cell is the root)

Needs Xvfb, xdotool and a TreeSheets built with Lobster (see README.md).
"""
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import referee  # noqa: E402

# name: (menu label in src/tsframe.h, key bound through TreeSheets.ini or None for the default,
# the same key for xdotool). A new binding must be a key that no menu item has already (of
# Ctrl+Alt+letter those are A C D L O R T V): the first item with a key gets it.
ACTIONS = {
    "sort": ("Sort &Ascending", "CTRL+ALT+J", "ctrl+alt+j"),
    "sort-descending": ("Sort &Descending", "CTRL+ALT+K", "ctrl+alt+k"),
    "hierarchify": ("&Hierarchify", "CTRL+ALT+H", "ctrl+alt+h"),
    "flatten": ("&Flatten", "CTRL+ALT+F", "ctrl+alt+f"),
    "tag-add": ("&Add Cell Text as Tag", "CTRL+ALT+G", "ctrl+alt+g"),
    "tag-remove": ("&Remove Cell Text from Tags", "CTRL+ALT+U", "ctrl+alt+u"),
    "image-remove": ("Remo&ve", "CTRL+ALT+I", "ctrl+alt+i"),
    "transpose": ("&Transpose", None, "ctrl+shift+t"),
    "hierarchy-swap": ("Hierarchy &Swap", None, "F8"),
    "wrap": ("&Wrap in new parent", None, "F9"),
    "collapse": ("Collapse Ce&lls", None, "ctrl+l"),
    "split": ("Split cell at the cursor", None, "alt+shift+Return"),
    "delete-grid": ("Delete Grid", None, "shift+Delete"),
    "toggle-check": ("Toggle todo chec&kbox", None, "ctrl+shift+x"),
    "bigger": ("&Increase text size (SHIFT+mousewheel)", None, "shift+Prior"),
    "smaller": ("&Decrease text size (SHIFT+mousewheel)", None, "shift+Next"),
    "bigger-flat": ("Increase text size (no sub grids, ...)", None, "ctrl+shift+Prior"),
    "smaller-flat": ("Decrease text size (no sub grids, ...)", None, "ctrl+shift+Next"),
    "wider": ("Increase column width (ALT+mousewheel)", None, "alt+Prior"),
    "narrower": ("Decrease column width (ALT+mousewheel)", None, "alt+Next"),
    "wider-flat": ("Increase column width (no sub grids, ...)", None, "ctrl+alt+Prior"),
    "narrower-flat": ("Decrease column width (no sub grids, ...)", None, "ctrl+alt+Next"),
    "reset-size": ("&Reset text sizes", None, "ctrl+shift+s"),
    "reset-style": ("&Reset text styles", None, "ctrl+shift+k"),
    "reset-color": ("Reset &colors", None, "ctrl+shift+c"),
    "bold": ("Toggle cell &BOLD", None, "ctrl+b"),
    "undo": ("&Undo", None, "ctrl+z"),
}


def _ini():
    """TreeSheets.ini with the extra key bindings, in wxFileConfig's spelling of the labels."""
    lines = []
    for label, binding, _ in ACTIONS.values():
        if binding:
            name = "".join(c if c.isalnum() else "\\" + c for c in label)
            lines.append(f"{name}={binding}\n")
    return "".join(lines)


def _xdotool(display, *args):
    if not shutil.which("xdotool"):
        raise referee.RefereeError("xdotool not found (apt-get install xdotool)")
    env = dict(os.environ, DISPLAY=display)
    env.pop("WAYLAND_DISPLAY", None)
    done = subprocess.run(["xdotool", *args], env=env, capture_output=True, text=True)
    if done.returncode != 0:
        raise referee.RefereeError(f"xdotool {' '.join(args)}: {done.stderr.strip()}")
    return done.stdout


def _steps(place):
    """`/2,1/1,3` as Lobster that goes there from the root."""
    code = ["ts.goto_root()"]
    for step in filter(None, place.split("/")):
        x, y = (int(n) for n in step.split(","))
        code.append(f"ts.goto_column_row({x - 1}, {y - 1})")
    return code


class Instance(referee.Instance):
    """A TreeSheets with the key bindings of ACTIONS, whose window has the keyboard."""

    def __init__(self, display, cts):
        self.display = display
        self.launchdir = tempfile.mkdtemp(prefix="ts-ui-")
        real = referee.find_binary()
        ini = os.path.join(self.launchdir, "TreeSheets.ini")
        with open(ini, "w", encoding="utf-8") as f:
            f.write(_ini())
        # The settings file must be in the working directory before the app starts, and
        # referee.Instance makes that directory itself: so start the app through a script
        # that puts the file there first. `exec` keeps the process ID, which names the socket.
        launcher = os.path.join(self.launchdir, "launch")
        with open(launcher, "w", encoding="utf-8") as f:
            f.write(f'#!/bin/sh\ncp "{ini}" ./TreeSheets.ini\nexec "{real}" "$@"\n')
        os.chmod(launcher, os.stat(launcher).st_mode | stat.S_IXUSR)
        find = referee.find_binary
        referee.find_binary = lambda: launcher
        try:
            super().__init__(display, cts)
        finally:
            referee.find_binary = find
        try:
            # There is no window manager, so nothing gave the window the keyboard yet.
            deadline = time.time() + 20
            while True:
                found = subprocess.run(
                    ["xdotool", "search", "--onlyvisible", "--pid", str(self.proc.pid)],
                    env=dict(os.environ, DISPLAY=display), capture_output=True, text=True)
                windows = found.stdout.split()
                if windows:
                    break
                if time.time() > deadline:
                    raise referee.RefereeError("TreeSheets showed no window")
                time.sleep(0.1)
            _xdotool(display, "windowfocus", windows[-1])
            time.sleep(0.2)
        except BaseException:
            self.close()
            raise

    def close(self):
        super().close()
        shutil.rmtree(self.launchdir, ignore_errors=True)

    def root(self):
        """The document as it is now, without the selection."""
        return referee.parse_dump(self.eval(file=referee.DUMP_SCRIPT))[0]["root"]

    def lobster(self, lines):
        self.eval("import std\nimport vec\n" + "\n".join(lines) + '\nts.agent_result("ok")')

    def keys(self, *keys):
        """Presses keys and waits until the app has dealt with them: the script that reads
        the document runs in the same event loop, after the key events."""
        _xdotool(self.display, "key", "--delay", "40", *keys)
        time.sleep(0.15)

    def press(self, key, changes=True):
        """Presses the key of an action; the document must then differ, or with changes
        False must not."""
        before = self.root()
        self.keys(key)
        deadline = time.time() + (5 if changes else 0.6)
        while True:
            after = self.root()
            if (after != before) == changes and changes:
                break
            if time.time() > deadline:
                break
            time.sleep(0.1)
        # Whatever happened has happened when two looks in a row agree.
        while True:
            time.sleep(0.1)
            again = self.root()
            if again == after:
                break
            after = again
        if (after != before) != changes:
            raise referee.RefereeError(
                f"the key {key} " + ("changed nothing" if changes else "changed the document"))

    def run_case(self, path):
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
        for number, line in enumerate(lines, 1):
            words = line.split()
            if not words or words[0].startswith("#"):
                continue
            try:
                self.run_line(words, line)
            except (ValueError, IndexError, KeyError) as e:
                raise referee.RefereeError(f"{path}:{number}: {line!r}: {e!r}")
            except referee.RefereeError as e:
                raise referee.RefereeError(f"{path}:{number}: {e}")

    def run_line(self, words, line):
        verb = words[0]
        if verb == "select":
            x, y, xs, ys = (int(n) for n in words[2:6])
            self.lobster(_steps(words[1]) +
                         [f"ts.select_range(int2 {{ {x - 1}, {y - 1} }}, int2 {{ {xs}, {ys} }})"])
        elif verb == "action":
            if words[2:] not in ([], ["unchanged"]):
                raise ValueError("after the action only `unchanged` may follow")
            self.press(ACTIONS[words[1]][2], changes=len(words) == 2)
        elif verb == "cursor":
            self.lobster(_steps(words[1]) + ["ts.select()"])
            # F2 edits with the cursor at the end, Ctrl+Home takes it to the start.
            self.keys("F2", "ctrl+Home", *["Right"] * int(words[2]))
        elif verb == "keys":
            self.keys(*words[1:])
        elif verb == "lobster":
            self.lobster(["ts.goto_root()"] + line.split(None, 1)[1].split(" ; "))
        else:
            raise ValueError("not select, action, cursor, keys or lobster")

    def check_keys_arrive(self):
        """Presses a key that always changes the document, to tell a refused action from a
        key press that went nowhere. Call it after reading the result of a case."""
        self.keys("Escape", "Escape")
        self.lobster(["ts.goto_root()", "ts.select_range(int2 { 0, 0 }, int2 { 1, 1 })"])
        self.press(ACTIONS["bold"][2])
