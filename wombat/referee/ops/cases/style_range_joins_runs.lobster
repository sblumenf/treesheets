import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(0, 1)
span(", one ") s, n: ts.set_span_style_bits(s, n, 1)
span("italic") s, n: ts.set_span_style_bits(s, n, 1)
