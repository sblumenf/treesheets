import std
import vec

def at(x, y):
    ts.goto_root()
    ts.goto_column_row(x, y)

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

at(0, 0)
ts.set_background_color(float4 { 1.0, 0.0, 0.0, 1.0 }, false)
ts.set_text_color(float4 { 0.0, 0.0, 1.0, 1.0 }, false)
ts.set_relative_size(-3)
ts.set_text_alignment(3)
ts.set_vertical_alignment(2)
ts.set_note("a note\nof two lines")
ts.set_column_width(55)
at(1, 2)
ts.set_border_color(float4 { 0.0, 0.0, 1.0, 1.0 }, false)
