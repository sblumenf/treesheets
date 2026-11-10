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
ts.set_span_text_color(0, 12, float4 { 0.0, 0.5, 0.0, 1.0 })
