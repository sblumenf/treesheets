import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(1, 0)
ts.set_span_style_bits(0, 4, 0)
ts.set_span_style_bits(5, 4, 3)
