use crate::board::Board;
use crate::types::*;

const FILE_A: u64 = 0x0101010101010101;
const FILE_H: u64 = 0x8080808080808080;

const RANK_1: u64 = 0x00000000000000FF;
const RANK_3: u64 = 0x0000000000FF0000;
const RANK_6: u64 = 0x0000FF0000000000;
const RANK_8: u64 = 0xFF00000000000000;

/// Computes knight attack squares for a given square at compile time.
///
/// Used to populate the `KNIGHT_ATTACKS` static table; not called at runtime.
const fn knight_attacks_sq(sq: u8) -> u64 {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut bb: u64 = 0;
    if r + 2 < 8 && f + 1 < 8 {
        bb |= 1u64 << ((r + 2) * 8 + f + 1) as u32;
    }
    if r + 2 < 8 && f > 0 {
        bb |= 1u64 << ((r + 2) * 8 + f - 1) as u32;
    }
    if r - 2 >= 0 && f + 1 < 8 {
        bb |= 1u64 << ((r - 2) * 8 + f + 1) as u32;
    }
    if r - 2 >= 0 && f > 0 {
        bb |= 1u64 << ((r - 2) * 8 + f - 1) as u32;
    }
    if r + 1 < 8 && f + 2 < 8 {
        bb |= 1u64 << ((r + 1) * 8 + f + 2) as u32;
    }
    if r + 1 < 8 && f - 2 >= 0 {
        bb |= 1u64 << ((r + 1) * 8 + f - 2) as u32;
    }
    if r > 0 && f + 2 < 8 {
        bb |= 1u64 << ((r - 1) * 8 + f + 2) as u32;
    }
    if r > 0 && f - 2 >= 0 {
        bb |= 1u64 << ((r - 1) * 8 + f - 2) as u32;
    }
    bb
}

/// Computes king attack squares for a given square at compile time.
///
/// Used to populate the `KING_ATTACKS` static table; not called at runtime.
const fn king_attacks_sq(sq: u8) -> u64 {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut bb: u64 = 0;
    if r + 1 < 8 && f > 0 {
        bb |= 1u64 << ((r + 1) * 8 + f - 1) as u32;
    }
    if r + 1 < 8 {
        bb |= 1u64 << ((r + 1) * 8 + f) as u32;
    }
    if r + 1 < 8 && f + 1 < 8 {
        bb |= 1u64 << ((r + 1) * 8 + f + 1) as u32;
    }
    if f > 0 {
        bb |= 1u64 << (r * 8 + f - 1) as u32;
    }
    if f + 1 < 8 {
        bb |= 1u64 << (r * 8 + f + 1) as u32;
    }
    if r > 0 && f > 0 {
        bb |= 1u64 << ((r - 1) * 8 + f - 1) as u32;
    }
    if r > 0 {
        bb |= 1u64 << ((r - 1) * 8 + f) as u32;
    }
    if r > 0 && f + 1 < 8 {
        bb |= 1u64 << ((r - 1) * 8 + f + 1) as u32;
    }
    bb
}

static KNIGHT_ATTACKS: [u64; 64] = {
    let mut t = [0u64; 64];
    let mut i = 0u8;
    while i < 64 {
        t[i as usize] = knight_attacks_sq(i);
        i += 1;
    }
    t
};

static KING_ATTACKS: [u64; 64] = {
    let mut t = [0u64; 64];
    let mut i = 0u8;
    while i < 64 {
        t[i as usize] = king_attacks_sq(i);
        i += 1;
    }
    t
};

/// All squares attacked by white pawns on `pawns`.
#[inline(always)]
fn white_pawn_attacks(pawns: u64) -> u64 {
    ((pawns << 9) & !FILE_A) | ((pawns << 7) & !FILE_H)
}

/// All squares attacked by black pawns on `pawns`.
#[inline(always)]
fn black_pawn_attacks(pawns: u64) -> u64 {
    ((pawns >> 7) & !FILE_A) | ((pawns >> 9) & !FILE_H)
}

/// Classical ray-casting rook attacks from `sq` against the given occupancy.
///
/// Slides outward in all four orthogonal directions, stopping at (and
/// including) the first occupied square in each direction.
#[inline]
fn rook_attacks(sq: Square, occupied: u64) -> u64 {
    let mut attacks = 0u64;
    let rank = (sq / 8) as i32;
    let file = (sq % 8) as i32;

    macro_rules! ray {
        ($dr:expr, $df:expr) => {{
            let (mut r, mut f) = (rank + $dr, file + $df);
            while (0..8).contains(&r) && (0..8).contains(&f) {
                let bit = 1u64 << (r * 8 + f);
                attacks |= bit;
                if occupied & bit != 0 {
                    break;
                }
                r += $dr;
                f += $df;
            }
        }};
    }
    ray!(1, 0);
    ray!(-1, 0);
    ray!(0, 1);
    ray!(0, -1);
    attacks
}

/// Classical ray-casting bishop attacks from `sq` against the given occupancy.
///
/// Slides outward in all four diagonal directions, stopping at (and
/// including) the first occupied square in each direction.
#[inline]
fn bishop_attacks(sq: Square, occupied: u64) -> u64 {
    let mut attacks = 0u64;
    let rank = (sq / 8) as i32;
    let file = (sq % 8) as i32;

    macro_rules! ray {
        ($dr:expr, $df:expr) => {{
            let (mut r, mut f) = (rank + $dr, file + $df);
            while (0..8).contains(&r) && (0..8).contains(&f) {
                let bit = 1u64 << (r * 8 + f);
                attacks |= bit;
                if occupied & bit != 0 {
                    break;
                }
                r += $dr;
                f += $df;
            }
        }};
    }
    ray!(1, 1);
    ray!(1, -1);
    ray!(-1, 1);
    ray!(-1, -1);
    attacks
}

/// Returns `true` if `sq` is attacked by any piece belonging to `by_color`.
pub fn is_square_attacked(board: &Board, sq: Square, by_color: Color) -> bool {
    let occ = board.all_occupancy();
    let bi = by_color.index();

    // Knights
    if KNIGHT_ATTACKS[sq as usize] & board.bb[bi][PieceType::Knight.index()] != 0 {
        return true;
    }
    // King
    if KING_ATTACKS[sq as usize] & board.bb[bi][PieceType::King.index()] != 0 {
        return true;
    }
    // Diagonal sliders (bishop + queen)
    let diag = board.bb[bi][PieceType::Bishop.index()] | board.bb[bi][PieceType::Queen.index()];
    if bishop_attacks(sq, occ) & diag != 0 {
        return true;
    }
    // Straight sliders (rook + queen)
    let straight = board.bb[bi][PieceType::Rook.index()] | board.bb[bi][PieceType::Queen.index()];
    if rook_attacks(sq, occ) & straight != 0 {
        return true;
    }
    // Pawns
    let their_pawns = board.bb[bi][PieceType::Pawn.index()];
    let pawn_atk = match by_color {
        Color::White => white_pawn_attacks(their_pawns),
        Color::Black => black_pawn_attacks(their_pawns),
    };
    if pawn_atk & (1u64 << sq) != 0 {
        return true;
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

/// Pushes all four promotion variants (Q, R, B, N) for a pawn moving
/// from `from` to `to` onto `moves`.
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

/// Expands a target bitboard into individual `Move::new` entries appended to
/// `moves`.
#[inline]
fn add_quiet_moves(from: Square, mut targets: u64, moves: &mut Vec<Move>) {
    while targets != 0 {
        let to = targets.trailing_zeros() as Square;
        targets &= targets - 1;
        moves.push(Move::new(from, to));
    }
}

/// Generates all pseudo-legal pawn moves for `us` and appends them to `moves`.
///
/// Covers single and double pushes, diagonal captures, en-passant captures,
/// and all promotion variants for both colours.
fn gen_pawn_moves(board: &Board, us: Color, their: u64, empty: u64, moves: &mut Vec<Move>) {
    let pawns = board.piece_bb(us, PieceType::Pawn);
    let ep_bit = board.en_passant.map_or(0u64, |sq| 1u64 << sq);
    let captures_to = their | ep_bit;

    match us {
        Color::White => {
            let single = (pawns << 8) & empty;
            let double = ((single & RANK_3) << 8) & empty;
            let cap_r = (pawns << 9) & !FILE_A & captures_to;
            let cap_l = (pawns << 7) & !FILE_H & captures_to;

            // Single pushes (non-promo)
            let mut bb = single & !RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                moves.push(Move::new(to - 8, to));
            }
            // Double pushes
            let mut bb = double;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                moves.push(Move::with_flag(to - 16, to, MoveFlag::DoublePawnPush));
            }
            // Promotions (push)
            let mut bb = single & RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to - 8, to, moves);
            }
            // Captures right (non-promo)
            let mut bb = cap_r & !RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                let from = to - 9;
                if ep_bit >> to & 1 != 0 {
                    moves.push(Move::with_flag(from, to, MoveFlag::EnPassant));
                } else {
                    moves.push(Move::new(from, to));
                }
            }
            // Captures left (non-promo)
            let mut bb = cap_l & !RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                let from = to - 7;
                if ep_bit >> to & 1 != 0 {
                    moves.push(Move::with_flag(from, to, MoveFlag::EnPassant));
                } else {
                    moves.push(Move::new(from, to));
                }
            }
            // Promo captures right
            let mut bb = cap_r & RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to - 9, to, moves);
            }
            // Promo captures left
            let mut bb = cap_l & RANK_8;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to - 7, to, moves);
            }
        }

        Color::Black => {
            let single = (pawns >> 8) & empty;
            let double = ((single & RANK_6) >> 8) & empty;
            let cap_r = (pawns >> 7) & !FILE_A & captures_to;
            let cap_l = (pawns >> 9) & !FILE_H & captures_to;

            // Single pushes (non-promo)
            let mut bb = single & !RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                moves.push(Move::new(to + 8, to));
            }
            // Double pushes
            let mut bb = double;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                moves.push(Move::with_flag(to + 16, to, MoveFlag::DoublePawnPush));
            }
            // Promotions (push)
            let mut bb = single & RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to + 8, to, moves);
            }
            // Captures right (non-promo) — toward a-file for black
            let mut bb = cap_r & !RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                let from = to + 7;
                if ep_bit >> to & 1 != 0 {
                    moves.push(Move::with_flag(from, to, MoveFlag::EnPassant));
                } else {
                    moves.push(Move::new(from, to));
                }
            }
            // Captures left (non-promo) — toward h-file for black
            let mut bb = cap_l & !RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                let from = to + 9;
                if ep_bit >> to & 1 != 0 {
                    moves.push(Move::with_flag(from, to, MoveFlag::EnPassant));
                } else {
                    moves.push(Move::new(from, to));
                }
            }
            // Promo captures right
            let mut bb = cap_r & RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to + 7, to, moves);
            }
            // Promo captures left
            let mut bb = cap_l & RANK_1;
            while bb != 0 {
                let to = bb.trailing_zeros() as Square;
                bb &= bb - 1;
                push_promotions(to + 9, to, moves);
            }
        }
    }
}

/// Appends castling moves for `color` if the rights and vacancy conditions are
/// met.
///
/// Does **not** check whether the king passes through check — that is deferred
/// to [`generate_legal_moves`].
fn gen_castling(board: &Board, king_sq: Square, color: Color, moves: &mut Vec<Move>) {
    let occ = board.all_occupancy();
    match color {
        Color::White => {
            if king_sq != 4 {
                return;
            }
            // f1 (5), g1 (6) must be empty
            if board.castling_rights.white_kingside && occ & 0x0000000000000060 == 0 {
                moves.push(Move::with_flag(4, 6, MoveFlag::CastleKingside));
            }
            // b1 (1), c1 (2), d1 (3) must be empty
            if board.castling_rights.white_queenside && occ & 0x000000000000000E == 0 {
                moves.push(Move::with_flag(4, 2, MoveFlag::CastleQueenside));
            }
        }
        Color::Black => {
            if king_sq != 60 {
                return;
            }
            // f8 (61), g8 (62) must be empty
            if board.castling_rights.black_kingside && occ & 0x6000000000000000 == 0 {
                moves.push(Move::with_flag(60, 62, MoveFlag::CastleKingside));
            }
            // b8 (57), c8 (58), d8 (59) must be empty
            if board.castling_rights.black_queenside && occ & 0x0E00000000000000 == 0 {
                moves.push(Move::with_flag(60, 58, MoveFlag::CastleQueenside));
            }
        }
    }
}

/// Generates all pseudo-legal moves for the side to move.
///
/// *Pseudo-legal* means moves that respect each piece's movement rules but may
/// leave the king in check. Filtering for full legality is done in
/// [`generate_legal_moves`].
pub fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves: Vec<Move> = Vec::with_capacity(48);
    let us = board.side_to_move;
    let occ = board.all_occupancy();
    let our = board.occupancy(us);
    let their = board.occupancy(us.opposite());
    let empty = !occ;
    let not_our = !our;

    gen_pawn_moves(board, us, their, empty, &mut moves);

    // Knights
    let mut bb = board.piece_bb(us, PieceType::Knight);
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        add_quiet_moves(from, KNIGHT_ATTACKS[from as usize] & not_our, &mut moves);
    }

    // Bishops
    let mut bb = board.piece_bb(us, PieceType::Bishop);
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        add_quiet_moves(from, bishop_attacks(from, occ) & not_our, &mut moves);
    }

    // Rooks
    let mut bb = board.piece_bb(us, PieceType::Rook);
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        add_quiet_moves(from, rook_attacks(from, occ) & not_our, &mut moves);
    }

    // Queens
    let mut bb = board.piece_bb(us, PieceType::Queen);
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let atk = (bishop_attacks(from, occ) | rook_attacks(from, occ)) & not_our;
        add_quiet_moves(from, atk, &mut moves);
    }

    // King
    let king_sq = board.king_square(us).unwrap_or(64);
    if king_sq < 64 {
        add_quiet_moves(
            king_sq,
            KING_ATTACKS[king_sq as usize] & not_our,
            &mut moves,
        );
        gen_castling(board, king_sq, us, &mut moves);
    }

    moves
}

/// Applies `mv` to `board` and returns the resulting board state.
///
/// All bitboard updates (captures, castling rook moves, en-passant removal,
/// promotions, castling-rights stripping) are handled here on the hot path
/// without any `piece_at` lookups.
pub fn apply_move(board: &Board, mv: &Move) -> Board {
    let mut nb = board.clone();
    let us = board.side_to_move;
    let them = us.opposite();
    let ui = us.index();
    let ti = them.index();
    let bit_from = 1u64 << mv.from;
    let bit_to = 1u64 << mv.to;

    // Identify the moving piece type (O(6), us only).
    let mut pt_idx = 0usize;
    for pi in 0..6usize {
        if nb.bb[ui][pi] & bit_from != 0 {
            pt_idx = pi;
            break;
        }
    }
    let pt = PieceType::from_index(pt_idx);

    // Half-move clock: reset on pawn moves or captures.
    let is_capture =
        board.bb[ti].iter().any(|&bb| bb & bit_to != 0) || mv.flag == MoveFlag::EnPassant;
    if is_capture || pt == PieceType::Pawn {
        nb.halfmove_clock = 0;
    } else {
        nb.halfmove_clock += 1;
    }

    // En passant target (only valid for double pawn pushes).
    nb.en_passant = if mv.flag == MoveFlag::DoublePawnPush {
        Some(if us == Color::White {
            mv.to - 8
        } else {
            mv.to + 8
        })
    } else {
        None
    };

    // Remove the moving piece from its source square.
    nb.bb[ui][pt_idx] &= !bit_from;

    match &mv.flag {
        MoveFlag::EnPassant => {
            // The captured pawn is not at mv.to but one rank behind it.
            let cap_sq = if us == Color::White {
                mv.to - 8
            } else {
                mv.to + 8
            };
            nb.bb[ti][PieceType::Pawn.index()] &= !(1u64 << cap_sq);
            // Place moving pawn.
            nb.bb[ui][pt_idx] |= bit_to;
        }

        MoveFlag::CastleKingside => {
            let (rook_from, rook_to) = match us {
                Color::White => (7u8, 5u8),
                Color::Black => (63u8, 61u8),
            };
            nb.bb[ui][PieceType::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
            nb.bb[ui][pt_idx] |= bit_to; // place king
        }

        MoveFlag::CastleQueenside => {
            let (rook_from, rook_to) = match us {
                Color::White => (0u8, 3u8),
                Color::Black => (56u8, 59u8),
            };
            nb.bb[ui][PieceType::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
            nb.bb[ui][pt_idx] |= bit_to; // place king
        }

        MoveFlag::Promotion(promo_type) => {
            // Clear any captured enemy piece at the destination.
            for pi in 0..6usize {
                nb.bb[ti][pi] &= !bit_to;
            }
            strip_castling_on_capture(&mut nb, mv.to);
            // Place promoted piece (not the original pawn type).
            nb.bb[ui][promo_type.index()] |= bit_to;
            strip_castling_on_move(&mut nb, mv.from, pt);
            nb.side_to_move = them;
            if us == Color::Black {
                nb.fullmove_number += 1;
            }
            return nb;
        }

        MoveFlag::Normal | MoveFlag::DoublePawnPush => {
            // Clear any captured enemy piece.
            for pi in 0..6usize {
                nb.bb[ti][pi] &= !bit_to;
            }
            strip_castling_on_capture(&mut nb, mv.to);
            nb.bb[ui][pt_idx] |= bit_to;
        }
    }

    strip_castling_on_move(&mut nb, mv.from, pt);
    nb.side_to_move = them;
    if us == Color::Black {
        nb.fullmove_number += 1;
    }
    nb
}

/// Revokes castling rights when a rook's starting square is captured.
fn strip_castling_on_capture(board: &mut Board, to: Square) {
    match to {
        0 => board.castling_rights.white_queenside = false,
        7 => board.castling_rights.white_kingside = false,
        56 => board.castling_rights.black_queenside = false,
        63 => board.castling_rights.black_kingside = false,
        _ => {}
    }
}

/// Revokes castling rights when a king or rook moves away from its home square.
fn strip_castling_on_move(board: &mut Board, from: Square, pt: PieceType) {
    match (board.side_to_move, pt) {
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
/// Filters the pseudo-legal move list by:
/// - Rejecting moves that leave the king in check.
/// - For castling, additionally verifying the king does not start in check and
///   does not pass through an attacked square.
pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
    let color = board.side_to_move;
    generate_pseudo_legal_moves(board)
        .into_iter()
        .filter(|mv| {
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
            let new_board = apply_move(board, mv);
            !is_in_check(&new_board, color)
        })
        .collect()
}
