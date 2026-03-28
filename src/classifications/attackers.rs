use crate::board::Board;
use crate::movegen::{apply_move, generate_pseudo_legal_moves};
use crate::types::{Color, Move, MoveFlag, PieceType, Square, file_of, rank_of};
use super::types::BoardPiece;

/// Returns all pieces of `by_color` that directly attack `square`.
pub fn get_direct_attackers(board: &Board, square: Square, by_color: Color) -> Vec<BoardPiece> {
    // Generate pseudo-legal moves for the attacking side and keep only those
    // that land on our target square. We use pseudo-legal moves so that pinned
    // pieces are included — a pinned piece still "attacks" a square even if
    // moving there would leave the king in check.
    let attacker_board = with_side_to_move(board, by_color);
    generate_pseudo_legal_moves(&attacker_board)
        .into_iter()
        .filter(|mv| capture_square(mv) == square)
        .filter_map(|mv| {
            board.piece_at(mv.from).map(|p| BoardPiece::new(p, mv.from))
        })
        .collect::<Vec<_>>()
        // Deduplicate: a queen/rook/bishop may appear multiple times via
        // different directions. Keep unique squares only.
        .into_iter()
        .fold(Vec::new(), |mut acc, bp| {
            if !acc.iter().any(|x: &BoardPiece| x.square == bp.square) {
                acc.push(bp);
            }
            acc
        })
}

/// Returns all pieces of `by_color` that attack `square`, including pieces
/// that are revealed after the front piece in a battery is removed (transitive
/// / X-ray attackers).
pub fn get_attackers(board: &Board, square: Square, by_color: Color) -> Vec<BoardPiece> {
    let mut all_attackers = get_direct_attackers(board, square, by_color);

    // Work through a frontier of direct attackers.  For each one, remove it
    // from a scratch board and re-check for newly revealed (X-ray) attackers.
    let mut frontier: Vec<(Board, Square, PieceType)> = all_attackers
        .iter()
        .map(|bp| (board.clone(), bp.square, bp.piece.piece_type))
        .collect();

    while let Some((ref_board, front_sq, front_type)) = frontier.pop() {
        // Kings cannot be at the front of a battery.
        if front_type == PieceType::King {
            continue;
        }

        // Remove the front piece from the scratch board.
        let mut scratch = ref_board.clone();
        scratch.set_piece(front_sq, None);

        // Re-compute direct attackers on the scratch board.
        let revealed = get_direct_attackers(&scratch, square, by_color)
            .into_iter()
            .filter(|bp| !all_attackers.iter().any(|a| a.square == bp.square))
            .collect::<Vec<_>>();

        for bp in revealed {
            frontier.push((scratch.clone(), bp.square, bp.piece.piece_type));
            all_attackers.push(bp);
        }
    }

    all_attackers
}

/// Returns the square being captured by `mv`.
/// For en passant the captured pawn is on a different square than `mv.to`.
pub fn capture_square(mv: &Move) -> Square {
    if mv.flag == MoveFlag::EnPassant {
        // Captured pawn is on the same rank as `from` but the file of `to`.
        let rank = rank_of(mv.from);
        let file = file_of(mv.to);
        rank * 8 + file
    } else {
        mv.to
    }
}

/// Returns a copy of `board` with `side_to_move` set to `color`.
/// All other state (pieces, castling, en passant) is preserved.
pub fn with_side_to_move(board: &Board, color: Color) -> Board {
    let mut b = board.clone();
    b.side_to_move = color;
    b
}

/// Applies `mv` to `board` and returns the new board, or `None` if the move
/// is not pseudo-legal on `board`.
pub fn try_apply_move(board: &Board, mv: &Move) -> Option<Board> {
    let pseudo = generate_pseudo_legal_moves(board);
    if pseudo.iter().any(|m| m == mv) {
        Some(apply_move(board, mv))
    } else {
        None
    }
}

/// Checks whether a move results in any legal checkmate on the resulting board.
/// Used in danger level calculations for low-value piece sacrifices.
pub fn any_move_is_checkmate(board: &Board) -> bool {
    use crate::movegen::{generate_legal_moves, is_in_check};
    let moves = generate_legal_moves(board);
    moves.iter().any(|mv| {
        let next = apply_move(board, mv);
        let legal_responses = generate_legal_moves(&next);
        legal_responses.is_empty() && is_in_check(&next, next.side_to_move)
    })
}
