import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(1, 2)
ts.delete(int2 { 0, 1 }, int2 { 2, 1 })
