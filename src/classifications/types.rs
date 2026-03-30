use crate::types::{Color, Move, Piece, Square};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardPiece {
    pub piece: Piece,
    pub square: Square,
}
impl BoardPiece {
    pub fn new(piece: Piece, square: Square) -> Self {
        Self { piece, square }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Evaluation {
    Centipawn(i32),
    Mate(i32),
}
impl Evaluation {
    pub fn centipawn(&self) -> Option<i32> {
        match self {
            Evaluation::Centipawn(v) => Some(*v),
            _ => None,
        }
    }
    pub fn mate_in(&self) -> Option<i32> {
        match self {
            Evaluation::Mate(v) => Some(*v),
            _ => None,
        }
    }
    pub fn subjective(&self, color: Color) -> Evaluation {
        let flip = color == Color::Black;
        match self {
            Evaluation::Centipawn(v) => Evaluation::Centipawn(if flip { -v } else { *v }),
            Evaluation::Mate(v) => Evaluation::Mate(if flip { -v } else { *v }),
        }
    }
    pub fn value(&self) -> i32 {
        match self {
            Evaluation::Centipawn(v) | Evaluation::Mate(v) => *v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassificationKind {
    Brilliant,
    Great,
    Best,
    Excellent,
    Good,
    Okay,
    Inaccuracy,
    Miss,
    Mistake,
    Blunder,
    Forced,
    Book,
    Risky,
}
impl fmt::Display for ClassificationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ClassificationKind::Brilliant => "Brilliant",
            ClassificationKind::Great => "Great",
            ClassificationKind::Best => "Best",
            ClassificationKind::Excellent => "Excellent",
            ClassificationKind::Good => "Good",
            ClassificationKind::Okay => "Okay",
            ClassificationKind::Inaccuracy => "Inaccuracy",
            ClassificationKind::Miss => "Miss",
            ClassificationKind::Mistake => "Mistake",
            ClassificationKind::Blunder => "Blunder",
            ClassificationKind::Forced => "Forced",
            ClassificationKind::Book => "Book",
            ClassificationKind::Risky => "Risky",
        };
        write!(f, "{s}")
    }
}

/// The full classification result for a single move.
#[derive(Debug, Clone)]
pub struct MoveClassification {
    /// SAN notation of the played move (e.g. `"Nf3+"`, `"O-O"`).
    pub san: String,
    /// UCI notation of the played move (e.g. `"g1f3"`).
    pub played_move: String,
    /// UCI notation of the engine's best move.
    pub best_move: String,
    /// Engine's 2nd-best and beyond, in UCI notation.
    ///
    /// Populated only when [`AnalyzerConfig::with_alternatives`] is set to a
    /// value greater than zero; otherwise this is always empty.
    pub alternatives: Vec<String>,
    /// Classification category.
    pub kind: ClassificationKind,
    /// Side that played this move.
    pub color: Color,
    /// Full-move number (1-based, increments after Black moves).
    pub move_number: u32,
    /// Expected point loss [0.0, 1.0].
    pub point_loss: f64,
    /// Move accuracy percentage [0.0, 100.0].
    pub accuracy: f64,
    /// Engine evaluation before the move (White's perspective).
    pub eval_before: Evaluation,
    /// Engine evaluation after the move (White's perspective).
    pub eval_after: Evaluation,
}

impl MoveClassification {
    /// Returns the move the engine recommends showing to the user, or `None`
    /// when no suggestion is meaningful.
    ///
    /// `None` is returned for [`ClassificationKind::Brilliant`],
    /// [`ClassificationKind::Great`], [`ClassificationKind::Forced`], and
    /// [`ClassificationKind::Book`] — these moves are either exceptional or
    /// the only legal option, so presenting a suggestion would be misleading.
    ///
    /// For every other classification the suggestion is the engine's top-ranked
    /// move, which may be identical to the played move (e.g. for Best/Excellent).
    pub fn suggested_move(&self) -> Option<&str> {
        match self.kind {
            ClassificationKind::Brilliant
            | ClassificationKind::Great
            | ClassificationKind::Forced
            | ClassificationKind::Book => None,
            _ => Some(&self.best_move),
        }
    }
}

impl fmt::Display for MoveClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}. {} {} [{}]  best: {}  loss: {:.3}  accuracy: {:.1}%",
            self.move_number,
            self.san,
            self.played_move,
            self.kind,
            self.best_move,
            self.point_loss,
            self.accuracy,
        )
    }
}

#[derive(Debug)]
pub struct ClassificationContext<'a> {
    pub played_move: &'a Move,
    pub best_move: &'a Move,
    pub eval_before: &'a Evaluation,
    pub eval_after: &'a Evaluation,
    pub second_best_eval: Option<&'a Evaluation>,
    pub point_loss: f64,
    pub is_book: bool,
    pub is_forced: bool,
    pub in_check_before: bool,
    pub color: Color,
}
