use crate::board::Board;
use crate::movegen::{apply_move, generate_pseudo_legal_moves};
use crate::types::PieceType;
use super::attackers::with_side_to_move;
use super::danger_levels::move_creates_greater_threat;
use super::piece_safety::is_piece_safe;
use super::types::BoardPiece;

/// Returns `true` if `piece` is trapped.
///
/// A piece is trapped when:
/// 1. It is currently unsafe on its square, AND
/// 2. Every square it can move to is also unsafe (or blocked), AND
/// 3. (Optionally) moving it allows the opponent a greater counterthreat.
pub fn is_piece_trapped(board: &Board, piece: &BoardPiece, check_danger_levels: bool) -> bool {
    // Generate moves from the piece's perspective.
    let calibrated = with_side_to_move(board, piece.piece.color);

    let standing_safe = is_piece_safe(&calibrated, piece);

    let piece_moves: Vec<_> = generate_pseudo_legal_moves(&calibrated)
        .into_iter()
        .filter(|mv| mv.from == piece.square)
        .collect();

    // A piece with no moves is trapped only if it is also unsafe.
    if piece_moves.is_empty() {
        return !standing_safe;
    }

    let all_moves_unsafe = piece_moves.iter().all(|mv| {
        // Capturing the king is not a real escape.
        if let Some(target) = calibrated.piece_at(mv.to)
            && target.piece_type == PieceType::King {
                return false;
            }

        // If danger levels are enabled, a move that creates a greater
        // counterthreat counts as "unsafe" for the escaping piece.
        if check_danger_levels && move_creates_greater_threat(&calibrated, piece, mv) {
            return true;
        }

        let escape_board = apply_move(&calibrated, mv);
        let escaped_piece = BoardPiece::new(piece.piece, mv.to);
        !is_piece_safe(&escape_board, &escaped_piece)
    });

    !standing_safe && all_moves_unsafe
}