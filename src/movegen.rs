use crate::board::Board;
use crate::types::*;

const KNIGHT_DIRS: [(i32, i32); 8] = [
    (-2, -1), (-2, 1), (-1, -2), (-1, 2),
    (1, -2),  (1, 2),  (2, -1),  (2, 1),
];

const KING_DIRS: [(i32, i32); 8] = [
    (-1, -1), (-1, 0), (-1, 1),
    (0, -1),           (0, 1),
    (1, -1),  (1, 0),  (1, 1),
];

const BISHOP_DIRS: [(i32, i32); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const ROOK_DIRS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

#[inline(always)]
fn to_sq(rank: i32, file: i32) -> Option<Square> {
    if rank >= 0 && rank < 8 && file >= 0 && file < 8 {
        Some((rank * 8 + file) as Square)
    } else {
        None
    }
}

/// Generates all pseudo-legal moves for the side to move.
///
/// *Pseudo-legal* means moves that respect each piece's movement rules but may
/// leave the king in check. Filtering for full legality is done in
/// [`generate_legal_moves`].
pub fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves: Vec<Move> = Vec::with_capacity(48);
    for sq in 0u8..64 {
        if let Some(p) = board.piece_at(sq) {
            if p.color == board.side_to_move {
                match p.piece_type {
                    PieceType::Pawn => gen_pawn_moves(board, sq, &mut moves),
                    PieceType::Knight => gen_knight_moves(board, sq, &mut moves),
                    PieceType::Bishop => gen_sliding(board, sq, &BISHOP_DIRS, &mut moves),
                    PieceType::Rook => gen_sliding(board, sq, &ROOK_DIRS, &mut moves),
                    PieceType::Queen => {
                        gen_sliding(board, sq, &BISHOP_DIRS, &mut moves);
                        gen_sliding(board, sq, &ROOK_DIRS, &mut moves);
                    }
                    PieceType::King => gen_king_moves(board, sq, &mut moves),
                }
            }
        }
    }
    moves
}

fn gen_pawn_moves(board: &Board, from: Square, moves: &mut Vec<Move>) {
    let color = board.side_to_move;
    let rank = rank_of(from) as i32;
    let file = file_of(from) as i32;

    // White pawns advance upward (+1 rank); Black advance downward (-1 rank).
    let (dir, start_rank, promo_rank) = match color {
        Color::White => (1i32, 1i32, 6i32),
        Color::Black => (-1i32, 6i32, 1i32),
    };

    let target_rank = rank + dir;

    // Single-square push
    if let Some(to) = to_sq(target_rank, file) {
        if board.piece_at(to).is_none() {
            if rank == promo_rank {
                push_promotions(from, to, moves);
            } else {
                moves.push(Move::new(from, to));

                // Double push from the starting rank
                if rank == start_rank {
                    if let Some(to2) = to_sq(rank + 2 * dir, file) {
                        if board.piece_at(to2).is_none() {
                            moves.push(Move::with_flag(from, to2, MoveFlag::DoublePawnPush));
                        }
                    }
                }
            }
        }
    }

    // Diagonal captures (including en passant)
    for &df in &[-1i32, 1i32] {
        if let Some(to) = to_sq(target_rank, file + df) {
            let is_ep = board.en_passant == Some(to);
            let has_enemy = board
                .piece_at(to)
                .map(|p| p.color != color)
                .unwrap_or(false);

            if has_enemy || is_ep {
                if rank == promo_rank {
                    push_promotions(from, to, moves);
                } else if is_ep {
                    moves.push(Move::with_flag(from, to, MoveFlag::EnPassant));
                } else {
                    moves.push(Move::new(from, to));
                }
            }
        }
    }
}

/// Pushes all four promotion variants (Q, R, B, N) onto the move list.
#[inline]
fn push_promotions(from: Square, to: Square, moves: &mut Vec<Move>) {
    for &pt in &[
        PieceType::Queen,
        PieceType::Rook,
        PieceType::Bishop,
        PieceType::Knight,
    ] {
        moves.push(Move::with_flag(from, to, MoveFlag::Promotion(pt)));
    }
}

fn gen_knight_moves(board: &Board, from: Square, moves: &mut Vec<Move>) {
    let color = board.side_to_move;
    let rank = rank_of(from) as i32;
    let file = file_of(from) as i32;

    for &(dr, df) in &KNIGHT_DIRS {
        if let Some(to) = to_sq(rank + dr, file + df) {
            if board.piece_at(to).map(|p| p.color != color).unwrap_or(true) {
                moves.push(Move::new(from, to));
            }
        }
    }
}

fn gen_sliding(board: &Board, from: Square, dirs: &[(i32, i32)], moves: &mut Vec<Move>) {
    let color = board.side_to_move;
    let rank = rank_of(from) as i32;
    let file = file_of(from) as i32;

    for &(dr, df) in dirs {
        let (mut r, mut f) = (rank + dr, file + df);
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let to = (r * 8 + f) as Square;
            match board.piece_at(to) {
                None => moves.push(Move::new(from, to)),
                Some(p) => {
                    if p.color != color {
                        moves.push(Move::new(from, to));
                    }
                    break;
                }
            }
            r += dr;
            f += df;
        }
    }
}

fn gen_king_moves(board: &Board, from: Square, moves: &mut Vec<Move>) {
    let color = board.side_to_move;
    let rank = rank_of(from) as i32;
    let file = file_of(from) as i32;

    for &(dr, df) in &KING_DIRS {
        if let Some(to) = to_sq(rank + dr, file + df) {
            if board.piece_at(to).map(|p| p.color != color).unwrap_or(true) {
                moves.push(Move::new(from, to));
            }
        }
    }

    gen_castling(board, from, color, moves);
}

fn gen_castling(board: &Board, king_sq: Square, color: Color, moves: &mut Vec<Move>) {
    match color {
        Color::White => {
            if king_sq != 4 {
                return; // King not on e1; castling rights are meaningless.
            }
            if board.castling_rights.white_kingside
                && board.piece_at(5).is_none()
                && board.piece_at(6).is_none()
            {
                moves.push(Move::with_flag(4, 6, MoveFlag::CastleKingside));
            }
            if board.castling_rights.white_queenside
                && board.piece_at(1).is_none()
                && board.piece_at(2).is_none()
                && board.piece_at(3).is_none()
            {
                moves.push(Move::with_flag(4, 2, MoveFlag::CastleQueenside));
            }
        }
        Color::Black => {
            if king_sq != 60 {
                return;
            }
            if board.castling_rights.black_kingside
                && board.piece_at(61).is_none()
                && board.piece_at(62).is_none()
            {
                moves.push(Move::with_flag(60, 62, MoveFlag::CastleKingside));
            }
            if board.castling_rights.black_queenside
                && board.piece_at(57).is_none()
                && board.piece_at(58).is_none()
                && board.piece_at(59).is_none()
            {
                moves.push(Move::with_flag(60, 58, MoveFlag::CastleQueenside));
            }
        }
    }
}

/// Returns `true` if `sq` is attacked by any piece belonging to `by_color`.
pub fn is_square_attacked(board: &Board, sq: Square, by_color: Color) -> bool {
    let rank = rank_of(sq) as i32;
    let file = file_of(sq) as i32;

    // Knight attacks
    for &(dr, df) in &KNIGHT_DIRS {
        if let Some(from) = to_sq(rank + dr, file + df) {
            if matches!(board.piece_at(from),
                Some(Piece { piece_type: PieceType::Knight, color }) if color == by_color)
            {
                return true;
            }
        }
    }

    // King attacks (needed to prevent the king from walking into an adjacent king)
    for &(dr, df) in &KING_DIRS {
        if let Some(from) = to_sq(rank + dr, file + df) {
            if matches!(board.piece_at(from),
                Some(Piece { piece_type: PieceType::King, color }) if color == by_color)
            {
                return true;
            }
        }
    }

    // Diagonal sliders (Bishop / Queen)
    for &(dr, df) in &BISHOP_DIRS {
        let (mut r, mut f) = (rank + dr, file + df);
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            if let Some(p) = board.piece_at((r * 8 + f) as Square) {
                if p.color == by_color
                    && (p.piece_type == PieceType::Bishop
                        || p.piece_type == PieceType::Queen)
                {
                    return true;
                }
                break;
            }
            r += dr;
            f += df;
        }
    }

    // Straight sliders (Rook / Queen)
    for &(dr, df) in &ROOK_DIRS {
        let (mut r, mut f) = (rank + dr, file + df);
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            if let Some(p) = board.piece_at((r * 8 + f) as Square) {
                if p.color == by_color
                    && (p.piece_type == PieceType::Rook || p.piece_type == PieceType::Queen)
                {
                    return true;
                }
                break;
            }
            r += dr;
            f += df;
        }
    }

    // Pawn attacks – a pawn of `by_color` attacks diagonally *forward* from its
    // own perspective, which is *backward* from the target square's perspective.
    let pawn_dir: i32 = match by_color {
        Color::White => -1, // White pawns stand below the target square.
        Color::Black => 1,
    };
    for &df in &[-1i32, 1i32] {
        if let Some(from) = to_sq(rank + pawn_dir, file + df) {
            if matches!(board.piece_at(from),
                Some(Piece { piece_type: PieceType::Pawn, color }) if color == by_color)
            {
                return true;
            }
        }
    }

    false
}

/// Returns `true` if the king of `color` is currently in check.
pub fn is_in_check(board: &Board, color: Color) -> bool {
    board
        .king_square(color)
        .map(|sq| is_square_attacked(board, sq, color.opposite()))
        .unwrap_or(false)
}

/// Applies `mv` to `board` and returns the resulting board state.
///
/// The caller must ensure `mv` was generated from `board`; applying an
/// arbitrary move to a mismatched board is undefined behaviour.
pub fn apply_move(board: &Board, mv: &Move) -> Board {
    let mut nb = board.clone();
    let piece = nb
        .take_piece(mv.from)
        .expect("apply_move: no piece at the from square");

    // Half-move clock: reset on pawn moves or captures.
    let is_capture =
        board.piece_at(mv.to).is_some() || mv.flag == MoveFlag::EnPassant;
    if is_capture || piece.piece_type == PieceType::Pawn {
        nb.halfmove_clock = 0;
    } else {
        nb.halfmove_clock += 1;
    }

    // En passant target square is valid only immediately after a double pawn push.
    nb.en_passant = if mv.flag == MoveFlag::DoublePawnPush {
        let ep_rank = if piece.color == Color::White {
            rank_of(mv.from) + 1
        } else {
            rank_of(mv.from) - 1
        };
        Some(make_square(file_of(mv.from), ep_rank))
    } else {
        None
    };

    // Handle move-specific side effects before placing the piece.
    match &mv.flag {
        MoveFlag::EnPassant => {
            // The captured pawn sits on the same rank as the capturing pawn
            // but on the file of the destination square.
            let captured_sq = make_square(file_of(mv.to), rank_of(mv.from));
            nb.take_piece(captured_sq);
        }

        MoveFlag::CastleKingside => {
            // Move the rook; the king will be placed below.
            let (rook_from, rook_to) = match piece.color {
                Color::White => (7u8, 5u8),  // h1 → f1
                Color::Black => (63u8, 61u8), // h8 → f8
            };
            let rook = nb.take_piece(rook_from);
            nb.set_piece(rook_to, rook);
        }

        MoveFlag::CastleQueenside => {
            let (rook_from, rook_to) = match piece.color {
                Color::White => (0u8, 3u8),  // a1 → d1
                Color::Black => (56u8, 59u8), // a8 → d8
            };
            let rook = nb.take_piece(rook_from);
            nb.set_piece(rook_to, rook);
        }

        MoveFlag::Promotion(promo_type) => {
            // Place the promoted piece (overwrites any captured piece at `to`).
            strip_castling_on_capture(&mut nb, mv.to);
            nb.set_piece(mv.to, Some(Piece::new(*promo_type, piece.color)));
            nb.side_to_move = piece.color.opposite();
            if piece.color == Color::Black {
                nb.fullmove_number += 1;
            }
            return nb; // Early return; castling-rights updates for pawn moves are no-ops.
        }

        MoveFlag::Normal | MoveFlag::DoublePawnPush => {
            strip_castling_on_capture(&mut nb, mv.to);
        }
    }

    // Place the moving piece at the destination.
    nb.set_piece(mv.to, Some(piece));

    // Update castling rights based on the piece that moved.
    strip_castling_on_move(&mut nb, mv.from, piece);

    nb.side_to_move = piece.color.opposite();
    if piece.color == Color::Black {
        nb.fullmove_number += 1;
    }

    nb
}

/// Revokes castling rights when a rook is captured on its home square.
fn strip_castling_on_capture(board: &mut Board, to: Square) {
    match to {
        0 => board.castling_rights.white_queenside = false,
        7 => board.castling_rights.white_kingside = false,
        56 => board.castling_rights.black_queenside = false,
        63 => board.castling_rights.black_kingside = false,
        _ => {}
    }
}

/// Revokes castling rights when the king or a rook moves away from its home square.
fn strip_castling_on_move(board: &mut Board, from: Square, piece: Piece) {
    match (piece.color, piece.piece_type) {
        (Color::White, PieceType::King) => {
            board.castling_rights.white_kingside = false;
            board.castling_rights.white_queenside = false;
        }
        (Color::Black, PieceType::King) => {
            board.castling_rights.black_kingside = false;
            board.castling_rights.black_queenside = false;
        }
        (Color::White, PieceType::Rook) => {
            if from == 0 {
                board.castling_rights.white_queenside = false;
            } else if from == 7 {
                board.castling_rights.white_kingside = false;
            }
        }
        (Color::Black, PieceType::Rook) => {
            if from == 56 {
                board.castling_rights.black_queenside = false;
            } else if from == 63 {
                board.castling_rights.black_kingside = false;
            }
        }
        _ => {}
    }
}

/// Generates all *legal* moves for the side to move.
///
/// A move is legal when it does not leave the mover's king in check and, for
/// castling, the king does not pass through or land on an attacked square.
pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
    let color = board.side_to_move;
    generate_pseudo_legal_moves(board)
        .into_iter()
        .filter(|mv| {
            // Castling has additional constraints beyond the post-move check test.
            match mv.flag {
                MoveFlag::CastleKingside => {
                    if is_in_check(board, color) {
                        return false;
                    }
                    // King passes through mv.from + 1 (f1 / f8).
                    if is_square_attacked(board, mv.from + 1, color.opposite()) {
                        return false;
                    }
                }
                MoveFlag::CastleQueenside => {
                    if is_in_check(board, color) {
                        return false;
                    }
                    // King passes through mv.from - 1 (d1 / d8).
                    if is_square_attacked(board, mv.from - 1, color.opposite()) {
                        return false;
                    }
                }
                _ => {}
            }

            // The king must not be in check after the move is applied.
            let new_board = apply_move(board, mv);
            !is_in_check(&new_board, color)
        })
        .collect()
}
