use crate::board::Board;
use super::attackers::{get_attackers, get_direct_attackers, with_side_to_move};
use super::types::BoardPiece;

/// Returns the set of pieces that defend `piece` on `board`.
/// The algorithm:
/// 1. Find all direct attackers of `piece`.
/// 2. For each attacker, simulate taking `piece` and count the recapturers.
/// 3. Return the smallest such recapturer set (worst case for the attacker).
/// 4. If there are no attackers at all, flip the piece colour and count
///    attackers of the flipped piece — these are effectively the defenders.
pub fn get_defenders(board: &Board, piece: &BoardPiece, transitive: bool) -> Vec<BoardPiece> {
    let attacker_color = piece.piece.color.opposite();
    let direct_attackers = get_direct_attackers(board, piece.square, attacker_color);

    if direct_attackers.is_empty() {
        // No attackers — defenders are found by flipping the piece colour and
        // asking who attacks that flipped piece.
        let flipped_piece = BoardPiece::new(
            crate::types::Piece::new(piece.piece.piece_type, attacker_color),
            piece.square,
        );
        let mut scratch = board.clone();
        scratch.set_piece(piece.square, Some(flipped_piece.piece));
        return get_attackers_of(&scratch, &flipped_piece, transitive);
    }

    // Simulate each attacker taking the piece and find the recapture set.
    let smallest_recapturers = direct_attackers
        .iter()
        .filter_map(|attacker| {
            // Build a board where the attacker captures the piece.
            let capture_board = with_side_to_move(board, attacker_color);
            let capture_mv = crate::types::Move::new(attacker.square, piece.square);
            let pseudo = crate::movegen::generate_pseudo_legal_moves(&capture_board);
            if !pseudo.iter().any(|m| m == &capture_mv) {
                return None;
            }
            let after_capture = crate::movegen::apply_move(&capture_board, &capture_mv);

            // The attacker is now on piece.square — find who can recapture it.
            let recaptured_piece = BoardPiece::new(attacker.piece, piece.square);
            let recapturers = get_attackers_of(&after_capture, &recaptured_piece, transitive);
            Some(recapturers)
        })
        .min_by_key(|recapturers| recapturers.len());

    smallest_recapturers.unwrap_or_default()
}

/// Helper: get all attackers of a `BoardPiece` on `board`.
fn get_attackers_of(board: &Board, piece: &BoardPiece, transitive: bool) -> Vec<BoardPiece> {
    let defending_color = piece.piece.color.opposite();
    if transitive {
        get_attackers(board, piece.square, defending_color)
    } else {
        get_direct_attackers(board, piece.square, defending_color)
    }
}
