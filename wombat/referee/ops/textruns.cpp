// Prints what TreeSheets' own run and link bookkeeping (src/textruns.h, compiled as it is,
// without the app) does for a few thousand edits of a 7-character text, one JSON object per
// line. Wombat's commands are compared with this (crates/wombat-core/tests/textruns.rs).
// Built and run by make_ops.
//
// Text edits call the pair of functions that Text::InsertText, RemoveText and ReplaceText call
// in src/text.h; "style" is what Text::ToggleStyle and the script API do with a range, "color"
// is Text::SetRunColor, "link" is TextLinks::Set, "cellstyle" is Text::SetStyleAll and
// "uncolor" is Text::ClearRunColors.

#include <cstdio>
#include <string>

#include "textruns.h"

static const int kLen = 7;

static std::string Runs(const TextRuns &runs) {
    std::string s = "[";
    for (auto &r : runs.v) {
        if (s.size() > 1) s += ",";
        s += "[" + std::to_string(r.start) + "," + std::to_string(r.len) + "," +
             std::to_string(r.stylebits) + "," + (r.hascolor ? std::to_string(r.color) : "-1") + "]";
    }
    return s + "]";
}

static std::string Links(const TextLinks<std::string> &links) {
    std::string s = "[";
    for (auto &l : links.v) {
        if (s.size() > 1) s += ",";
        s += "[" + std::to_string(l.start) + "," + std::to_string(l.len) + ",\"" + l.target + "\"]";
    }
    return s + "]";
}

struct State {
    int base;
    TextRuns runs;
    TextLinks<std::string> links;
};

static void Print(const char *op, const State &before, int a, int b, int c, const char *target,
                  const State &after) {
    printf("{\"op\":\"%s\",\"base\":%d,\"runs\":%s,\"links\":%s,\"a\":%d,\"b\":%d,\"c\":%d,"
           "\"target\":\"%s\",\"out_base\":%d,\"out_runs\":%s,\"out_links\":%s}\n",
           op, before.base, Runs(before.runs).c_str(), Links(before.links).c_str(), a, b, c, target,
           after.base, Runs(after.runs).c_str(), Links(after.links).c_str());
}

int main() {
    const unsigned red = 0xFF0000, green = 0x00FF00;
    std::vector<State> states = {
        {0, {}, {}},
        {0, {{{2, 2, 1, 0, false}}}, {{{2, 2, false, "a"}}}},
        {0, {{{0, 3, 2, 0, false}, {3, 2, 1, 0, false}}}, {{{0, 3, false, "a"}, {3, 2, false, "b"}}}},
        {0, {{{1, 2, 1, 0, false}, {4, 2, 1, 0, false}}}, {{{1, 2, false, "a"}, {4, 2, false, "a"}}}},
        {0, {{{0, 7, 1, red, true}}}, {{{0, 7, false, "a"}}}},
        {0, {{{1, 2, 0, red, true}, {3, 2, 2, red, true}, {5, 2, 1, 0, false}}}, {{{5, 2, false, "b"}}}},
        {1, {{{2, 3, 0, 0, false}, {5, 1, 3, 0, false}}}, {{{0, 1, false, "a"}, {6, 1, false, "a"}}}},
        {1, {{{0, 2, 1, green, true}, {4, 3, 2, 0, false}}}, {{{1, 5, false, "b"}}}},
    };
    for (auto &s : states) {
        for (int pos = 0; pos <= kLen; pos++) {
            for (int n : {1, 3}) {
                auto t = s;
                t.runs.Insert(pos, n);
                t.links.Insert(pos, n);
                Print("insert", s, pos, n, 0, "", t);
            }
            for (int n = 1; pos + n <= kLen; n++) {
                auto t = s;
                t.runs.Remove(pos, n, t.base);
                t.links.Remove(pos, n);
                Print("remove", s, pos, n, 0, "", t);
                for (int m : {1, 2, 4}) {
                    auto u = s;
                    u.runs.Replace(pos, n, m, u.base);
                    u.links.Replace(pos, n, m);
                    Print("replace", s, pos, n, m, "", u);
                }
                // Switch style bits on (b) and off (c) in a range.
                for (auto [set, clear] : {std::pair {1, 0}, {0, 1}, {2, 1}, {0, 31}, {31, 0}}) {
                    auto u = s;
                    u.runs.Modify(pos, pos + n, kLen, u.base, [&](TextRun &r) {
                        r.stylebits = (r.stylebits | set) & ~clear;
                    });
                    Print("style", s, pos, n, set * 32 + clear, "", u);
                }
                for (auto color : {red, green}) {
                    auto u = s;
                    u.runs.Modify(pos, pos + n, kLen, u.base, [&](TextRun &r) {
                        r.hascolor = true;
                        r.color = color;
                    });
                    Print("color", s, pos, n, static_cast<int>(color), "", u);
                }
                for (auto target : {"a", "b", ""}) {
                    auto u = s;
                    u.links.Set(pos, pos + n, kLen, false, target);
                    Print("link", s, pos, n, 0, target, u);
                }
            }
        }
        for (int bit : {1, 2}) {
            for (int set : {0, 1}) {
                auto t = s;
                t.base = set ? (t.base | bit) : (t.base & ~bit);
                for (auto &r : t.runs.v) r.stylebits = set ? (r.stylebits | bit) : (r.stylebits & ~bit);
                t.runs.Normalize(t.base);
                Print("cellstyle", s, bit, set, 0, "", t);
            }
        }
        auto t = s;
        for (auto &r : t.runs.v) r.hascolor = false;
        t.runs.Normalize(t.base);
        Print("uncolor", s, 0, 0, 0, "", t);
    }
    return 0;
}
