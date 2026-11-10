"""Drives the real TreeSheets app, headless, over its agent socket.

Shared by export_golden and run_script. Python 3.8+ standard library only; needs Xvfb and a
TreeSheets binary built with Lobster (see README.md).
"""
import contextlib
import getpass
import hashlib
import json
import os
import shutil
import signal
import socket
import subprocess
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
DUMP_SCRIPT = os.path.join(HERE, "dump_tree.lobster")

CELL_TYPES = ["data", "operation", "variable_assign", "view_horizontal", "variable_read",
              "view_vertical", "formula"]
ALIGN = ["auto", "left", "center", "right"]
VALIGN = ["auto", "top", "middle", "bottom"]
RENDER_STYLES = ["grid", "bubble", "line"]


class RefereeError(Exception):
    pass


def find_binary():
    path = os.environ.get("TREESHEETS_BIN") or os.path.join(REPO, "_build", "TreeSheets")
    if not os.access(path, os.X_OK):
        raise RefereeError(f"no TreeSheets binary at {path}: build it (see README.md) or set "
                           f"TREESHEETS_BIN")
    return path


@contextlib.contextmanager
def xvfb():
    """A private X server; yields its DISPLAY. Never the user's display."""
    if not shutil.which("Xvfb"):
        raise RefereeError("Xvfb not found (apt-get install xvfb)")
    r, w = os.pipe()
    proc = subprocess.Popen(
        ["Xvfb", "-displayfd", str(w), "-screen", "0", "1280x1024x24", "-nolisten", "tcp"],
        pass_fds=[w], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.close(w)
    try:
        with os.fdopen(r) as f:
            line = f.readline().strip()
        if not line:
            raise RefereeError("Xvfb did not start")
        yield ":" + line
    finally:
        proc.terminate()
        try:
            proc.wait(10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


class Instance:
    """One TreeSheets process showing a private copy of one .cts file."""

    def __init__(self, display, cts, start_timeout=60):
        self.workdir = tempfile.mkdtemp(prefix="ts-referee-")
        self.proc = None
        self.started = int(time.time() * 1000) - 2000  # the clock of last_edit, with a margin
        self.endpoint = None
        try:
            # A copy, so that autosave/backup files never land next to the original.
            self.doc = os.path.join(self.workdir, "doc", os.path.basename(cts))
            os.makedirs(os.path.dirname(self.doc))
            shutil.copyfile(cts, self.doc)
            home = os.path.join(self.workdir, "home")
            os.makedirs(home)
            env = {k: v for k, v in os.environ.items()
                   if k not in ("WAYLAND_DISPLAY", "XAUTHORITY", "DBUS_SESSION_BUS_ADDRESS")
                   and not k.startswith("LC_")}
            env.update(DISPLAY=display, HOME=home, XDG_CONFIG_HOME=os.path.join(home, ".config"),
                       XDG_DATA_HOME=os.path.join(home, ".local", "share"),
                       XDG_CACHE_HOME=os.path.join(home, ".cache"), GDK_BACKEND="x11",
                       LANG="C.UTF-8", LANGUAGE="en", NO_AT_BRIDGE="1", GSETTINGS_BACKEND="memory")
            self.log = open(os.path.join(self.workdir, "treesheets.log"), "wb")
            # -i: own instance, -p: settings in the working directory, -a: agent socket.
            self.proc = subprocess.Popen([find_binary(), "-i", "-p", "-a", self.doc], cwd=home,
                                         env=env, stdin=subprocess.DEVNULL, stdout=self.log,
                                         stderr=self.log)
            self.endpoint = f"/tmp/TreeSheets-agent-{getpass.getuser()}-{self.proc.pid}.sock"
            deadline = time.time() + start_timeout
            while True:
                if self.proc.poll() is not None:
                    raise RefereeError(f"TreeSheets exited with {self.proc.returncode} at startup")
                if os.path.exists(self.endpoint) and os.path.exists(self.endpoint + ".token"):
                    try:
                        if self.request({"cmd": "ping"}, 5).get("ok"):
                            break
                    except (OSError, ValueError):
                        pass
                if time.time() > deadline:
                    raise RefereeError("TreeSheets did not open its agent socket (a dialog may "
                                       "be blocking startup)")
                time.sleep(0.05)
            loaded = self.eval("ts.agent_result(ts.get_filename())")
            if os.path.realpath(loaded) != os.path.realpath(self.doc):
                raise RefereeError(f"TreeSheets did not load the file (open document: "
                                   f"{loaded or 'unnamed'!r})")
        except BaseException:
            self.close()
            raise

    def request(self, req, timeout):
        with open(self.endpoint + ".token") as f:
            token = f.read().strip()
        req = dict(req, id="1", token=token)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
            s.settimeout(timeout)
            s.connect(self.endpoint)
            # The app's JSON reader takes UTF-8 as is, but no \u escapes beyond the BMP.
            s.sendall(json.dumps(req, ensure_ascii=False).encode("utf-8") + b"\n")
            buf = bytearray()
            while not buf.endswith(b"\n"):
                chunk = s.recv(1 << 16)
                if not chunk:
                    raise RefereeError("TreeSheets closed the connection without a reply")
                buf += chunk
        return json.loads(buf.decode("utf-8"))

    def eval_raw(self, code=None, file=None, timeout=300):
        req = {"cmd": "eval"}
        if code is not None:
            req["code"] = code
        else:
            req["file"] = os.path.abspath(file)
        return self.request(req, timeout)

    def eval(self, code=None, file=None, timeout=300):
        resp = self.eval_raw(code, file, timeout)
        if not resp.get("ok"):
            raise RefereeError("script failed: " + resp.get("error", "unknown error"))
        return resp.get("result", "")

    def subtree_text(self, fmt):
        """The whole document as TreeSheets exports it: 0 = indented text, 2 = XML."""
        return unix_lines(self.eval(f"ts.agent_result(ts.get_subtree_text({fmt}))"))

    def close(self):
        if self.proc is not None and self.proc.poll() is None:
            # No window manager to send a close request, so end the process. That leaves the
            # socket and token files, which are removed below.
            self.proc.send_signal(signal.SIGTERM)
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        if getattr(self, "log", None):
            self.log.close()
        if self.endpoint:
            for p in (self.endpoint, self.endpoint + ".token"):
                with contextlib.suppress(OSError):
                    os.remove(p)
        shutil.rmtree(self.workdir, ignore_errors=True)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()


def unix_lines(text):
    """TreeSheets ends the line of a cell with CR LF on Linux and LF elsewhere. Cell text has no
    line breaks of its own (control characters become spaces on load), so this loses nothing."""
    return text.replace("\r\n", "\n")


def cts_header(path):
    """(format version, encrypted) of a .cts file."""
    with open(path, "rb") as f:
        head = f.read(6)
    if head[:4] != b"TSFF" or len(head) < 5:
        raise RefereeError(f"{path} is not a .cts file")
    return head[4], len(head) > 5 and head[4] >= 30 and head[5] != 0


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


# ---- dump_tree.lobster output -> JSON -----------------------------------------------------------

class Netstrings:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def bytes(self):
        colon = self.data.index(b":", self.pos)
        n = int(self.data[self.pos:colon])
        self.pos = colon + 1 + n
        return self.data[colon + 1:self.pos]

    def str(self):
        return self.bytes().decode("utf-8")

    def int(self):
        return int(self.bytes())

    def bool(self):
        return self.str() not in ("0", "false")

    def color(self):
        r, g, b = (int(x) for x in self.str().split(","))
        return f"#{r:02X}{g:02X}{b:02X}"


def parse_cell(ns, path, stats, lastedit):
    stats["cells"] += 1
    textbytes = ns.bytes()
    cell = {"path": path, "text": textbytes.decode("utf-8"), "note": ns.str()}
    formula = ns.str()
    celltype = ns.int()
    cell["type"] = CELL_TYPES[celltype]
    cell["formula"] = formula if celltype == 6 else None
    cell["bg"] = ns.color()
    cell["fg"] = ns.color()
    cell["relsize"] = ns.int()
    cell["stylebits"] = ns.int()
    cell["align"] = ALIGN[ns.int()]
    cell["valign"] = VALIGN[ns.int()]
    cell["render_style"] = RENDER_STYLES[ns.int()]
    cell["vertical_layout"] = ns.bool()
    cell["filtered"] = ns.bool()
    cell["is_tag"] = ns.bool()
    cell["has_image"] = ns.bool()
    # Milliseconds since the Unix epoch, or null where it is not from the file (see parse_dump).
    edited = ns.int()
    cell["last_edit"] = edited if lastedit and edited < lastedit else None
    colwidth = ns.int()
    runs, links = [], []
    for _ in range(ns.int()):
        start, length, stylebits, color = ns.int(), ns.int(), ns.int(), ns.color()
        target, internal = ns.str(), ns.bool()
        # Positions in bytes of the UTF-8 text and in Unicode code points.
        pos = {"start": len(textbytes[:start].decode("utf-8")),
               "len": len(textbytes[start:start + length].decode("utf-8")),
               "start_utf8": start, "len_utf8": length}
        if stylebits != cell["stylebits"] or color != cell["fg"]:
            runs.append(dict(pos, stylebits=stylebits, fg=color))
        if target:
            link = dict(pos, target=target, internal=internal)
            last = links[-1] if links else None
            if (last and last["target"] == target and last["internal"] == internal and
                    last["start_utf8"] + last["len_utf8"] == start):
                last["len"] += pos["len"]
                last["len_utf8"] += length
            else:
                links.append(link)
    cell["runs"] = runs
    cell["links"] = links
    for key, flag in (("formulas", celltype == 6), ("operations", celltype in (1, 2, 3, 4, 5)),
                      ("notes", cell["note"]), ("cells_with_runs", runs),
                      ("cells_with_links", links), ("images", cell["has_image"]),
                      ("tags", cell["is_tag"]), ("filtered", cell["filtered"])):
        if flag:
            stats[key] = stats.get(key, 0) + 1
    cols, rows = ns.int(), ns.int()
    if cols:
        stats["grids"] += 1
        grid = {"cols": cols, "rows": rows, "folded": ns.bool(), "border_color": ns.color(),
                "col_widths": [], "cells": []}
        if grid["folded"]:
            stats["folded_grids"] = stats.get("folded_grids", 0) + 1
        for i in range(cols * rows):
            child, width = parse_cell(ns, path + [[i % cols, i // cols]], stats, lastedit)
            if i < cols:
                grid["col_widths"].append(width)
            grid["cells"].append(child)
        cell["grid"] = grid
    else:
        cell["grid"] = None
    return cell, colwidth


def parse_dump(data, lastedit=0):
    """The JSON document for the output of dump_tree.lobster, and statistics about it. lastedit is when the app was started (Instance.started), or 0 if a
    script may have changed the document: edit times from then on are not from the file, so
    last_edit is null for them. TreeSheets gives cells the time of loading if the file has no
    time for them: all cells of files older than format version 14, and cells without text."""
    ns = Netstrings(data.encode("utf-8"))
    doc = {}
    hassel = ns.bool()
    x, y, xs, ys = (int(v) for v in ns.str().split(","))
    doc["selection"] = {"x": x, "y": y, "cols": xs, "rows": ys} if hassel else None
    doc["autoexport_html"] = ns.int()
    doc["autoexport_pdf"] = ns.bool()
    stats = {"cells": 0, "grids": 0}
    doc["root"], _ = parse_cell(ns, [], stats, lastedit)
    if ns.pos != len(ns.data):
        raise RefereeError("trailing data in the tree dump")
    return doc, stats


def _dumps(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def _write_cell(out, cell, indent):
    """One line per cell, so that diffs of golden files stay readable."""
    pad = " " * indent
    props = ", ".join(f"{_dumps(k)}: {_dumps(v)}" for k, v in cell.items() if k != "grid")
    grid = cell["grid"]
    if grid is None:
        out.append(f'{pad}{{{props}, "grid": null}}')
        return
    gprops = ", ".join(f"{_dumps(k)}: {_dumps(v)}" for k, v in grid.items() if k != "cells")
    out.append(f'{pad}{{{props}, "grid": {{{gprops}, "cells": [')
    for i, child in enumerate(grid["cells"]):
        _write_cell(out, child, indent + 1)
        if i + 1 < len(grid["cells"]):
            out[-1] += ","
    out.append(f"{pad}]}}}}")


def format_document(doc):
    head = {k: v for k, v in doc.items() if k != "root"}
    out = ["{" + ", ".join(f"{_dumps(k)}: {_dumps(v)}" for k, v in head.items()) + ', "root":']
    _write_cell(out, doc["root"], 0)
    out.append("}")
    return "\n".join(out) + "\n"
