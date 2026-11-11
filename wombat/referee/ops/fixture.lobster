// Builds fixture.cts, the small document that every operation case in cases/ starts from.
// Run by make_ops --fixture, which puts a line that defines out_path in front.
//
//   plain                  | bold cell (bold)           | colours (yellow, navy text)
//   one bold, one italic.. | a web link and more text   | naïve 日本語 😀 emoji end
//   sized (bigger, centre) | nested (2 x 2 grid a b c d) | note (has a note)
//   (empty)                | (empty)                    | last

import std
import vec

def cell(x, y, text, f):
    ts.goto_root()
    ts.goto_column_row(x, y)
    ts.set_text(text)
    f()

// The byte offset and size of part in the text of the current cell.
def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

ts.new_document(3, 4)

cell(0, 0, "plain"): nil
cell(1, 0, "bold cell"): ts.set_style_bits(1)
cell(2, 0, "colours"):
    ts.set_background_color(float4 { 1.0, 1.0, 0.5, 1.0 }, false)
    ts.set_text_color(float4 { 0.0, 0.0, 0.5, 1.0 }, false)
cell(0, 1, "one bold, one italic and one red word"):
    span("bold") s, n: ts.set_span_style_bits(s, n, 1)
    span("italic") s, n: ts.set_span_style_bits(s, n, 2)
    span("red") s, n: ts.set_span_text_color(s, n, float4 { 1.0, 0.0, 0.0, 1.0 })
cell(1, 1, "a web link and more text"):
    span("web link") s, n: ts.set_link(s, n, "https://example.com/", false)
    span("more") s, n: ts.set_link(s, n, "docs/more.pdf", false)
    ts.set_column_width(120)
cell(2, 1, "naïve 日本語 😀 emoji end"):
    span("日本語") s, n: ts.set_span_style_bits(s, n, 1)
    span("😀 emoji") s, n: ts.set_link(s, n, "https://example.com/emoji", false)
cell(0, 2, "sized"):
    ts.set_relative_size(2)
    ts.set_text_alignment(2)
cell(1, 2, "nested"):
    ts.create_grid(2, 2)
    ts.set_border_color(float4 { 0.5, 0.0, 0.0, 1.0 }, false)
    for(4) i:
        ts.goto_child(i)
        ts.set_text([ "a", "b", "c", "d" ][i])
        ts.goto_parent()
    ts.goto_child(3)
    ts.set_background_color(float4 { 0.5, 1.0, 0.5, 1.0 }, false)
cell(2, 2, "note"): ts.set_note("A note.")
cell(2, 3, "last"): nil

ts.goto_root()
ts.goto_column_row(0, 0)
ts.select()
assert ts.save_document_as(out_path)
ts.agent_result("saved " + ts.get_filename())
