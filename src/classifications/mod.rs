pub mod types;
pub mod expected_points;
pub mod accuracy;
pub mod attackers;
pub mod defenders;
pub mod piece_safety;
pub mod piece_trapped;
pub mod danger_levels;
pub mod point_loss;
pub mod brilliant;
pub mod great;

pub use types::{
    BoardPiece, ClassificationContext, ClassificationKind, Evaluation, MoveClassification,
};
pub use accuracy::{get_move_accuracy, PlayerAccuracy};
pub use expected_points::{get_expected_points, get_expected_points_loss};

use crate::board::Board;

/// Classifies a move given the full context.
///
/// Checks are applied in priority order — the first matching classification
/// wins. The order matters: Brilliant and Great require additional board-level
/// checks and are tested before the point-loss path.
///
/// Call this after computing `ctx.point_loss` via
/// [`get_expected_points_loss`].
pub fn classify(
    board_before: &Board,
    board_after: &Board,
    ctx: &ClassificationContext,
) -> ClassificationKind {
    if ctx.is_book {
        return ClassificationKind::Book;
    }

    if ctx.is_forced {
        return ClassificationKind::Forced;
    }

    if brilliant::is_brilliant(board_before, board_after, ctx) {
        return ClassificationKind::Brilliant;
    }

    if great::is_great(board_before, ctx) {
        return ClassificationKind::Great;
    }

    // Point-loss based classifications ===
    // This handles: Best, Excellent, Good, Okay, Inaccuracy, Miss, Mistake, Blunder.
    let mut kind = point_loss::point_loss_classify(ctx);

    // Good / Miss distinction ===
    // "Good" sits between Excellent and Okay. "Miss" means the player had a
    // winning tactic they overlooked (was winning before, not after).
    // We reclassify Okay→Good when loss is in the 0.045–0.08 range but the
    // position is still clearly fine, and Best→Miss when a forced win was missed.
    kind = refine_good_and_miss(kind, ctx);

    // Risky ====
    // A move that is objectively sound (point_loss < inaccuracy threshold) but
    // leaves our own pieces in danger without compensation.
    if is_risky(&kind, board_after, ctx) {
        return ClassificationKind::Risky;
    }

    kind
}

/// Refines the raw point-loss classification to separate Good from Okay and
/// to detect a Miss (overlooked forced win).
fn refine_good_and_miss(
    kind: ClassificationKind,
    ctx: &ClassificationContext,
) -> ClassificationKind {
    match kind {
        // Okay in the 0.045–0.08 range → Good
        ClassificationKind::Okay if ctx.point_loss < 0.08 && ctx.point_loss >= 0.045 => {
            ClassificationKind::Good
        }
        // Best move played but a previous forced mate was missed
        ClassificationKind::Best => {
            if let (Some(Evaluation::Mate(prev_m)), Evaluation::Centipawn(_)) =
                (ctx.second_best_eval, ctx.eval_after)
                && *prev_m > 0 && ctx.point_loss > 0.01 {
                    return ClassificationKind::Miss;
                }
            ClassificationKind::Best
        }
        other => other,
    }
}

/// Returns `true` if the move should be flagged as Risky.
///
/// A move is risky when it is objectively fine (below inaccuracy threshold)
/// but leaves the mover's own pieces unsafe on the resulting board.
fn is_risky(
    kind: &ClassificationKind,
    board_after: &Board,
    ctx: &ClassificationContext,
) -> bool {
    // Only flag moves that are otherwise classified as Good or better.
    let is_sound = matches!(
        kind,
        ClassificationKind::Best
            | ClassificationKind::Excellent
            | ClassificationKind::Good
            | ClassificationKind::Okay
    );
    if !is_sound {
        return false;
    }

    // If we left unsafe pieces on the board, the move is risky.
    let unsafe_pieces = piece_safety::get_unsafe_pieces(board_after, ctx.color, None);
    !unsafe_pieces.is_empty()
}
