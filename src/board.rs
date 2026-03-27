use crate::types::*;

/// The complete state of the chess board at a single point in time.
///
/// The `squares` array uses the index scheme `rank * 8 + file` (a1 = 0, h8 = 63).
/// All move application logic lives in `movegen` to keep this struct simple.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Board {
    pub squares: [Option<Piece>; 64],
    pub side_to_move: Color,
    pub castling_rights: CastlingRights,
    /// Target square for a possible en passant capture; `None` if not applicable.
    pub en_passant: Option<Square>,
    /// Half-move clock for the 50-move rule (resets on pawn moves or captures).
    pub halfmove_clock: u32,
    /// Full-move number, starting at 1 and incrementing after Black's move.
    pub fullmove_number: u32,
}

impl Board {

    #[inline(always)]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq as usize]
    }

    #[inline(always)]
    pub fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.squares[sq as usize] = piece;
    }

    /// Removes and returns whatever piece occupies `sq` (may be `None`).
    #[inline(always)]
    pub fn take_piece(&mut self, sq: Square) -> Option<Piece> {
        self.squares[sq as usize].take()
    }

    /// Finds the square occupied by the king of `color`. Returns `None` only in
    /// an invalid position (no king on the board).
    pub fn king_square(&self, color: Color) -> Option<Square> {
        self.squares.iter().position(|p| {
            matches!(p, Some(Piece { piece_type: PieceType::King, color: c }) if *c == color)
        }).map(|i| i as Square)
    }

    pub fn starting_position() -> Self {
        let mut b = Self {
            squares: [None; 64],
            side_to_move: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };

        use PieceType::*;
        use Color::*;

        // White back rank (rank 1 = indices 0-7)
        let back = [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook];
        for (file, &pt) in back.iter().enumerate() {
            b.squares[file] = Some(Piece::new(pt, White));
            b.squares[8 + file] = Some(Piece::new(Pawn, White));
        }

        // Black back rank (rank 8 = indices 56-63)
        let back = [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook];
        for (file, &pt) in back.iter().enumerate() {
            b.squares[56 + file] = Some(Piece::new(pt, Black));
            b.squares[48 + file] = Some(Piece::new(Pawn, Black));
        }

        b
    }

    /// Returns the piece-placement field of the FEN string (rank 8 … rank 1).
    pub fn fen_piece_placement(&self) -> String {
        let mut out = String::with_capacity(72);
        for rank in (0..8u8).rev() {
            let mut empty: u8 = 0;
            for file in 0..8u8 {
                match self.squares[(rank * 8 + file) as usize] {
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

    /// A compact position key suitable for threefold-repetition detection.
    /// It encodes piece placement, side to move, castling rights, and en passant.
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
