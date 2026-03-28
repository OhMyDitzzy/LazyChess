use crate::board::Board;
use crate::movegen::{apply_move, generate_pseudo_legal_moves};
use crate::types::{Move, PieceType};
use super::attackers::{any_move_is_checkmate, get_direct_attackers, with_side_to_move};
use super::piece_safety::get_unsafe_pieces;
use super::types::BoardPiece;

/// Returns all unsafe pieces of the same color as `acting_move` that are of
/// equal or greater value than `threatened_piece`, excluding `threatened_piece`
/// itself.
fn relative_unsafe_piece_attacks(
    board: &Board,
    threatened_piece: &BoardPiece,
    acting_color: crate::types::Color,
    last_captured_value: Option<i32>,
) -> Vec<Vec<super::types::BoardPiece>> {
    get_unsafe_pieces(board, acting_color, last_captured_value)
        .into_iter()
        .filter(|up| {
            up.square != threatened_piece.square
                && up.piece.piece_type.value() >= threatened_piece.piece.piece_type.value()
        })
        .map(|up| get_direct_attackers(board, up.square, acting_color.opposite()))
        .collect()
}

/// Returns `true` if playing `acting_move` creates a counterthreat that is
/// greater than the threat currently imposed on `threatened_piece`.
///
/// "Greater" means new unsafe pieces of equal or higher value are introduced,
/// or a checkmate threat appears after a low-value sacrifice.
pub fn move_creates_greater_threat(
    board: &Board,
    threatened_piece: &BoardPiece,
    acting_move: &Move,
) -> bool {
    let acting_color = threatened_piece.piece.color;
    let acting_board = with_side_to_move(board, acting_color);

    let prev_relative_attacks = relative_unsafe_piece_attacks(
        &acting_board, threatened_piece, acting_color, None,
    );

    // Apply the move.
    let pseudo = generate_pseudo_legal_moves(&acting_board);
    if !pseudo.iter().any(|m| m == acting_move) {
        return false;
    }
    let after_board = apply_move(&acting_board, acting_move);

    // Captured piece value, if any.
    let captured_value = board
        .piece_at(acting_move.to)
        .map(|p| p.piece_type.value());

    let new_relative_attacks = relative_unsafe_piece_attacks(
        &after_board, threatened_piece, acting_color, captured_value,
    );

    // New threats that didn't exist before.
    let has_new_threats = new_relative_attacks.len() > prev_relative_attacks.len();
    if has_new_threats {
        return true;
    }

    // Low-value sacrifice that leads to checkmate.
    let is_low_value = threatened_piece.piece.piece_type.value() < PieceType::Queen.value();
    is_low_value && any_move_is_checkmate(&after_board)
}

/// Returns `true` if playing `acting_move` leaves a counterthreat at least as
/// great as the threat on `threatened_piece` — even if not directly caused by
/// the move itself.
pub fn move_leaves_greater_threat(
    board: &Board,
    threatened_piece: &BoardPiece,
    acting_move: &Move,
) -> bool {
    let acting_color = threatened_piece.piece.color;
    let acting_board = with_side_to_move(board, acting_color);

    let pseudo = generate_pseudo_legal_moves(&acting_board);
    if !pseudo.iter().any(|m| m == acting_move) {
        return false;
    }
    let after_board = apply_move(&acting_board, acting_move);

    let captured_value = board
        .piece_at(acting_move.to)
        .map(|p| p.piece_type.value());

    let relative_attacks = relative_unsafe_piece_attacks(
        &after_board, threatened_piece, acting_color, captured_value,
    );

    if !relative_attacks.is_empty() {
        return true;
    }

    let is_low_value = threatened_piece.piece.piece_type.value() < PieceType::Queen.value();
    is_low_value && any_move_is_checkmate(&after_board)
}

/// Returns `true` if every move in `acting_moves` creates (or leaves) a
/// counterthreat greater than that imposed on `threatened_piece`.
///
/// `equality_strategy`:
/// - `"creates"` — the threat must be a *direct result* of the acting move.
/// - `"leaves"` — it is enough that the threat *exists* after the move.
pub fn has_danger_levels(
    board: &Board,
    threatened_piece: &BoardPiece,
    acting_moves: &[Move],
    equality_strategy: DangerEqualityStrategy,
) -> bool {
    if acting_moves.is_empty() {
        return false;
    }
    acting_moves.iter().all(|mv| match equality_strategy {
        DangerEqualityStrategy::Creates => {
            move_creates_greater_threat(board, threatened_piece, mv)
        }
        DangerEqualityStrategy::Leaves => {
            move_leaves_greater_threat(board, threatened_piece, mv)
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangerEqualityStrategy {
    /// The counterthreat must be a direct result of the acting move.
    Creates,
    /// It is enough that the counterthreat exists after the move.
    Leaves,
}
