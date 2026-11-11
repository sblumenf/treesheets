"""Building documents in the real TreeSheets and reading back what it shows, for make_formulas.

A document is written as nested lists, the same way in the case files and in the golden data:

    [text, kind]                       a cell without a grid
    [text, kind, columns, rows, [cells, row by row]]

kind is TreeSheets' number: 0 data, 1 operation, 2 variable assign, 3 horizontal view,
4 variable read, 5 vertical view, 6 formula. In a case, the text of a formula cell is its
formula; in the golden data it is what the app shows (the result). The root cell has no text.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(HERE))
import referee  # noqa: E402
import ui  # noqa: E402

OPERATIONS = ["+", "-", "*", "/", "<", ">", "<=", ">=", "=", "==", "!=", "<>", "inc", "dec", "neg",
              "graph", "sum", "min", "max", "avg", "median", "transpose", "if"]

# Program > Run has Ctrl+Alt+F5, which never arrives (the X server takes it): bind another key.
ui.ACTIONS["run"] = ("&Run", "CTRL+ALT+N", "ctrl+alt+n")


def lit(s):
    """A Lobster expression for any text, with no escaping to get wrong."""
    if s and all(c.isalnum() or c in " _.+-*/=<>(),#!?" for c in s) and s.isascii():
        return '"' + s + '"'
    return "unicode_to_string([%s])" % ",".join(str(ord(c)) for c in s) if s else '""'


def build_cell(cell, code):
    """Lobster that makes the current cell what `cell` says."""
    text, kind = cell[0], cell[1]
    if kind == 6:
        code.append(f"ts.set_formula({lit(text)})")
    elif kind == 1 and text not in OPERATIONS:
        # Only a cell whose text is an operation can be marked as one; the text can change
        # afterwards (as a file can have it).
        code += ['ts.set_text("+")', "ts.set_cell_type(1)", f"ts.set_text({lit(text)})"]
    else:
        if text:
            code.append(f"ts.set_text({lit(text)})")
        if kind:
            code.append(f"ts.set_cell_type({kind})")
    if len(cell) > 2:
        xs, ys, cells = cell[2], cell[3], cell[4]
        code.append(f"ts.create_grid({xs}, {ys})")
        build_grid(xs, cells, code)


def build_grid(xs, cells, code):
    for i, child in enumerate(cells):
        if child[0] == "" and child[1] == 0 and len(child) == 2:
            continue
        code.append(f"ts.goto_column_row({i % xs}, {i // xs})")
        build_cell(child, code)
        code.append("ts.goto_parent()")


def build(doc):
    """Lobster that opens a new document and makes it `doc` (the root: only its grid)."""
    xs, ys, cells = doc[2], doc[3], doc[4]
    code = ["import std", f"ts.new_document({xs}, {ys})", "ts.goto_root()"]
    build_grid(xs, cells, code)
    code.append('ts.agent_result("ok")')
    return "\n".join(code)


def goto(path):
    """Lobster that goes to the cell at `path`: a list of [column, row] from the root."""
    return ["ts.goto_root()"] + [f"ts.goto_column_row({x}, {y})" for x, y in path]


def compact(cell):
    """A cell of referee.parse_dump as nested lists."""
    kind = referee.CELL_TYPES.index(cell["type"])
    grid = cell["grid"]
    if grid is None:
        return [cell["text"], kind]
    return [cell["text"], kind, grid["cols"], grid["rows"], [compact(c) for c in grid["cells"]]]


def shown(ts):
    """The open document as the app shows it."""
    return compact(referee.parse_dump(ts.eval(file=referee.DUMP_SCRIPT))[0]["root"])
