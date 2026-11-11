import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(2, 1)
span("日本") s, n: ts.set_link(s, n, "https://example.org/jp", false)
