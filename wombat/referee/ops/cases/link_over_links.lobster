import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(1, 1)
span("link and mo") s, n: ts.set_link(s, n, "https://example.org/new", false)
