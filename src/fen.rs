use crate::board::Board;
use crate::types::*;

/// Parses a FEN string into a `Board`.
///
/// Expects the standard six-field format:
/// `<placement> <color> <castling> <en_passant> <halfmove> <fullmove>`
///
/// The last two fields (halfmove clock and fullmove number) are optional and
/// default to `0` and `1` respectively if omitted.
///
/// Returns a [`ChessError`] if any field is malformed or a piece character is
/// unrecognised.
pub fn parse_fen(fen: &str) -> ChessResult<Board> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(ChessError::new("FEN must have at least 4 fields"));
    }

    // Side to move
    let side_to_move = match parts[1] {
        "w" => Color::White,
        "b" => Color::Black,
        s => return Err(ChessError::new(format!("Invalid side to move: '{s}'"))),
    };

    // Castling rights
    let mut castling_rights = CastlingRights::none();
    for ch in parts[2].chars() {
        match ch {
            'K' => castling_rights.white_kingside = true,
            'Q' => castling_rights.white_queenside = true,
            'k' => castling_rights.black_kingside = true,
            'q' => castling_rights.black_queenside = true,
            '-' => {}
            c => return Err(ChessError::new(format!("Invalid castling char: '{c}'"))),
        }
    }

    // En passant
    let en_passant = match parts[3] {
        "-" => None,
        s => Some(
            parse_square(s)
                .ok_or_else(|| ChessError::new(format!("Invalid en passant square: '{s}'")))?,
        ),
    };

    let halfmove_clock = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0u32);
    let fullmove_number = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1u32);

    // Build an empty board with all metadata set, then populate bitboards.
    let mut board = Board {
        bb: [[0u64; 6]; 2],
        side_to_move,
        castling_rights,
        en_passant,
        halfmove_clock,
        fullmove_number,
    };

    // Piece placement — FEN lists rank 8 first, rank 1 last.
    let mut rank: i32 = 7;
    let mut file: i32 = 0;
    for ch in parts[0].chars() {
        match ch {
            '/' => {
                rank -= 1;
                file = 0;
            }
            '1'..='8' => {
                file += ch as i32 - '0' as i32;
            }
            _ => {
                let piece = Piece::from_fen_char(ch)
                    .ok_or_else(|| ChessError::new(format!("Unknown FEN char: '{ch}'")))?;
                if rank < 0 || file > 7 {
                    return Err(ChessError::new("FEN piece placement out of bounds"));
                }
                board.set_piece((rank * 8 + file) as Square, Some(piece));
                file += 1;
            }
        }
    }

    Ok(board)
}

/// Serialises a `Board` to a complete FEN string.
pub fn board_to_fen(board: &Board) -> String {
    let ep = board
        .en_passant
        .map(square_name)
        .unwrap_or_else(|| "-".to_string());

    format!(
        "{} {} {} {} {} {}",
        board.fen_piece_placement(),
        board.side_to_move.to_char(),
        board.castling_rights.to_fen_str(),
        ep,
        board.halfmove_clock,
        board.fullmove_number,
    )
}
