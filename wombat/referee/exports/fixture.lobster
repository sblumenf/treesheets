// Builds fixture.cts, the document whose grids the real TreeSheets exports for golden/: one
// column of cells, each named after a case and holding the grid of that case.
// Run by make_exports --fixture, which puts a line that defines out_path in front.
//
//   plain      3 x 2   a b c / d e f
//   quotes     2 x 4   texts with double quotes, commas, semicolons, single quotes, spaces
//                      at either end
//   empties    3 x 3   only the first cell has text: empty rows and columns at the end
//   blank      1 x 2   two empty cells: rows that are nothing but one empty field
//   unicode    2 x 2   accents, CJK, an emoji, a checkbox
//   risky      2 x 4   texts that a spreadsheet would run: = + - @ at the start, and numbers
//   nested     2 x 3   cells with grids inside, with and without text of their own
//   formula    2 x 2   two formula cells, which export as what they show
//   tree       1 x 3   an outline three levels deep, all grids of one column
//   wide       3 x 2   a table with a table inside, for the flattened export
import std
import vec

// Gives the current cell a grid with these texts, row by row.
def fill(cols, texts):
    ts.create_grid(cols, texts.length / cols)
    for(texts) t, i:
        ts.goto_child(i)
        ts.set_text(t)
        ts.goto_parent()

// Goes to a cell by column and row numbers from the root, runs f there.
def at(path, f):
    ts.goto_root()
    for(path.length / 2) i: ts.goto_column_row(path[i * 2], path[i * 2 + 1])
    f()

let names = [ "plain", "quotes", "empties", "blank", "unicode", "risky", "nested", "formula",
              "tree", "wide" ]
ts.new_document(1, names.length)
for(names) t, i:
    ts.goto_root()
    ts.goto_column_row(0, i)
    ts.set_text(t)

at([ 0, 0 ]): fill(3, [ "a", "b", "c", "d", "e", "f" ])
at([ 0, 1 ]):
    fill(2, [ "say \"hi\"", "\"quoted\"", "a,b", "x;y", " lead", "trail ", "it\'s", "a\"b" ])
at([ 0, 2 ]): fill(3, [ "only", "", "", "", "", "", "", "", "" ])
at([ 0, 3 ]): fill(1, [ "", "" ])
at([ 0, 4 ]): fill(2, [ "Éclair à la crème", "日本語", "smile 😀", "☐ open" ])
at([ 0, 5 ]): fill(2, [ "=1+1", "+1", "-1", "@x", "-cmd|\' /C calc\'!A0", "=SUM(A)", "-2.5", "1-2" ])
at([ 0, 6 ]): fill(2, [ "Fruit", "has text", "", "Veg", "last", "" ])
at([ 0, 6, 0, 0 ]): fill(2, [ "Apple", "red", "Pear", "green" ])
at([ 0, 6, 0, 1 ]): fill(1, [ "in a cell without text" ])
at([ 0, 6, 1, 1 ]): fill(1, [ "Leek", "Kale" ])
at([ 0, 6, 1, 1, 0, 0 ]): fill(1, [ "green" ])
at([ 0, 7 ]): fill(2, [ "10", "32", "", "" ])
at([ 0, 7, 0, 1 ]): ts.set_formula("=1+2")
at([ 0, 7, 1, 1 ]): ts.set_formula("=LEN(\'abcd\') * 10")
at([ 0, 8 ]): fill(1, [ "Projects", "Notes", "" ])
at([ 0, 8, 0, 0 ]): fill(1, [ "Apollo", "Borealis" ])
at([ 0, 8, 0, 0, 0, 0 ]): fill(1, [ "Design", "Build" ])
at([ 0, 8, 0, 1 ]): fill(1, [ "one note" ])
at([ 0, 9 ]): fill(3, [ "Apollo", "Ana", "", "Borealis", "Chidi", "", "Cygnus", "Eve", "none" ])
at([ 0, 9, 2, 0 ]): fill(2, [ "Design", "done", "Build", "open" ])
at([ 0, 9, 2, 1 ]): fill(2, [ "Survey", "open" ])

ts.goto_root()
ts.goto_column_row(0, 0)
ts.select()
assert ts.save_document_as(out_path)
ts.agent_result("saved " + ts.get_filename())
