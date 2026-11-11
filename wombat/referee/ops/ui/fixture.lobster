// Builds ui/fixture.cts, the document that every case in ui/cases/ starts from: one column of
// six cells, each with a grid for one family of operations (places as `wombat edit` names them).
// Run by make_ui_ops --fixture, which puts lines that define out_path and image_path in front.
//
//   /1,1 sort    3 x 8   pear 10 x / Apple 9 y / apple 100 z / (empty) 2 w / Éclair 1 v /
//                        banana 9 u / apple 9 Y / éclair 1 t
//                        (Apple bold; "ban" of banana red, banana has a grid b1 b2; the last
//                        apple yellow; rows 2 and 7 are equal but for letter case)
//   /1,2 table   3 x 6   Fruit Apple red / Fruit Apple green / Veg Leek green /
//                        Fruit Pear green / Fruit Fig (empty) / Veg Leek white
//                        (the second Fruit has a note, Pear is bold, the first green is green,
//                        column 2 is 60 wide)
//   /1,3 tree    1 x 3   Fruit [Apple [red, green], Pear] / Veg [Apple, Leek] / solo
//                        (Fruit bold, red is red, both Apples have notes)
//   /1,4 block   3 x 3   a1 b1 c1 / a2 (empty) c2 / a3 b3 c3
//                        (b3 has a grid x y; "1" of a1 italic; c2 blue; column 3 is 50 wide;
//                        a3 has a note; c3 is bigger)
//   /1,5 chain   1 x 2   P [Q [T [t1, t2]]] / R [T, S]      (the T in R has a note)
//   /1,6 misc    3 x 2   todo "☐ open" "☑ done" / big img "wide text here"
//                        (big is 2 bigger, img shows an image, todo is green on black)

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

def span(part, f):
    let start = find_string(ts.get_text(), part)
    assert start >= 0
    f(start, part.length)

let red = float4 { 1.0, 0.0, 0.0, 1.0 }
let green = float4 { 0.0, 0.5, 0.0, 1.0 }
let yellow = float4 { 1.0, 1.0, 0.5, 1.0 }
let blue = float4 { 0.5, 0.5, 1.0, 1.0 }

ts.new_document(1, 6)
for([ "sort", "table", "tree", "block", "chain", "misc" ]) t, i:
    ts.goto_root()
    ts.goto_column_row(0, i)
    ts.set_text(t)

at([ 0, 0 ]):
    fill(3, [ "pear", "10", "x", "Apple", "9", "y", "apple", "100", "z", "", "2", "w",
              "Éclair", "1", "v", "banana", "9", "u", "apple", "9", "Y", "éclair", "1", "t" ])
at([ 0, 0, 0, 1 ]): ts.set_style_bits(1)
at([ 0, 0, 0, 5 ]):
    span("ban") s, n: ts.set_span_text_color(s, n, red)
    fill(1, [ "b1", "b2" ])
at([ 0, 0, 0, 6 ]): ts.set_background_color(yellow, false)

at([ 0, 1 ]):
    fill(3, [ "Fruit", "Apple", "red", "Fruit", "Apple", "green", "Veg", "Leek", "green",
              "Fruit", "Pear", "green", "Fruit", "Fig", "", "Veg", "Leek", "white" ])
at([ 0, 1, 0, 1 ]): ts.set_note("second fruit")
at([ 0, 1, 1, 3 ]): ts.set_style_bits(1)
at([ 0, 1, 2, 1 ]): ts.set_text_color(green, false)
at([ 0, 1, 1, 0 ]): ts.set_column_width(60)

at([ 0, 2 ]): fill(1, [ "Fruit", "Veg", "solo" ])
at([ 0, 2, 0, 0 ]):
    ts.set_style_bits(1)
    fill(1, [ "Apple", "Pear" ])
at([ 0, 2, 0, 0, 0, 0 ]):
    ts.set_note("fruit apple")
    fill(1, [ "red", "green" ])
at([ 0, 2, 0, 0, 0, 0, 0, 0 ]): ts.set_text_color(red, false)
at([ 0, 2, 0, 1 ]): fill(1, [ "Apple", "Leek" ])
at([ 0, 2, 0, 1, 0, 0 ]): ts.set_note("veg apple")

at([ 0, 3 ]): fill(3, [ "a1", "b1", "c1", "a2", "", "c2", "a3", "b3", "c3" ])
at([ 0, 3, 0, 0 ]): span("1") s, n: ts.set_span_style_bits(s, n, 2)
at([ 0, 3, 2, 1 ]): ts.set_background_color(blue, false)
at([ 0, 3, 2, 0 ]): ts.set_column_width(50)
at([ 0, 3, 0, 2 ]): ts.set_note("a note on a3")
at([ 0, 3, 2, 2 ]): ts.set_relative_size(1)
at([ 0, 3, 1, 2 ]): fill(2, [ "x", "y" ])

at([ 0, 4 ]): fill(1, [ "P", "R" ])
at([ 0, 4, 0, 0 ]): fill(1, [ "Q" ])
at([ 0, 4, 0, 0, 0, 0 ]): fill(1, [ "T" ])
at([ 0, 4, 0, 0, 0, 0, 0, 0 ]): fill(1, [ "t1", "t2" ])
at([ 0, 4, 0, 1 ]): fill(1, [ "T", "S" ])
at([ 0, 4, 0, 1, 0, 0 ]): ts.set_note("second")

at([ 0, 5 ]): fill(3, [ "todo", "☐ open", "☑ done", "big", "img", "wide text here" ])
at([ 0, 5, 0, 0 ]):
    ts.set_background_color(float4 { 0.0, 0.0, 0.0, 1.0 }, false)
    ts.set_text_color(float4 { 0.0, 1.0, 0.0, 1.0 }, false)
at([ 0, 5, 0, 1 ]): ts.set_relative_size(2)
at([ 0, 5, 1, 1 ]): assert ts.set_image(image_path)

ts.goto_root()
ts.goto_column_row(0, 0)
ts.select()
assert ts.save_document_as(out_path)
ts.agent_result("saved " + ts.get_filename())
