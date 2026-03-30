use crate::types::*;

impl PieceType {
    #[inline(always)]
    pub const fn index(self) -> usize {
        match self {
            PieceType::Pawn => 0,
            PieceType::Knight => 1,
            PieceType::Bishop => 2,
            PieceType::Rook => 3,
            PieceType::Queen => 4,
            PieceType::King => 5,
        }
    }

    #[inline(always)]
    pub const fn from_index(i: usize) -> Self {
        match i {
            0 => PieceType::Pawn,
            1 => PieceType::Knight,
            2 => PieceType::Bishop,
            3 => PieceType::Rook,
            4 => PieceType::Queen,
            _ => PieceType::King,
        }
    }
}

impl Color {
    /// White = 0, Black = 1.
    #[inline(always)]
    pub const fn index(self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 1,
        }
    }
}

/// The complete state of the chess board at a single point in time.
///
/// Piece positions are stored as twelve `u64` bitboards — one per
/// (colour, piece-type) pair. Bit *n* is set when a piece of that kind
/// occupies square *n* (a1 = 0 … h8 = 63).
///
/// All move-application logic lives in `movegen` to keep this struct simple.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Board {
    /// `bb[color_index][piece_type_index]`
    ///
    /// color  : White = 0 | Black = 1  
    /// piece  : Pawn=0 Knight=1 Bishop=2 Rook=3 Queen=4 King=5
    pub bb: [[u64; 6]; 2],
    pub side_to_move: Color,
    pub castling_rights: CastlingRights,
    /// Target square for a possible en-passant capture; `None` if not applicable.
    pub en_passant: Option<Square>,
    /// Half-move clock for the 50-move rule (resets on pawn moves or captures).
    pub halfmove_clock: u32,
    /// Full-move number, starting at 1 and incrementing after Black's move.
    pub fullmove_number: u32,
}

impl Board {
    /// Bitboard of every square occupied by `color`.
    #[inline(always)]
    pub fn occupancy(&self, color: Color) -> u64 {
        let c = color.index();
        self.bb[c][0]
            | self.bb[c][1]
            | self.bb[c][2]
            | self.bb[c][3]
            | self.bb[c][4]
            | self.bb[c][5]
    }

    /// Bitboard of every occupied square (both colours).
    #[inline(always)]
    pub fn all_occupancy(&self) -> u64 {
        self.occupancy(Color::White) | self.occupancy(Color::Black)
    }

    /// Bitboard for a specific (colour, piece-type) combination.
    #[inline(always)]
    pub fn piece_bb(&self, color: Color, pt: PieceType) -> u64 {
        self.bb[color.index()][pt.index()]
    }

    /// Returns the piece on `sq`, or `None` if the square is empty.
    ///
    /// Scans all 12 bitboards (O(12)); avoid on the movegen hot path —
    /// use bitboard operations directly there.
    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        let bit = 1u64 << sq;
        for ci in 0..2usize {
            for pi in 0..6usize {
                if self.bb[ci][pi] & bit != 0 {
                    let color = if ci == 0 { Color::White } else { Color::Black };
                    return Some(Piece::new(PieceType::from_index(pi), color));
                }
            }
        }
        None
    }

    /// Places `piece` on `sq`, first clearing any piece already there.
    #[inline]
    pub fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.clear_square(sq);
        if let Some(p) = piece {
            self.bb[p.color.index()][p.piece_type.index()] |= 1u64 << sq;
        }
    }

    /// Removes and returns whatever piece is on `sq`, or `None` if the square
    /// was already empty.
    #[inline]
    pub fn take_piece(&mut self, sq: Square) -> Option<Piece> {
        let p = self.piece_at(sq);
        if let Some(ref p) = p {
            self.bb[p.color.index()][p.piece_type.index()] &= !(1u64 << sq);
        }
        p
    }

    /// Returns a flat `[Option<Piece>; 64]` snapshot of the board.
    ///
    /// Index `n` corresponds to square `n` (a1 = 0 … h8 = 63). This is
    /// primarily used by code that needs to iterate over all pieces without
    /// working directly with bitboards.
    pub fn squares(&self) -> [Option<Piece>; 64] {
        let mut arr = [None; 64];
        for sq in 0u8..64 {
            arr[sq as usize] = self.piece_at(sq);
        }
        arr
    }

    /// Square of the king of `color`, found in O(1) via `trailing_zeros`.
    ///
    /// Returns `None` only in an illegal position where the king is missing.
    #[inline]
    pub fn king_square(&self, color: Color) -> Option<Square> {
        let bb = self.bb[color.index()][PieceType::King.index()];
        if bb == 0 {
            None
        } else {
            Some(bb.trailing_zeros() as Square)
        }
    }

    /// Clears all bitboards at `sq`.
    ///
    /// We scan all 12 BBs unconditionally rather than calling `piece_at` first —
    /// both approaches are O(12), but the branchless AND-mask loop avoids the
    /// early-return overhead and is friendlier to the branch predictor on the
    /// hot path (e.g. inside `set_piece` during FEN parsing or move application).
    #[inline]
    pub(crate) fn clear_square(&mut self, sq: Square) {
        let mask = !(1u64 << sq);
        for ci in 0..2usize {
            for pi in 0..6usize {
                self.bb[ci][pi] &= mask;
            }
        }
    }

    /// Returns a board set up in the standard chess starting position.
    pub fn starting_position() -> Self {
        let mut b = Self {
            bb: [[0u64; 6]; 2],
            side_to_move: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };

        let w = Color::White.index();
        let k = Color::Black.index();

        b.bb[w][PieceType::Pawn.index()] = 0x000000000000FF00; // rank 2
        b.bb[w][PieceType::Knight.index()] = 0x0000000000000042; // b1, g1
        b.bb[w][PieceType::Bishop.index()] = 0x0000000000000024; // c1, f1
        b.bb[w][PieceType::Rook.index()] = 0x0000000000000081; // a1, h1
        b.bb[w][PieceType::Queen.index()] = 0x0000000000000008; // d1
        b.bb[w][PieceType::King.index()] = 0x0000000000000010; // e1

        b.bb[k][PieceType::Pawn.index()] = 0x00FF000000000000; // rank 7
        b.bb[k][PieceType::Knight.index()] = 0x4200000000000000; // b8, g8
        b.bb[k][PieceType::Bishop.index()] = 0x2400000000000000; // c8, f8
        b.bb[k][PieceType::Rook.index()] = 0x8100000000000000; // a8, h8
        b.bb[k][PieceType::Queen.index()] = 0x0800000000000000; // d8
        b.bb[k][PieceType::King.index()] = 0x1000000000000000; // e8

        b
    }

    /// Returns the piece-placement field of the FEN string (rank 8 … rank 1).
    pub fn fen_piece_placement(&self) -> String {
        let mut out = String::with_capacity(72);
        for rank in (0..8u8).rev() {
            let mut empty: u8 = 0;
            for file in 0..8u8 {
                match self.piece_at(rank * 8 + file) {
                    None => empty += 1,
                    Some(p) => {
                        if empty > 0 {
                            out.push((b'0' + empty) as char);
                            empty = 0;
                        }
                        out.push(p.to_fen_char());
                    }
                }
            }
            if empty > 0 {
                out.push((b'0' + empty) as char);
            }
            if rank > 0 {
                out.push('/');
            }
        }
        out
    }

    /// A compact position key for threefold-repetition detection.
    ///
    /// Includes piece placement, side to move, castling rights, and en passant
    /// target — matching the fields that define a unique chess position.
    pub fn position_key(&self) -> String {
        let ep = self
            .en_passant
            .map(square_name)
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{} {} {} {}",
            self.fen_piece_placement(),
            self.side_to_move.to_char(),
            self.castling_rights.to_fen_str(),
            ep
        )
    }
}
