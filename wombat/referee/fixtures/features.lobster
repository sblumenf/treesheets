// Builds features.cts: a document with what none of the TreeSheets examples have (they are all
// older than the current file format): styled and colored spans, links, formulas of the current
// kind, alignment, filtered text, on top of notes, an image and a folded grid.
// Run by make_fixtures, which puts a line that defines out_path and repo_path in front.

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

ts.new_document(4, 7)

// Whole cell styles and sizes.
cell(0, 0, "plain"): nil
cell(1, 0, "bold"): ts.set_style_bits(1)
cell(2, 0, "bold italic fixed underline strikethru"): ts.set_style_bits(31)
cell(3, 0, "bigger"): ts.set_relative_size(2)
cell(0, 1, "smaller"): ts.set_relative_size(-2)
cell(1, 1, "colors"):
    ts.set_background_color(float4 { 1.0, 1.0, 0.5, 1.0 }, false)
    ts.set_text_color(float4 { 0.0, 0.0, 0.5, 1.0 }, false)

// Spans.
cell(2, 1, "one bold, one italic, one red and one fixed green word"):
    span("bold") s, n: ts.set_span_style_bits(s, n, 1)
    span("italic") s, n: ts.set_span_style_bits(s, n, 2)
    span("red") s, n: ts.set_span_text_color(s, n, float4 { 1.0, 0.0, 0.0, 1.0 })
    span("fixed green") s, n:
        ts.set_span_style_bits(s, n, 4)
        ts.set_span_text_color(s, n, float4 { 0.0, 0.5, 0.0, 1.0 })
cell(3, 1, "naïve 日本語 😀 emoji, שלום and déjà vu"):
    span("日本語") s, n: ts.set_span_style_bits(s, n, 1)
    span("😀 emoji") s, n: ts.set_span_text_color(s, n, float4 { 0.0, 0.0, 1.0, 1.0 })
    span("déjà") s, n: ts.set_span_style_bits(s, n, 24)
cell(0, 2, "italic cell with a plain span"):
    ts.set_style_bits(2)
    span("plain") s, n: ts.set_span_style_bits(s, n, 0)

// Links.
cell(1, 2, "a web link and a file link"):
    span("web link") s, n: ts.set_link(s, n, "https://example.com/a?b=1&c=2", false)
    span("file link") s, n: ts.set_link(s, n, "docs/report.pdf", false)
cell(2, 2, "go to Numbers, or to 日本 (missing)"):
    span("Numbers") s, n: ts.set_link(s, n, "Numbers", true)
    span("日本") s, n: ts.set_link(s, n, "no such cell", true)
cell(3, 2, "bold link"):
    span("link") s, n:
        ts.set_span_style_bits(s, n, 1)
        ts.set_link(s, n, "https://example.com/", false)
cell(0, 3, "https://example.com/cell-that-is-a-url"): nil

// Formulas and the grid they read.
cell(1, 3, "Numbers"):
    ts.create_grid(2, 3)
    for(6) i:
        ts.goto_child(i)
        ts.set_text([ "1", "2.5", "three", "-4", "", "1e2" ][i])
        ts.goto_parent()
cell(2, 3, ""): ts.set_formula("=SUM(Numbers) - 10")
cell(3, 3, ""): ts.set_formula("=UPPER(SUBSET(Numbers, 0, 1)) & \' / \' & COUNT(Numbers)")
cell(0, 4, ""): ts.set_formula("=AVG(Missing)")

// Notes, alignment, filtered text.
cell(1, 4, "has a note"): ts.set_note("The note, with \"quotes\", <angle brackets> & an ampersand.")
cell(2, 4, "left"): ts.set_text_alignment(1)
cell(3, 4, "center"): ts.set_text_alignment(2)
cell(0, 5, "right bottom"):
    ts.set_text_alignment(3)
    ts.set_vertical_alignment(3)
cell(1, 5, "middle"): ts.set_vertical_alignment(2)
cell(2, 5, "filtered"): ts.set_text_filtered(true)
cell(3, 5, "<xml> & \"quotes\" \'apostrophes\'    spaces"): nil

// Grids: folded, border color, column widths, nesting, an image.
cell(0, 6, "folded"):
    ts.create_grid(2, 2)
    ts.set_border_color(float4 { 0.5, 0.0, 0.0, 1.0 }, false)
    ts.goto_child(0)
    ts.set_text("inside a folded grid")
    ts.goto_parent()
    ts.set_folded(true)
cell(1, 6, "column widths"):
    ts.create_grid(3, 1)
    for(3) i:
        ts.goto_child(i)
        ts.set_text("wide text that wraps at the width of its column {i}")
        ts.set_column_width(40 + i * 100)
        ts.goto_parent()
cell(2, 6, "nested"):
    ts.create_grid(1, 2)
    ts.goto_child(1)
    ts.set_text("level 2")
    ts.create_grid(2, 1)
    ts.goto_child(1)
    ts.set_text("level 3")
    ts.create_grid(1, 1)
    ts.goto_child(0)
    ts.set_text("level 4")
cell(3, 6, "image"):
    assert ts.set_image(repo_path + "/TS/images/icon16.png")

ts.goto_root()
ts.goto_column_row(1, 0)
ts.select()
assert ts.save_document_as(out_path)
ts.agent_result("saved " + ts.get_filename())
