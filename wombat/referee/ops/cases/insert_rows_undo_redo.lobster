import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

ts.goto_root()
ts.insert_row(1)
ts.insert_column(1)
assert ts.undo()
assert ts.redo()
