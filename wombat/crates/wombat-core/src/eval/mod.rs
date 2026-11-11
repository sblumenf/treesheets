//! Formulas and the program of a document: TreeSheets' two ways to calculate (decision
//! A14), ported from `src/evaluator.h` and checked against the real app (`referee/formulas`).
//!
//! - **Formula cells** (`=SUM(Costs) - 10`): [`Calc`] holds what each one shows. The text
//!   of the cell stays its formula; the result is never saved.
//! - **The program** (Program > Run): cells marked as operation, variable or view are
//!   worked through grid by grid, and the views are filled in. `run` works that out on a
//!   copy; the command [`crate::Command::Run`] puts it into the document.
//!
//! Nothing here changes a document, reads a clock or depends on the machine's locale.

mod calc;
mod formula;
mod num;
mod run;

pub use calc::{Calc, RunReport, Stats, MAX_WORK};
pub use formula::{formula_of, is_formula_text, MAX_FORMULA_LEN, MAX_NESTING};
pub use num::{is_name_char, is_space, lower, num_text, stream_num, strict_num, upper};
pub use run::{
    is_operation, Failure, FailureKind, Value, ValueGrid, Variables, MAX_RUN_WORK, MAX_VALUE_DEPTH,
};

pub(crate) use run::run;
