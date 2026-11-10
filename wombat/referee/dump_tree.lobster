// Walks the whole open document and hands every property the script API can read back as a
// stream of netstrings ("<bytes>:<data>"), which referee.py turns into JSON. Netstrings rather
// than JSON so that no cell text can break the framing and Lobster needs no escaping code.
//
// Per cell, in this order (see parse_cell() in referee.py):
//   text note formula celltype bg fg relsize stylebits textalign vertalign renderstyle
//   verticallayout filtered istag hasimage lastedit colwidth
//   nsegments, then per segment: start length stylebits color linktarget linkinternal
//   cols rows, and if cols > 0: folded bordercolor, then cols * rows cells (row by row)

import std

let out = []

def emit(s:string): out.push("{s.length}:{s}")
def emiti(i:int): emit("{i}")

// Colors arrive as floats 0..1 per channel.
def emitc(c):
    emit("{round(c.x * 255.0)},{round(c.y * 255.0)},{round(c.z * 255.0)}")

// The style, color and link of every character, merged into segments of equal ones. Positions
// are UTF-8 byte offsets, as the script API counts them.
def emit_segments(t:string):
    let segs = []
    var start = 0
    var last = ""
    var fields = []
    var i = 0
    while i < t.length:
        let sb = ts.get_span_style_bits(i)
        let col = ts.get_span_text_color(i)
        let target, internal = ts.get_link(i)
        let cols = "{round(col.x * 255.0)},{round(col.y * 255.0)},{round(col.z * 255.0)}"
        let key = "{sb} {cols} {internal} {target}"
        if key != last:
            if i > 0:
                segs.push([ "{start}", "{i - start}" ].append(fields))
            start = i
            last = key
            fields = [ "{sb}", cols, target, "{internal}" ]
        i++
        while i < t.length and (t[i] & 0xC0) == 0x80: i++
    if t.length > 0:
        segs.push([ "{start}", "{t.length - start}" ].append(fields))
    emiti(segs.length)
    for(segs) s:
        for(s) f: emit(f)

def dump_cell() -> void:
    let t = ts.get_text()
    emit(t)
    emit(ts.get_note())
    emit(ts.get_formula())
    emiti(ts.get_cell_type())
    emitc(ts.get_background_color())
    emitc(ts.get_text_color())
    emiti(ts.get_relative_size())
    emiti(ts.get_style_bits())
    emiti(ts.get_text_alignment())
    emiti(ts.get_vertical_alignment())
    emiti(ts.get_render_style())
    emiti(ts.is_vertical_layout())
    emiti(ts.is_text_filtered())
    emiti(ts.is_tag())
    emiti(ts.has_image())
    emiti(ts.get_last_edit())
    emiti(ts.get_column_width())
    emit_segments(t)
    let size = ts.num_columns_rows()
    emiti(size.x)
    emiti(size.y)
    if size.x > 0:
        emiti(ts.is_folded())
        emitc(ts.get_border_color())
        for(ts.num_children()) i:
            ts.goto_child(i)
            dump_cell()
            ts.goto_parent()

ts.goto_root()
let selsize, selpos = ts.selection()
emiti(ts.has_selection())
emit("{selpos.x},{selpos.y},{selsize.x},{selsize.y}")
let html, pdf = ts.get_autoexport()
emiti(html)
emiti(pdf)
dump_cell()
ts.agent_result(concat_string(out, ""))
