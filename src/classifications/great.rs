use crate::board::Board;
use super::brilliant::is_critical_candidate;
use super::expected_points::get_expected_points_loss;
use super::piece_safety::is_piece_safe;
use super::types::{BoardPiece, ClassificationContext};
use crate::Piece;

/// The minimum second-best-move point loss required for a move to be "Great".
/// A value of 0.10 corresponds to a loss midway between inaccuracy and mistake.
const GREAT_SECOND_MOVE_THRESHOLD: f64 = 0.10;

/// Returns `true` if the move deserves a Great classification.
///
/// A Great move is critical to maintaining an advantage — it is not easy to
/// find, not forced, and the second-best alternative is significantly worse.
pub fn is_great(
    board_before: &Board,
    ctx: &ClassificationContext,
) -> bool {
    if !is_critical_candidate(ctx) {
        return false;
    }

    // A great move cannot be a capture of genuinely free material — that would
    // be too easy to find.
    if let Some(captured_type) = board_before.piece_at(ctx.played_move.to).map(|p| p.piece_type) {
    let captured_piece = BoardPiece::new(
        Piece::new(captured_type, ctx.color),
        ctx.played_move.to,
    );
    if !is_piece_safe(board_before, &captured_piece) {
        return false;
    }
}

    // We need the second-best evaluation to gauge how hard the move was to find.
    let second_eval = match ctx.second_best_eval {
        Some(e) => e,
        None    => return false,
    };

    // If the second-best alternative is already winning (small point loss),
    // the played move is not especially difficult to find.
    let second_loss = get_expected_points_loss(ctx.eval_before, second_eval, ctx.color);
    second_loss >= GREAT_SECOND_MOVE_THRESHOLD
}
