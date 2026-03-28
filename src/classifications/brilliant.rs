use crate::board::Board;
use crate::types::PieceType;
use super::attackers::get_direct_attackers;
use super::danger_levels::{has_danger_levels, DangerEqualityStrategy};
use super::piece_safety::get_unsafe_pieces;
use super::piece_trapped::is_piece_trapped;
use super::types::ClassificationContext;

/// Returns `true` if the move deserves a Brilliant classification.
///
/// A brilliant move must:
/// 1. Pass the critical-move candidate check (see `is_critical_candidate`).
/// 2. Not be a promotion.
/// 3. Not simply move a piece to safety (fewer unsafe pieces after than before).
/// 4. Leave the opponent without adequate counterthreats against every remaining
///    unsafe piece.
/// 5. Not involve a piece that was already trapped (no bravery required).
/// 6. Leave at least one piece of ours genuinely at risk — i.e., the sacrifice
///    is real, not illusory.
pub fn is_brilliant(
    board_before: &Board,
    board_after: &Board,
    ctx: &ClassificationContext,
) -> bool {
    if !is_critical_candidate(ctx) {
        return false;
    }

    // Promotions cannot be brilliant.
    if ctx.played_move.promotion_piece().is_some() {
        return false;
    }

    let mover_color = ctx.color;
    let opponent_color = mover_color.opposite();

    let prev_unsafe = get_unsafe_pieces(board_before, mover_color, None);
    let curr_unsafe = get_unsafe_pieces(
        board_after,
        mover_color,
        board_before.piece_at(ctx.played_move.to).map(|p| p.piece_type.value()),
    );

    // Moving a piece to safety disallows brilliant.
    if !board_after.king_square(mover_color)
        .map(|_sq| crate::movegen::is_in_check(board_after, mover_color))
        .unwrap_or(false)
        && curr_unsafe.len() < prev_unsafe.len()
    {
        return false;
    }

    // All remaining unsafe pieces must have counterthreats (danger levels).
    let all_protected = curr_unsafe.iter().all(|unsafe_piece| {
        let attacking_moves: Vec<_> = get_direct_attackers(board_after, unsafe_piece.square, opponent_color)
            .iter()
            .map(|bp| crate::types::Move::new(bp.square, unsafe_piece.square))
            .collect();
        has_danger_levels(board_after, unsafe_piece, &attacking_moves, DangerEqualityStrategy::Leaves)
    });

    if all_protected {
        return false;
    }

    // Disallow if the moved piece was already trapped before the move.
    let prev_trapped: Vec<_> = prev_unsafe
        .iter()
        .filter(|up| is_piece_trapped(board_before, up, false))
        .collect();

    let moved_piece_was_trapped = prev_trapped
        .iter()
        .any(|tp| tp.square == ctx.played_move.from);

    let curr_trapped: Vec<_> = curr_unsafe
        .iter()
        .filter(|up| is_piece_trapped(board_after, up, false))
        .collect();

    if curr_trapped.len() == curr_unsafe.len()
        || moved_piece_was_trapped
        || curr_trapped.len() < prev_trapped.len()
    {
        return false;
    }

    !curr_unsafe.is_empty()
}

/// Preliminary check shared by Brilliant and Great classifications.
///
/// A move cannot be critical if:
/// - The position is already completely winning even without this move.
/// - The player is in a losing position (negative subjective eval).
/// - It is a queen promotion.
/// - The player was in check (forced response, no ingenuity required).
pub fn is_critical_candidate(ctx: &ClassificationContext) -> bool {
    use super::types::Evaluation;

    // Already winning even with the second-best move → not critical.
    if let Some(second_eval) = ctx.second_best_eval {
        let second_subj = second_eval.subjective(ctx.color);
        if let Evaluation::Centipawn(v) = second_subj
            && v >= 700 {
                return false;
            }
    } else {
        let curr_subj = ctx.eval_after.subjective(ctx.color);
        if let Evaluation::Centipawn(v) = curr_subj
            && v >= 700 {
                return false;
            }
    }

    // Cannot be critical from a losing position.
    let subj = ctx.eval_after.subjective(ctx.color);
    if subj.value() < 0 {
        return false;
    }

    // Queen promotions are never critical (too obvious).
    if ctx.played_move.promotion_piece() == Some(PieceType::Queen) {
        return false;
    }

    // Moves that escape check are forced — no critical thinking required.
    if ctx.in_check_before {
        return false;
    }

    true
}
