"""The cases of make_formulas: hand-written ones for every function, operation and error path of
TreeSheets' src/evaluator.h, and random ones from a grammar with a fixed seed.

See app.py for how a document is written. Nothing here needs the app.
"""
import random

from app import OPERATIONS

SEED = 20261111


def D(text=""):
    return [str(text), 0]


def F(formula):
    return [formula, 6]


def K(text, kind):
    return [text, kind]


def G(cell, rows):
    """`cell` with a grid made of rows (lists of cells or plain texts)."""
    cells = [c if isinstance(c, list) else D(c) for row in rows for c in row]
    return [cell[0], cell[1], len(rows[0]), len(rows), cells]


def T(name, *column):
    """A data cell named `name` with a grid of one column."""
    return G(D(name), [[c] for c in column])


def root(rows):
    return G(D(""), rows)


def column(cells):
    return root([[c] for c in cells])


# ---- formulas, by hand ----------------------------------------------------------------------

def world():
    """Tables that the hand-written formulas refer to."""
    return [
        T("Nums", "1", "2.5", "-3", "", "abc", "4e2"),
        T("One", "7"),
        T("Word", "hello"),
        T("Empty", ""),
        T("Words", "x", "y"),
        T("Unit price", "5"),
        T("Sales, 2025", "10", "20"),
        T("Größe", "3"),
        T("日本 語", "8"),
        T("x_1", "9"),
        T("T)", "1"),
        T("it's", "2"),
        T("Zeros", "0", "-0", "0.0"),
        T("Odd", "007", " 4 ", "0x10", ".5", "5.", "+6", "1,000", "12abc", "1e400", "1e-400",
          "0e5", "0-5", "inf", "nan", "١٢", "5 "),
        T("Big", "1e15", "123456789.123456789", "99999999999999999999", "1e22", "-1e-11",
          "0.1", "0.2", "1e300", "1e300"),
        T("Even", "4", "1", "3", "2"),
        T("Neg", "-1", "-5", "-0.5"),
        T("Text007", "007"),
        T("Padded", "  padded  "),
        G(D("Table"), [["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"], ["a", "", "10"]]),
        G(D("Tree"), [["1", T("Sub", "2", "3", T("Deep", "4", "x"))],
                      [F("=SUM(Sub)"), T("Sub2", "10", F("=1+1"))]]),
        T("Errs", "1", F("=1/0"), "2", F("=Missing")),
        T("Fs", F("=1+1"), F("=One*2"), F("='text'")),
        K("Var", 2),
        G(K("VarGrid", 2), [["v1", "v2"]]),
        D("NoGrid"),
        T("", "42"),
        T("dup", "1"),
        G(D("Outer"), [[T("dup", "2"), F("=dup")], [T("Scope", "5"), F("=SUM(Nums)")]]),
        T("#NAME?", "6"),
        T("SUM", "11"),
        T("e", "12"),
        T("A1", "13"),
        T("1st", "14"),
        T("a", "15"),
        T("A", "16"),
    ]


FORMULAS = [
    # numbers and arithmetic
    "=1", "=1+2", "=1 + 2 * 3", "=(1+2)*3", "=10/4", "=1/3", "=2/3", "=-1/3", "=1-1", "=0", "=-0",
    "=0*-1", "=-5", "=--5", "=-+-5", "=+5", "=- 5", "=1e3", "=1E3", "=1e+3", "=1e-3", "=.5", "=5.",
    "=0.1+0.2", "=1e15*1000", "=1e22+1", "=1e308*10", "=1e308+1e308", "=-1e308*10", "=1e400",
    "=1e-400", "=1e-320*1e10", "=0.00000000001", "=0.00000000005", "=0.000000000049",
    "=-0.00000000001", "=123456789.123456789", "=1.23456789012345", "=99999999999999999999",
    "=05", "=007", "=00.5", "=0.", "=0e5", "=0.0", "=1e", "=1e+", "=1.2.3", "=.", "=..5", "=1.",
    "=1/0", "=0/0", "=1/(1-1)", "=1/0+Missing", "=Missing+1/0", "=5/-0", "=2 2", "=2 +", "=* 2",
    "=(1", "=1)", "=()", "=", "= ", "==1", "=1=1", "=1,2", "=1;2", "=2^3", "=5%", "=1<2",
    "=((((((((((1))))))))))", "=1+2+3+4+5+6+7+8+9+10", "=2*3*4/5/6", "=1 - 2 - 3", "=8/4/2",
    "=1e2e3", "=0x10", "=1_000", "=1 000", "=3e", "=5e-1", "=1.5e+2", "=1e-10", "=1e-11",
    "=100000000000000000000000", "=0.30000000000000004", "=2.5-2.5", "=-(2.5-2.5)", "=1e16+1",
    # spaces
    "= 1 +　2", "=1 +2", "=1 +2", "=1 +2", "=　1", "=1 ", "=1\t+2",
    # names
    "=One", "=One*2", "=one", "=ONE", "=Word", "=Empty", "=Words", "=Nums", "=Unit price",
    "=Unit price * 2", "=Unit  price", "=Unit price*One", "=Größe", "=Größe*2", "=日本 語",
    "=x_1", "=x_1+1", "=\"Sales, 2025\"", "=\"One\"", "=\"One\"*2", "=\"T)\"", "=\"it's\"",
    "=\"\"", "=\"Missing\"", "=\"One", "=One\"", "=Missing", "=NoGrid", "=Var", "=VarGrid", "=v1",
    "=Sub", "=Deep", "=Inner", "=dup", "=Scope", "=Tree", "=Table", "=Zeros", "=Text007",
    "=Text007+0", "=Text007&''", "=Padded", "=Padded&'|'", "=e", "=A1", "=1st", "=a", "=A",
    "=a+A", "=SUM", "=\"#NAME?\"", "=#NAME?", "=One One", "=One(", "=One)", "=_", "=é",
    # aggregates
    "=SUM(Nums)", "=MIN(Nums)", "=MAX(Nums)", "=AVG(Nums)", "=AVERAGE(Nums)", "=MEDIAN(Nums)",
    "=COUNT(Nums)", "=sum(Nums)", "=Sum(Nums)", "=SUM (Nums)", "=SUM( Nums )", "=SUM(Nums )",
    "=SUM(\"Nums\")", "=SUM( \"Nums\" )", "=SUM(\"Nums\" )", "=SUM(\"Nums\"x)", "=SUM(nums)",
    "=SUM(One)", "=SUM(Word)", "=SUM(Empty)", "=COUNT(Empty)", "=MIN(Empty)", "=MAX(Words)",
    "=AVG(Empty)", "=MEDIAN(Empty)", "=MEDIAN(One)", "=MEDIAN(Even)", "=MEDIAN(Neg)",
    "=MIN(Neg)", "=MAX(Neg)", "=MIN(Zeros)", "=MAX(Zeros)", "=SUM(Zeros)", "=SUM(Odd)",
    "=COUNT(Odd)", "=MIN(Odd)", "=MAX(Odd)", "=SUM(Big)", "=AVG(Big)", "=MAX(Big)", "=MIN(Big)",
    "=SUM(Missing)", "=SUM()", "=SUM(", "=SUM(Nums", "=SUM)", "=SUM(Nums))", "=SUM(Nums)(",
    "=SUM(Nums, One)", "=SUM(1)", "=SUM(1+2)", "=SUM(SUM(Nums))", "=SUM(Table)", "=COUNT(Table)",
    "=SUM(Tree)", "=COUNT(Tree)", "=SUM(Sub)", "=SUM(Deep)", "=SUM(Errs)", "=COUNT(Errs)",
    "=SUM(Fs)", "=COUNT(Fs)", "=MAX(Fs)", "=SUM(NoGrid)", "=SUM(Var)", "=SUM(VarGrid)",
    "=SUM(v1)", "=SUM(\"\")", "=SUM(dup)", "=SUM(Unit price)", "=SUM(Sales, 2025)",
    "=SUM(\"Sales, 2025\")", "=SUM(T))", "=SUM(\"T)\")", "=SUM(Größe)", "=SUM(日本 語)",
    "=SUM(Nums)+SUM(One)", "=SUM(Nums) - MAX(Nums) / COUNT(Nums)", "=-SUM(Nums)",
    "=SUM(Nums)&COUNT(Nums)", "=STDEV(Nums)", "=FOO(Nums)", "=SUMX(Nums)", "=TREE(Nums)",
    "=SUM(#NAME?)", "=SUM(Outer)", "=SUM(Scope)", "=COUNT(Text007)", "=SUM(Padded)",
    # TREE
    "=TREESUM(Tree)", "=TREECOUNT(Tree)", "=TREEMIN(Tree)", "=TREEMAX(Tree)", "=TREEAVG(Tree)",
    "=TREEAVERAGE(Tree)", "=TREEMEDIAN(Tree)", "=treesum(Tree)", "=TreeSum(Tree)",
    "=TREESUM(Sub)", "=TREESUM(Nums)", "=TREESUM(Errs)", "=TREESUM(Fs)", "=TREESUM(Outer)",
    "=TREESUM(Missing)", "=TREESUM(One)", "=TREECOUNT(Outer)", "=TREE SUM(Tree)",
    "=TREESUM(Sub2)", "=TREECOUNT(Sub2)",
    # SUBSET
    "=SUM(SUBSET(Table, 0, 0))", "=SUM(SUBSET(Table, 0, -))", "=SUM(SUBSET(Table, -, 0))",
    "=SUM(SUBSET(Table, -, -))", "=SUM(SUBSET(Table, 1-2, 0-1))", "=SUM(SUBSET(Table, 1-, 2-))",
    "=SUM(SUBSET(Table, -1, -1))", "=SUM(SUBSET(Table, 5, 0))", "=SUM(SUBSET(Table, 0, 9))",
    "=SUM(SUBSET(Table, 2-1, 0))", "=SUM(SUBSET(Table, 0-99, 0-99))", "=SUBSET(Table, 1, 1)",
    "=SUBSET(Table, 0, 3)", "=SUBSET(Table, 1, 3)", "=SUBSET(Table, 0-1, 0)",
    "=SUBSET(Table, 1, 1)*2", "=COUNT(SUBSET(Table, -, -))", "=MAX(SUBSET(Table,2,-))",
    "=SUM(SUBSET(\"Table\", 0, 0))", "=SUM(SUBSET(\"Sales, 2025\", 0, 1))", "=SUM(subset(Table,0,0))",
    "=SUM(SUBSET (Table,0,0))", "=SUM(SUBSET(Table,0))", "=SUM(SUBSET(Table))", "=SUM(SUBSET())",
    "=SUM(SUBSET(Table,0,0,0))", "=SUM(SUBSET(Table,a,0))", "=SUM(SUBSET(Table,0,0)",
    "=SUM(SUBSET(Table,,0))", "=SUM(SUBSET(Table, 0 - 1 , 0 ))", "=SUM(SUBSET(Missing,0,0))",
    "=SUM(SUBSET(NoGrid,0,0))", "=SUM(SUBSET(Var,0,0))", "=SUM(SUBSET(Table,99999999999,0))",
    "=SUM(SUBSET(Table,99999999999999999999999,0))", "=SUM(SUBSET(Table,99999999999999999999999-,0))",
    "=SUM(SUBSET(Table,0,-99999999999999999999999))", "=SUM(SUBSET(Table, 01, 00))",
    "=SUM(SUBSET(Tree, 1, 0))", "=TREESUM(SUBSET(Tree, 1, 0))", "=TREESUM(SUBSET(Tree, -, -))",
    "=SUM(SUBSET(Tree, -, -))", "=COUNT(SUBSET(Tree, 0, 1))", "=SUBSET(Tree, 0, 1)",
    "=SUM(SUBSET(SUBSET(Tree, 1, 0), 0, 0-1))", "=SUM(SUBSET(SUBSET(Tree, 1, -), 0, -))",
    "=SUM(SUBSET(SUBSET(Tree, 0, 0), 0, 0))", "=SUM(SUBSET(SUBSET(SUBSET(Tree,1,0),0,2),0,-))",
    "=SUBSET(SUBSET(Tree, 1, 0), 0, 0)", "=SUM(SUBSET(SUBSET(Missing,0,0),0,0))",
    "=SUM(SUBSET(SUBSET(Tree,1,0),0))", "=SUM(SUBSET(Errs,0,0))", "=SUM(SUBSET(Errs,0,-))",
    "=SUM(SUBSET(Errs,0,2-))", "=SUBSET(Errs,0,1)", "=SUBSET(Table,0,0)&SUBSET(Table,0,3)",
    # TREESUBSET
    "=SUM(TREESUBSET(Tree, -, -, 0))", "=SUM(TREESUBSET(Tree, -, -, 1))",
    "=SUM(TREESUBSET(Tree, -, -, 0-1))", "=SUM(TREESUBSET(Tree, -, -, -))",
    "=SUM(TREESUBSET(Tree, -, -, 1-))", "=SUM(TREESUBSET(Tree, -, -, 2))",
    "=SUM(TREESUBSET(Tree, 0, -, -))", "=SUM(TREESUBSET(Tree, 1, 0, 0-2))",
    "=COUNT(TREESUBSET(Tree, -, -, -))", "=TREESUM(TREESUBSET(Tree, -, -, 0))",
    "=TREESUBSET(Tree, 0, 0, 0)", "=SUM(TREESUBSET(Tree, -, -))", "=SUM(TREESUBSET(Tree,-,-,-,-))",
    "=SUM(TREESUBSET(Table, -, -, -))", "=SUM(treesubset(Tree,-,-,1))",
    "=SUM(TREESUBSET(SUBSET(Tree,1,0),-,-,-))", "=SUM(SUBSET(TREESUBSET(Tree,-,-,-),0,0))",
    "=SUM(TREESUBSET(Missing,-,-,-))", "=SUM(TREESUBSET(Tree,-,-,9-))",
    # text
    "='abc'", "='it''s'", "=''", "='abc", "='a'&'b'", "='a' & 'b' & 'c'", "='a'&1", "=1&2",
    "=1&2+3", "=1+2&3", "='1'+1", "='1'+'2'", "=' 5 '+1", "='abc'+1", "='0x10'+1", "=''+1",
    "='1e3'*2", "=-'5'", "=+'5'", "=+'x'", "='a'*'b'", "='10'/'4'", "='1'/'0'", "=One&Word",
    "=Word&One", "=Word & ' ' & Word", "=One&''", "=(1/3)&''", "=1/4&''", "=Word+1",
    "=Nums&''", "=Missing&'x'", "=1/0&'x'", "='x'&1/0", "=1e308*10&''", "='a''", "='''", "=''''",
    "='é😀'", "='a\"b'", "=\"a'b\"", "='=1+1'", "='#NAME?'", "='a' 'b'", "='a'b", "=('a')",
    "=('a'&'b')&'c'", "=-0&''", "=0.1+0.2&''", "=1e15*1000&''", "=Text007&'!'", "=Text007*1&'!'",
    # text functions
    "=LEN('abc')", "=LEN('')", "=LEN('é😀')", "=LEN(Word)", "=LEN(One)", "=LEN(123)", "=LEN(1/3)",
    "=LEN(Nums)", "=LEN(Missing)", "=LEN()", "=LEN('a','b')", "=LEN('abc'", "=len('abc')",
    "=LEN ('abc')", "=LEN(1/0)", "=LEN(1e308*10)", "=LEN(Padded)", "=LEN(Text007)",
    "=LEFT('abcdef', 2)", "=LEFT('abcdef', 0)", "=LEFT('abcdef', 99)", "=LEFT('abcdef', -1)",
    "=LEFT('abcdef', 2.9)", "=LEFT('abcdef', '2')", "=LEFT('abcdef', 'x')", "=LEFT('abcdef')",
    "=LEFT('abcdef', 1, 2)", "=LEFT('é😀x', 2)", "=LEFT(Word, One)", "=LEFT(Word, 1e20)",
    "=LEFT(Word, 1/0)", "=LEFT(12345, 2)", "=LEFT(12345, 2)+1",
    "=RIGHT('abcdef', 2)", "=RIGHT('abcdef', 0)", "=RIGHT('abcdef', 99)", "=RIGHT('abcdef', -1)",
    "=RIGHT('é😀x', 2)", "=RIGHT(Word, 3)", "=RIGHT('abc', 1e10)",
    "=MID('abcdef', 2, 3)", "=MID('abcdef', 1, 1)", "=MID('abcdef', 0, 1)", "=MID('abcdef', 6, 5)",
    "=MID('abcdef', 7, 1)", "=MID('abcdef', 99, 1)", "=MID('abcdef', 2, 0)", "=MID('abcdef', 2, -1)",
    "=MID('abcdef', -1, 2)", "=MID('abcdef', 2)", "=MID('abcdef', 1.9, 2.9)", "=MID('é😀x', 2, 1)",
    "=MID(Word, 2, 2)", "=MID('abcdef', 0.5, 1)", "=MID('abcdef', 0, -1)",
    "=UPPER('abc')", "=UPPER('aBc é ß ǆ ᾀ ŉ ﬁ')", "=UPPER(Word)", "=UPPER(1)", "=UPPER()",
    "=LOWER('ABC')", "=LOWER('ÀÉ Σ İ ǅ ẞ')", "=LOWER(Word)", "=upper('x')", "=Lower('X')",
    "=TRIM('  a  ')", "=TRIM('a')", "=TRIM('')", "=TRIM('   ')", "=TRIM(' a b ')&'|'",
    "=TRIM('　a　')&'|'", "=TRIM(' a')&'|'", "=TRIM(Padded)&'|'", "=TRIM(1)",
    "=SUBSTITUTE('banana', 'a', 'o')", "=SUBSTITUTE('banana', 'an', '')",
    "=SUBSTITUTE('banana', '', 'x')", "=SUBSTITUTE('banana', 'x', 'y')",
    "=SUBSTITUTE('aaa', 'aa', 'b')", "=SUBSTITUTE('banana', 'a')", "=SUBSTITUTE('banana')",
    "=SUBSTITUTE('a.b.c', '.', 1/4)", "=SUBSTITUTE(Word, 'l', One)", "=SUBSTITUTE(12321, 2, 'x')",
    "=SUBSTITUTE('abc', 'b', 1/0)", "=SUBSTITUTE('abc', '', 1/0)", "=SUBSTITUTE('abc', 'B', 'x')",
    "=UPPER(LEFT(Word, 1))&MID(Word, 2, 99)", "=LEN(UPPER(TRIM('  ab  ')))",
    "=LEN('a')+LEN('bc')", "=LEFT('abc', LEN('ab'))", "=LEN(SUM(Nums))", "=LEN(SUM(Nums)&'')",
    "=LEFT(SUM(Nums), 2)", "=LEFT(One, 1)", "=LEFT('abc', SUM(One))", "=LEN(LEN(LEN('abc')))",
    # nesting and length
    "=" + "(" * 30 + "1" + ")" * 30, "=" + "-" * 40 + "1", "=" + "+".join(["1"] * 200),
    "=" + "LEN(" * 20 + "'ab'" + ")" * 20, "=" + "&".join(["'ab'"] * 100),
    "=" + "(" * 60 + "1" + ")" * 60, "=" + "(1+" * 50 + "1" + ")" * 50,
]


def formula_documents():
    """Documents of one column: the world, then formulas. Yields (name, document)."""
    size = 60
    for n in range(0, len(FORMULAS), size):
        yield f"hand_{n // size:02}", column(world() + [F(f) for f in FORMULAS[n:n + size]])
    yield from special_documents()


def special_documents():
    # Cycles, references between formulas, and their order.
    yield "cycle_self", root([[T("A", F("=SUM(A)"), "1")], [F("=SUM(A)")], [F("=A")]])
    yield "cycle_self_named", column([T("A", "5", F("=A")), F("=SUM(A)")])
    yield "cycle_two", column([T("A", F("=SUM(B)"), "1"), T("B", F("=SUM(A)"), "2"),
                               F("=SUM(A)"), F("=SUM(B)"), F("=SUM(A)+SUM(B)")])
    yield "cycle_two_reversed", column([F("=SUM(A)+SUM(B)"), F("=SUM(B)"), F("=SUM(A)"),
                                        T("B", F("=SUM(A)"), "2"), T("A", F("=SUM(B)"), "1")])
    yield "cycle_three", column([T("A", F("=SUM(B)")), T("B", F("=SUM(C)")), T("C", F("=SUM(A)")),
                                 F("=SUM(A)"), F("=SUM(C)")])
    yield "cycle_with_error", column([
        T("X", F("=1/0")), T("A", F("=SUM(X)+SUM(B)")), T("B", F("=SUM(X)+SUM(A)")),
        F("=SUM(A)"), F("=SUM(B)")])
    yield "cycle_error_order", column([
        T("B", F("=SUM(X)+SUM(A)")), T("A", F("=SUM(B)+SUM(X)")), T("X", F("=1/0")),
        F("=SUM(A)"), F("=SUM(B)")])
    yield "cycle_tail", column([T("A", F("=SUM(B)"), "1"), T("B", F("=SUM(A)"), "2"),
                                T("C", F("=SUM(A)"), "3"), F("=SUM(C)"), F("=COUNT(C)")])
    yield "cycle_in_one_grid", column([T("A", F("=SUM(A)+1"), F("=SUM(A)+2"), "3"), F("=SUM(A)")])
    yield "cycle_tree", column([G(D("A"), [["1", T("B", "2", F("=TREESUM(A)"))]]),
                                F("=TREESUM(A)"), F("=SUM(B)")])
    yield "tree_skips_formulas", column([
        G(D("A"), [["1", F("=10+10")], [T("B", "2", F("=100*2"), T("C", F("=5"), "3")), "4"]]),
        F("=TREESUM(A)"), F("=SUM(A)"), F("=TREESUM(B)"), F("=TREECOUNT(A)"), F("=SUM(B)"),
        F("=TREESUM(SUBSET(A, -, -))"), F("=SUM(TREESUBSET(A, -, -, -))")])
    yield "chain_forward", column(
        [T("C0", "1")] + [T(f"C{i}", F(f"=SUM(C{i - 1})+1")) for i in range(1, 40)]
        + [F("=SUM(C39)")])
    yield "chain_backward", column(
        [F("=SUM(C39)")] + [T(f"C{i}", F(f"=SUM(C{i - 1})+1")) for i in range(39, 0, -1)]
        + [T("C0", "1")])
    yield "errors_carry", column([
        T("A", "1", F("=1/0"), F("=Missing")), T("B", F("=Missing"), F("=1/0")),
        F("=SUM(A)"), F("=SUM(B)"), F("=SUM(A)+SUM(B)"), F("=SUM(B)+SUM(A)"), F("=COUNT(A)"),
        T("C", F("=SUM(A)")), F("=SUM(C)"), F("=C"), F("=C&'x'"), F("=LEN(C)"),
        T("N", F("=1e308*10")), F("=SUM(N)"), T("V", F("='a'+1")), F("=SUM(V)"),
        T("E", F("=(")), F("=SUM(E)"), T("Y", F("=SUM(Var)")), K("Var", 2), F("=SUM(Y)")])
    yield "formula_text_results", column([
        T("A", F("='12'"), F("='x'"), F("=''"), F("=' 3 '"), F("='0x10'")),
        F("=SUM(A)"), F("=COUNT(A)"), F("=A"), T("B", F("='7'")), F("=B"), F("=B+1"), F("=B&B"),
        T("C", F("='x'")), F("=C"), F("=SUM(C)"), F("=C+1")])
    # Names: nearest first, breadth first, a cell and what is around it.
    yield "names_nearest", root([
        [T("N", "1"), G(D("Box"), [[T("N", "2"), F("=N")], [G(D("In"), [[T("N", "3"), F("=N")]]),
                                                           F("=N")]])],
        [F("=N"), G(D("Other"), [[F("=N"), G(D("In2"), [[F("=N")]])]])]])
    yield "names_breadth_first", root([
        [G(D("a"), [[G(D("b"), [[T("N", "3")]])]]), T("N", "1")],
        [F("=N"), G(D("c"), [[F("=N")]])]])
    yield "names_breadth_first_2", root([
        [G(D("a"), [[G(D("b"), [[T("N", "3")]]), T("N", "2")]]), F("=N")],
        [G(D("c"), [[T("N", "4")]]), F("=N")]])
    yield "names_parent_itself", column([
        G(D("N"), [["5", F("=SUM(N)")], [T("N", "1"), F("=N")]]), F("=SUM(N)")])
    yield "names_own_grid_first", column([
        T("N", "100"), G(D("P"), [[F("=N"), T("N", "1")]]),
        G(D("Q"), [[G(D("R"), [[F("=N")]]), T("N", "2")]])])
    yield "names_exact", column([
        T("Name", "1"), T("name", "2"), T(" Name", "3"), T("Name ", "4"), T("NAME", "5"),
        F("=Name"), F("=name"), F("=NAME"), F("=\" Name\""), F("=\"Name \""), F("=SUM(Name )"),
        F("=SUM( Name)"), F("=SUM(\"Name \")"), F("=Name ")])
    yield "names_not_assignments", column([
        G(K("N", 2), [["a", "b"]]), F("=N"), F("=SUM(N)"), F("=a"), F("=b"), F("=c"),
        G(D("Box"), [[T("N", "7"), F("=N")]]), G(K("x", 2), [[K("y", 2)]]), F("=x"), F("=y")])
    yield "names_other_kinds", column([
        G(K("+", 1), [["1"]]), G(K("R", 4), [["2"]]), G(K("H", 3), [["3"]]),
        G(K("V", 5), [["4"]]), F("=\"+\""), F("=R"), F("=H"), F("=V"), F("=SUM(R)+SUM(H)")])
    yield "names_unicode", column([
        T("Ünï cödé", "1"), T("x²", "2"), T("x٣", "3"), T("xⅣ", "4"), T("Ⓐ", "5"),
        T("naïve", "6"), T("ＡＢ", "7"), T("é", "8"), T("é", "9"), T("ß", "10"),
        F("=Ünï cödé"), F("=x²"), F("=\"x²\""), F("=x٣"), F("=xⅣ"), F("=Ⓐ"), F("=naïve*2"),
        F("=ＡＢ"), F("=é"), F("=é"), F("=ß"), F("=SUM(x²)"), F("=SUM(é)+SUM(ß)")])
    yield "names_roots", root([[T("A", "1"), F("=A")], [F("=SUM(A)"), F("=B")]])
    yield "one_value", column([
        T("N1", "5"), T("N2", "5", "6"), T("T1", "a"), T("T2", "a", "b"), T("NT", "5", "a"),
        T("NTT", "5", "a", "b"), T("E", "", ""), G(D("Grid"), [["5", ""], ["", ""]]),
        F("=N1"), F("=N2"), F("=T1"), F("=T2"), F("=NT"), F("=NTT"), F("=E"), F("=Grid"),
        F("=N1&T1"), F("=T1&N1"), F("=NT&'!'"), F("=NT+1")])
    yield "number_texts", column([
        T("A", "007"), T("B", " 4 "), T("C", "0x10"), T("D", "+6"), T("E", "5."), T("G", ".5"),
        T("H", "1e3"), T("I", "-0"), T("J", "1e-320"),
        F("=A"), F("=A&''"), F("=A+0"), F("=B&'|'"), F("=B*2"), F("=C"), F("=C&''"), F("=C+1"),
        F("=D&''"), F("=E&''"), F("=G&''"), F("=H&''"), F("=H+0&''"), F("=I"), F("=I&''"),
        F("=J"), F("=J&''"), F("=J*1e300"), F("=LEN(A)"), F("=LEFT(C, 2)")])


def formula_cells_with_grids():
    """Formula cells that have a grid: their result is their name. TreeSheets goes by the
    result it showed last, so these are recorded but looked at one by one (see README)."""
    yield "fgrid_named_by_result", column([
        G(F("='N'"), [["1"], ["2"]]), F("=SUM(N)"), F("=N")])
    yield "fgrid_named_by_number", column([G(F("=1+1"), [["5"]]), F("=SUM(\"2\")")])
    yield "fgrid_after", column([F("=SUM(N)"), G(F("='N'"), [["1"], ["2"]])])
    yield "fgrid_own_grid", column([G(F("=SUM(A)"), [["1"], ["2"]]), T("A", "3", "4")])


# ---- formulas, at random --------------------------------------------------------------------

NAMES = ["A", "B", "Costs", "Unit price", "Größe", "x_1", "Sales, 2025", "T)", "日本", "a b",
         "Total", "A", "B", "Costs"]
NUMBERS = ["1", "2", "3", "10", "2.5", "-3", "0", "007", "1e3", " 4 ", "0x10", ".5", "5.", "+6",
           "0.1", "0.2", "-0", "1e15", "123456.789", "-1e-11", "3.14159", "1e300", "1e-5"]
TEXTS = ["", "", "abc", "x", "1,000", "12abc", "  ", "TRUE", "#DIV/0!", "é", "1e400", "0e5",
         "inf", "=1", "'q'"]
AGGREGATES = ["SUM", "MIN", "MAX", "AVG", "AVERAGE", "MEDIAN", "COUNT"]


def spell(r, word):
    return r.choice([word, word, word.lower(), word.capitalize()])


def name_ref(r, names, closer=")"):
    name = r.choice(names) if r.random() < 0.9 else r.choice(["Missing", "nope", ""])
    bare = name and all(c.isalnum() or c in "_ " for c in name) and not name[0].isdigit() \
        and name.strip() == name and "  " not in name
    if closer == "" and bare and r.random() < 0.8:
        return name
    if closer != "" and closer not in name and r.random() < 0.5:
        return r.choice(["", " "]) + name + r.choice(["", " "])
    return '"' + name + '"'


def span(r):
    a, b = r.randint(0, 3), r.randint(0, 3)
    return r.choice([str(a), str(a), "-", f"{a}-{b}", f"{a}-", f"-{b}", f" {a} - {b} "])


def subset(r, names, depth=0):
    if r.random() < 0.3:
        inner = subset(r, names, depth + 1) if depth < 2 and r.random() < 0.3 \
            else name_ref(r, names, ",")
        return f"{spell(r, 'TREESUBSET')}({inner},{span(r)},{span(r)},{span(r)})"
    inner = subset(r, names, depth + 1) if depth < 2 and r.random() < 0.2 \
        else name_ref(r, names, ",")
    return f"{spell(r, 'SUBSET')}({inner}, {span(r)}, {span(r)})"


def text_expr(r, names, depth):
    k = r.randint(0, 9)
    if k == 0:
        return "'" + r.choice(["abc", "", " x ", "it''s", "12", "é😀", "a.b.c", "AbC"]) + "'"
    if k == 1:
        return f"{spell(r, 'UPPER')}({expr(r, names, depth + 1)})"
    if k == 2:
        return f"{spell(r, 'LOWER')}({expr(r, names, depth + 1)})"
    if k == 3:
        return f"{spell(r, 'TRIM')}({expr(r, names, depth + 1)})"
    if k == 4:
        return f"LEFT({expr(r, names, depth + 1)}, {r.choice(['0', '1', '2', '9', '-1', '1.5'])})"
    if k == 5:
        return f"RIGHT({expr(r, names, depth + 1)}, {r.choice(['0', '1', '2', '9', '-1'])})"
    if k == 6:
        return f"MID({expr(r, names, depth + 1)}, {r.choice(['0', '1', '2', '5'])}, " \
               f"{r.choice(['0', '1', '3', '-1'])})"
    if k == 7:
        return f"SUBSTITUTE({expr(r, names, depth + 1)}, '{r.choice(['a', '1', '.', ''])}', " \
               f"'{r.choice(['', 'X', '0'])}')"
    if k == 8:
        return f"LEN({expr(r, names, depth + 1)})"
    return f"{expr(r, names, depth + 1)} & {expr(r, names, depth + 1)}"


def term(r, names, depth):
    k = r.randint(0, 19)
    if k <= 3:
        return r.choice(["1", "2", "10", "0", "2.5", "1e3", ".5", "100", "0.1", "3"])
    if k <= 6:
        return name_ref(r, names, "")
    if k <= 10:
        tree = spell(r, "TREE") if r.random() < 0.25 else ""
        arg = subset(r, names) if r.random() < 0.25 else name_ref(r, names)
        return f"{tree}{spell(r, r.choice(AGGREGATES))}({arg})"
    if k == 11:
        return subset(r, names)
    if k <= 13 and depth < 3:
        return "(" + expr(r, names, depth + 1) + ")"
    if k == 14:
        return "-" + term(r, names, depth + 1)
    if k <= 16 and depth < 3:
        return text_expr(r, names, depth)
    if k == 17:
        # Something wrong.
        return r.choice(["", "(", ")", "1 2", "SUM(", "FOO(A)", "'abc", "\"A", "1..2", ",", "#",
                         "SUM(A,B)", "SUBSET(A,0)", "LEN()", "1e", "A B C D"])
    return r.choice(["1", "2", "5"])


def expr(r, names, depth=0):
    parts = [term(r, names, depth)]
    for _ in range(r.choice([0, 0, 1, 1, 2, 3]) if depth < 2 else r.choice([0, 0, 1])):
        op = r.choice(["+", "-", "*", "/", "+", "*", "&"])
        parts.append(r.choice([op, f" {op} ", f"{op} "]))
        parts.append(term(r, names, depth))
    return "".join(parts)


def formula(r, names):
    return r.choice(["=", "=", "= "]) + expr(r, names)


def content(r, names, depth, formulas):
    k = r.random()
    if k < 0.5:
        return D(r.choice(NUMBERS))
    if k < 0.65:
        return D(r.choice(TEXTS))
    if k < 0.65 + formulas:
        return F(formula(r, names))
    if depth < 3 and k < 0.9:
        return table(r, names, depth + 1)
    return D(r.choice(NUMBERS))


def table(r, names, depth):
    xs, ys = r.choice([1, 1, 1, 2, 3]), r.choice([1, 2, 3, 4])
    formulas = r.choice([0, 0, 0.1, 0.25])
    rows = [[content(r, names, depth, formulas) for _ in range(xs)] for _ in range(ys)]
    kind = 0 if r.random() < 0.95 else r.choice([2, 4, 3, 5])
    return G(K(r.choice(names), kind), rows)


def random_formula_document(r):
    names = r.sample(NAMES, r.randint(2, 6))
    xs, ys = r.choice([1, 2, 3]), r.randint(3, 7)
    rows = []
    for _ in range(ys):
        row = []
        for _ in range(xs):
            k = r.random()
            if k < 0.4:
                row.append(table(r, names, 1))
            elif k < 0.85:
                row.append(F(formula(r, names)))
            else:
                row.append(D(r.choice(NUMBERS + TEXTS)))
        rows.append(row)
    return root(rows)


def random_formula_documents(count):
    r = random.Random(SEED)
    for n in range(count):
        yield f"random_{n:03}", random_formula_document(r)


# ---- programs -------------------------------------------------------------------------------

def O(op):
    return K(op, 1)


VH, VV = K("", 3), K("", 5)


def view(vert=False, text=""):
    return K(text, 5 if vert else 3)


def assign(name):
    return K(name, 2)


def read(name):
    return K(name, 4)


def bad(op):
    """An operation cell whose text is no operation."""
    return K(op, 1)


def data_grid(rows):
    return G(D(""), rows)


def program_documents():
    V = view()
    binary = ["+", "-", "*", "/", "<", ">", "<=", ">=", "=", "==", "!=", "<>"]
    pairs = [("7", "2"), ("2", "7"), ("3", "3"), ("1.5", "0"), ("0", "0"), ("-4", "2.5"),
             ("abc", "2"), ("12abc", "1"), ("1e3", "1e-3"), ("1e308", "1e308"), ("0.1", "0.2"),
             (" 5", "5 "), ("1e999", "1"), ("-1e999", "1"), (".", "1"), ("1e", "1"), ("0x10", "1"),
             ("1,5", "1"), ("  ", "1"), ("1", "  "), ("inf", "1"), ("-0", "0"), ("1e-400", "1")]
    for op in binary:
        yield f"op_{OPERATIONS.index(op):02}", root([[a, O(op), b, V] for a, b in pairs])
    numbers = ["5", "-2.5", "0", "abc", "12abc", "1e308", "  ", "1e999", "0.1", "-0", "1500",
               "999.5", "1000.5", "-3", "3.7"]
    for op in ["inc", "dec", "neg", "graph"]:
        yield f"unary_{op}", root([[n, O(op), V, O(op), V] for n in numbers])
    lists = [
        data_grid([["1", "2", "3"]]), data_grid([["1"], ["2"], ["3"]]),
        data_grid([["1", "2"], ["3", "4"], ["5", "6"]]), data_grid([["4", "1", "3", "2"]]),
        data_grid([["", "abc", "12abc", "5"]]), data_grid([["1.5", " 2 ", "0x10", "1e2"]]),
        data_grid([[""]]), data_grid([["", ""]]), data_grid([["  ", "1"]]),
        data_grid([["-1", "-5", "-0.5"]]), data_grid([["0", "-0"]]), data_grid([["-0", "0"]]),
        data_grid([["1", data_grid([["100"]]), "2"]]), data_grid([["x", "y"], ["z", "w"]]),
        data_grid([["1e999", "1"]]), G(D("named"), [["1", "2"]]), D("5"), D(""),
    ]
    for op in ["sum", "min", "max", "avg", "median", "transpose", "graph", "inc", "neg"]:
        yield f"list_{op}", root([[g, O(op), V] for g in lists])
        yield f"list_{op}_down", root([lists, [O(op)] * len(lists), [view(True)] * len(lists)])
    grids = [data_grid([["1", "2"], ["3", "4"]]), data_grid([["10", "20"], ["30", "40"]]),
             data_grid([["1", "2", "3"]]), data_grid([["1", ""], ["", "4"]]),
             data_grid([["1", data_grid([["5", "6"]])]]), data_grid([["2", data_grid([["7", "8"]])]])]
    rows = []
    for a in grids:
        for b in grids + [D("10"), D("")]:
            rows.append([a, O("+"), b, V])
    yield "grid_plus", root(rows)
    yield "grid_times_scalar", root([[g, O("*"), "3", V, O("<"), "10", V] for g in grids]
                                    + [["3", O("*"), g, V, O("neg"), V] for g in grids])
    yield "if", root([
        [c, O("if"), a, b, V] for c in ["1", "0", "", "abc", "-1", "0.0", "  ", "2"]
        for a, b in [("yes", "no"), (data_grid([["1", "2"]]), "no"), ("yes", K("x", 1))]])
    yield "if_edges", root([
        ["1", O("if"), "yes", V, D("")], ["1", O("if"), V, D(""), D("")],
        [O("if"), "a", "b", V, D("")], ["0", O("if"), "a", read("nope"), V],
        ["1", O("if"), read("nope"), "b", V], ["1", O("if"), D("a"), D("b"), D("c")]])
    yield "chains", root([
        ["1", O("+"), "2", O("*"), "3", V, O("-"), "1", V],
        ["10", O("/"), "4", O("inc"), O("neg"), V, O("graph"), V, V],
        ["5", V, O("inc"), V, O("inc"), V, D(""), D(""), D("")],
        ["2", O("*"), "3", V, "4", O("*"), "5", V, D("")],
        [V, "1", V, V, "2", O("+"), "3", V, V]])
    yield "failures_unknown", root([["1", bad("foo"), V], ["2", O("inc"), V]])
    yield "failures_no_input", root([[O("inc"), "1", V], [O("+"), "1", V]])
    yield "failures_no_argument", root([["1", O("+")], ["2", D("")]])
    yield "failures_no_argument_2", root([["1", O("if"), "2"], ["3", V, D("")]])
    yield "failures_variable", root([[read("nope"), V, D("")], ["1", O("+"), read("nope")]])
    yield "failures_order", root([["1", bad("foo"), V], [read("nope"), V, D("")]])
    yield "variables", root([
        ["5", assign("x"), V, D(""), D("")],
        [read("x"), O("+"), "1", assign("y"), V],
        [read("x"), O("*"), read("y"), V, D("")],
        [read("y"), V, read("z"), V, D("")],
        ["7", assign(""), V, D(""), D("")],
        ["8", assign("x"), read("x"), V, D("")],
        [data_grid([["1", "2"]]), assign("g"), V, D(""), D("")],
        [read("g"), O("sum"), V, D(""), D("")]])
    yield "variables_destructure", root([
        [data_grid([["1", "2"], ["3", "4"]]), G(assign("m"), [["a", "b"], ["c", "d"]]), V],
        [read("a"), O("+"), read("d")], [read("m"), O("sum"), V],
        [data_grid([["1", "2", "3"]]), G(assign("n"), [["p", "q"]]), read("p")],
        [read("n"), V, D("")]])
    yield "variables_from_own_grid", root([
        [D(""), G(assign("v"), [["1", "2"]]), V], [read("v"), O("sum"), V],
        ["9", G(assign("w"), [["1", "2"]]), V], [read("w"), V, D("")]])
    yield "variables_later_rows", root([
        [read("late"), V, D("")], ["3", assign("late"), V], [read("late"), V, D("")]])
    yield "views", root([
        ["1", V, view(True), V], ["2", view(text="old"), view(True, "old"), D("")],
        [V, V, D("x"), V], [data_grid([["a", "b"]]), V, D(""), D("")],
        [G(D("named"), [["c"]]), G(view(), [["old", "grid"]]), D(""), D("")]])
    yield "views_down", root([
        ["1", "2", data_grid([["a", "b"]]), V], [O("inc"), view(True), view(True), "x"],
        [view(True), O("inc"), "y", view(True)], ["3", view(True), view(True), view(True)]])
    yield "nested_programs", root([
        [G(D("p"), [["1", O("+"), "2", V]]), V, D("")],
        [G(D("q"), [["2", O("*"), "3"]]), O("inc"), V],
        [G(D(""), [["4"], [O("inc")], [view(True)]]), V, D("")],
        [G(D("r"), [[G(D(""), [["5", O("inc"), V]]), O("inc"), V]]), V, D("")]])
    yield "nested_data", root([
        [G(D("a"), [["1", G(D("b"), [["2", G(D("c"), [["3"]])]])]]), V],
        [G(D("d"), [["1", "2"], ["3", "4"]]), V]])
    yield "nested_views_in_data", root([
        [G(D("a"), [["1", "2"], ["3", G(D("b"), [["4", O("inc"), V]])]]), V],
        [G(D("c"), [["5", V], ["6", V]]), V]])
    yield "formulas_in_programs", root([
        [T("N", "1", "2"), F("=SUM(N)"), O("inc"), V],
        [F("=SUM(N)*2"), V, D(""), D("")],
        [G(D("box"), [[F("=SUM(N)"), "5"]]), V, D(""), D("")],
        [F("=Missing"), O("inc"), V, D("")],
        [F("='text'"), V, O("graph"), V],
        [G(D("box2"), [[F("=SUM(N)"), O("inc"), V]]), V, D(""), D("")]])
    yield "operation_names", root([
        ["3", K(n, 1), V] for n in ["INC", "Inc", " inc", "inc ", "plus", "", "sum", "++"]])
    yield "transpose", root([
        [data_grid([["1", "2", "3"], ["4", "5", "6"]]), O("transpose"), V, O("transpose"), V],
        [data_grid([["1"], ["2"]]), O("transpose"), O("sum"), V, D("")],
        ["5", O("transpose"), V, D(""), D("")]])
    yield "single_cells", root([["1"]])
    yield "single_row", root([["1", O("inc"), V]])
    yield "single_column", root([["1"], [O("inc")], [view(True)]])
    yield "square", root([["1", O("inc")], [O("neg"), V]])
    yield "both_ways", root([
        ["1", "2", "3"], [O("+"), O("*"), O("-")], ["10", "20", "30"],
        [view(True), view(True), view(True)], [V, D(""), D("")]])


def program_cell(r, depth):
    return D(r.choice(["1", "2", "3", "10", "2.5", "-3", "0", "abc", "", "12abc", "1e3", "0.5"]))


def program_row(r, width, depth):
    """A row that is a small program, as wide as `width`. A variable is assigned only right
    after a plain data cell or a variable read: where nothing is in hand (at the start of a
    row, after an operation that failed or a grid that gave nothing) TreeSheets reads
    through an empty value, which crashes it."""
    V = view()
    value = lambda: (program_cell(r, depth) if depth >= 2 or r.random() < 0.8  # noqa: E731
                     else program_grid(r, depth + 1))
    row = []
    holds = False  # whether a value is surely in hand
    while len(row) < width:
        k = r.randint(0, 11)
        if not holds or k <= 1:
            row.append(value())
            holds = True
        elif k <= 4:
            row += [O(r.choice(["+", "-", "*", "/", "<", ">", "<=", ">=", "=", "==", "!=", "<>"])),
                    value()]
        elif k <= 6:
            row.append(O(r.choice(["inc", "dec", "neg", "graph", "sum", "min", "max", "avg",
                                   "median", "transpose"])))
        elif k == 7 and len(row[-1]) == 2 and row[-1][1] in (0, 4):
            row.append(assign(r.choice(["x", "y", "z"])))
        elif k == 8:
            row.append(read(r.choice(["x", "y", "z", "nope"])))
        elif k == 9:
            row += [O("if"), value(), value()]
        else:
            row.append(V)
    row = row[:width]
    # What was cut off may have been an argument: then the operation fails, which is a case too.
    return row + [D("")] * (width - len(row))


def program_grid(r, depth):
    kind = r.random()
    if kind < 0.35:
        xs, ys = r.randint(1, 3), r.randint(1, 3)
        return data_grid([[program_cell(r, depth) for _ in range(xs)] for _ in range(ys)])
    if kind < 0.85:
        width = r.randint(2, 6)
        return G(D(r.choice(["", "", "p"])), [program_row(r, width, depth)
                                              for _ in range(r.randint(1, 3))])
    # A column program: no assignments (they are skipped downwards), views that go down.
    height = r.randint(2, 5)
    columns = []
    for _ in range(r.randint(1, 2)):
        col = [program_cell(r, depth)]
        while len(col) < height:
            k = r.randint(0, 5)
            if k <= 1:
                col.append(O(r.choice(["inc", "dec", "neg", "sum", "graph"])))
            elif k == 2 and len(col) + 1 < height:
                col += [O(r.choice(["+", "*", "-", "<"])), program_cell(r, depth)]
            elif k == 3:
                col.append(program_cell(r, depth))
            else:
                col.append(view(True))
        columns.append(col[:height])
    return G(D(""), [[c[y] for c in columns] for y in range(height)])


def random_program_documents(count):
    r = random.Random(SEED + 1)
    for n in range(count):
        width = r.randint(3, 7)
        yield f"random_{n:03}", root([program_row(r, width, 0) for _ in range(r.randint(2, 5))])


# ---- sequences ------------------------------------------------------------------------------
# Steps (columns and rows from 0, a path is a list of [column, row] from the root):
#   ["text", path, text]            ts.set_text: a formula cell becomes a data cell
#   ["formula", path, formula]      ts.set_formula
#   ["kind", path, kind]            ts.set_cell_type
#   ["insert-row", path, row]       in the grid of the cell at path
#   ["insert-column", path, column]
#   ["delete", path, column, row, columns, rows]
#   ["sort", path, column, row, rows, descending]   by pressing the key of the menu item
#   ["run"]                         Program > Run, by pressing its key (it keeps the variables)
#   ["undo"], ["redo"]

def sequence_start():
    return root([
        [T("Costs", "10", "20", "30"), F("=SUM(Costs)"), F("=SUM(Costs)*2")],
        [T("Prices", "1.5", "2.5"), F("=MAX(Prices)"), F("=SUM(Costs)+SUM(Prices)")],
        [G(D("Items"), [["pear", "3"], ["apple", "1"], ["fig", F("=SUM(Prices)")],
                        ["Banana", "2"]]),
         F("=SUM(SUBSET(Items, 1, -))"), F("=SUBSET(Items, 0, 0)")],
        [G(D("Box"), [[T("Costs", "1", "2"), F("=SUM(Costs)")]]), F("=TREESUM(Box)"),
         F("=Missing")],
        ["5", assign("rate"), F("=rate*2")],
        [read("rate"), O("inc"), view()],
    ])


def sequences():
    yield "edit_a_number", [
        ["text", [[0, 0], [0, 1]], "25"], ["text", [[0, 0], [0, 0]], "abc"], ["undo"], ["undo"],
        ["redo"], ["text", [[0, 1], [0, 0]], "100"], ["text", [[0, 3], [0, 0], [0, 0]], "50"]]
    yield "rename_a_table", [
        ["text", [[0, 0]], "Kosten"], ["undo"], ["redo"], ["text", [[0, 3], [0, 0]], "Kosten"],
        ["text", [[0, 0]], "Costs"], ["text", [[0, 1]], "Costs"]]
    yield "rows_and_columns", [
        ["insert-row", [[0, 0]], 1], ["text", [[0, 0], [0, 1]], "5"], ["insert-row", [], 0],
        ["insert-column", [], 1], ["delete", [[0, 1]], 0, 0, 1, 1], ["undo"], ["redo"],
        ["delete", [], 0, 0, 1, 1], ["insert-column", [[0, 2]], 0], ["text", [[0, 2], [0, 1]], "7"],
        ["delete", [[0, 2]], 1, 0, 1, 1]]
    yield "delete_a_table", [
        ["delete", [], 0, 0, 1, 1], ["undo"], ["delete", [[0, 3]], 0, 0, 1, 1],
        ["delete", [], 0, 1, 1, 1], ["text", [[0, 2]], "Prices"]]
    yield "formulas_come_and_go", [
        ["formula", [[0, 0], [0, 0]], "=SUM(Prices)"], ["formula", [[1, 0]], "=MIN(Costs)"],
        ["text", [[1, 0]], "plain"], ["undo"], ["formula", [[0, 1], [0, 0]], "=SUM(Costs)"],
        ["formula", [[0, 0], [0, 1]], "=SUM(Prices)"], ["text", [[0, 0], [0, 1]], "1"],
        ["formula", [[2, 3]], "SUM(Costs)"], ["formula", [[2, 3]], "='x' & Missing"],
        ["kind", [[1, 1]], 0], ["kind", [[0, 0], [0, 2]], 6], ["kind", [[0, 0], [0, 2]], 0]]
    yield "sort", [
        ["sort", [[0, 2]], 0, 0, 4, False], ["sort", [[0, 2]], 1, 0, 4, False],
        ["sort", [[0, 2]], 1, 0, 4, True], ["text", [[0, 1], [0, 1]], "0.5"],
        ["sort", [[0, 2]], 1, 0, 4, False], ["sort", [[0, 2]], 0, 1, 3, True],
        ["sort", [[0, 0]], 0, 0, 3, True], ["sort", [], 1, 0, 4, False]]
    yield "run", [
        ["run"], ["text", [[0, 4]], "8"], ["run"], ["text", [[1, 4]], "other"], ["run"],
        ["formula", [[2, 3]], "=other+rate"], ["text", [[1, 5]], "neg"], ["run"],
        ["kind", [[1, 4]], 0], ["run"], ["delete", [], 0, 4, 3, 1], ["run"]]
    yield "run_and_undo", [
        ["run"], ["text", [[0, 4]], "9"], ["undo"], ["text", [[0, 4]], "6"], ["run"]]


def random_sequences(count):
    r = random.Random(SEED + 2)
    cells = [[[0, 0], [0, 0]], [[0, 0], [0, 1]], [[0, 0], [0, 2]], [[0, 1], [0, 0]],
             [[0, 1], [0, 1]], [[0, 2], [1, 0]], [[0, 2], [1, 1]], [[0, 3], [0, 0], [0, 0]],
             [[0, 4]], [[0, 0]], [[0, 1]], [[1, 0]], [[1, 1]], [[2, 2]]]
    for n in range(count):
        steps, fresh, changed = [], 100 + n, False
        for _ in range(r.randint(6, 10)):
            k = r.randint(0, 11)
            if k <= 3:
                fresh += 1
                steps.append(["text", r.choice(cells), r.choice([
                    str(fresh), str(fresh) + ".5", f"t{fresh}", "Costs", "Prices", "-" + str(fresh)])])
                changed = True
            elif k <= 5:
                fresh += 1
                steps.append(["formula", r.choice(cells), formula(r, ["Costs", "Prices", "Items",
                                                                    "Box", "rate"]) + f"+{fresh}"])
                changed = True
            elif k == 6:
                steps.append(["insert-row", r.choice([[], [[0, 0]], [[0, 2]]]), r.randint(0, 2)])
                changed = True
            elif k == 7:
                steps.append(["insert-column", r.choice([[], [[0, 0]], [[0, 2]]]), r.randint(0, 1)])
                changed = True
            elif k == 8 and changed:
                steps.append(["undo"])
                if r.random() < 0.5:
                    steps.append(["redo"])
                changed = False
            elif k == 9:
                steps.append(["sort", [[0, 2]], r.randint(0, 1), 0, 4, r.random() < 0.5])
                changed = False
            elif k == 10:
                steps.append(["run"])
                changed = False
            else:
                steps.append(["delete", r.choice([[[0, 0]], [[0, 1]]]), 0, 0, 1, 1])
                changed = False
        yield f"random_{n:02}", steps
