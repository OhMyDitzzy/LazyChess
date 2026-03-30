use crate::board::Board;
use crate::types::{Color, PieceType};
use super::attackers::get_attackers;
use super::defenders::get_defenders;
use super::types::BoardPiece;

/// Returns `true` if `piece` is safe on its current square — i.e., it cannot
/// be won by the opponent through a sequence of exchanges.
pub fn is_piece_safe(board: &Board, piece: &BoardPiece) -> bool {
    let attacker_color = piece.piece.color.opposite();

    let direct_attackers = super::attackers::get_direct_attackers(
        board, piece.square, attacker_color,
    );
    let attackers = get_attackers(board, piece.square, attacker_color);
    let defenders = get_defenders(board, piece, true);

    // A piece is safe if it has no direct attacker of lower value.
    let has_lower_value_attacker = direct_attackers.iter().any(|attacker| {
        attacker.piece.piece_type.value() < piece.piece.piece_type.value()
    });
    if has_lower_value_attacker {
        return false;
    }

    // A piece with no more attackers than defenders is safe.
    if attackers.len() <= defenders.len() {
        return true;
    }

    // A piece lower in value than all direct attackers, and with any defender
    // lower in value than all direct attackers, is safe.
    let lowest_attacker_value = direct_attackers
        .iter()
        .map(|a| a.piece.piece_type.value())
        .min();

    if let Some(lav) = lowest_attacker_value
        && piece.piece.piece_type.value() < lav
            && defenders.iter().any(|d| d.piece.piece_type.value() < lav)
        {
            return true;
        }

    // A piece defended by any pawn is safe (pawn is cheapest defender).
    if defenders.iter().any(|d| d.piece.piece_type == PieceType::Pawn) {
        return true;
    }

    false
}

/// Returns all pieces of `color` that are currently unsafe on the board.
///
/// Pawns and kings are excluded — pawns because their value makes safety
/// calculations less meaningful, kings because they cannot be captured.
///
/// If `last_move_captured` is `Some(value)`, pieces whose value is less than
/// or equal to the captured piece's value are also excluded (they would be
/// willingly traded away).
pub fn get_unsafe_pieces(
    board: &Board,
    color: Color,
    last_move_captured_value: Option<i32>,
) -> Vec<BoardPiece> {
    let captured_value = last_move_captured_value.unwrap_or(0);

    board
        .squares()
        .into_iter()
        .enumerate()
        .filter_map(|(sq, cell)| {
            let piece = cell.filter(|p| {
                p.color == color
                    && p.piece_type != PieceType::Pawn
                    && p.piece_type != PieceType::King
                    && p.piece_type.value() > captured_value
            })?;
            Some(BoardPiece::new(piece, sq as u8))
        })
        .filter(|bp| !is_piece_safe(board, bp))
        .collect()
}
