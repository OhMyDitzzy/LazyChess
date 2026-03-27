use crate::board::Board;
use crate::movegen::{apply_move, generate_legal_moves, is_in_check};
use crate::types::*;
use crate::fen::parse_fen;

/// Converts a `Move` to Standard Algebraic Notation (SAN).
///
/// Disambiguation suffixes (file, rank, or both) are added only when required
/// to distinguish between two or more legal moves that land on the same square
/// with the same piece type.
pub fn move_to_san(board: &Board, mv: &Move) -> String {
    // Castling is written before disambiguation.
    if mv.flag == MoveFlag::CastleKingside {
        return "O-O".to_string();
    }
    if mv.flag == MoveFlag::CastleQueenside {
        return "O-O-O".to_string();
    }

    let piece = match board.piece_at(mv.from) {
        Some(p) => p,
        None => return mv.to_uci(), // Fallback; should never happen.
    };

    let mut san = String::with_capacity(8);

    if piece.piece_type != PieceType::Pawn {
        san.push(piece.piece_type.to_char());
    }

    // Disambiguation
    if piece.piece_type != PieceType::Pawn {
        let legal = generate_legal_moves(board);
        let ambiguous: Vec<&Move> = legal
            .iter()
            .filter(|m| {
                m.to == mv.to
                    && m.from != mv.from
                    && board
                        .piece_at(m.from)
                        .map(|p| p.piece_type == piece.piece_type)
                        .unwrap_or(false)
            })
            .collect();

        if !ambiguous.is_empty() {
            let same_file = ambiguous
                .iter()
                .any(|m| file_of(m.from) == file_of(mv.from));
            let same_rank = ambiguous
                .iter()
                .any(|m| rank_of(m.from) == rank_of(mv.from));

            if !same_file {
                san.push((b'a' + file_of(mv.from)) as char);
            } else if !same_rank {
                san.push((b'1' + rank_of(mv.from)) as char);
            } else {
                san.push((b'a' + file_of(mv.from)) as char);
                san.push((b'1' + rank_of(mv.from)) as char);
            }
        }
    }

    // Capture indicator
    let is_capture = board.piece_at(mv.to).is_some() || mv.flag == MoveFlag::EnPassant;
    if is_capture {
        if piece.piece_type == PieceType::Pawn {
            san.push((b'a' + file_of(mv.from)) as char);
        }
        san.push('x');
    }

    // Destination square
    san.push_str(&square_name(mv.to));

    // Promotion
    if let MoveFlag::Promotion(pt) = mv.flag {
        san.push('=');
        san.push(pt.to_char());
    }

    // Check / checkmate annotations
    let new_board = apply_move(board, mv);
    if is_in_check(&new_board, new_board.side_to_move) {
        let has_legal = !generate_legal_moves(&new_board).is_empty();
        san.push(if has_legal { '+' } else { '#' });
    }

    san
}

/// Generates a PGN string from a list of moves applied to a starting position.
///
/// `tags` is an ordered list of `(name, value)` pairs for the seven mandatory
/// PGN tag roster plus any extras the caller wishes to include.
pub fn moves_to_pgn(
    start_board: &Board,
    move_history: &[(Move, String)],
    tags: &[(&str, &str)],
) -> String {
    let mut pgn = String::with_capacity(512);

    for &(name, value) in tags {
        pgn.push_str(&format!("[{} \"{}\"]\n", name, value));
    }
    pgn.push('\n');

    let mut board = start_board.clone();
    let mut move_num = board.fullmove_number;
    let is_black_start = board.side_to_move == Color::Black;

    for (i, (mv, _san)) in move_history.iter().enumerate() {
        let is_white_turn = board.side_to_move == Color::White;

        if is_white_turn || i == 0 {
            if i == 0 && is_black_start {
                pgn.push_str(&format!("{}... ", move_num));
            } else if is_white_turn {
                pgn.push_str(&format!("{}. ", move_num));
            }
        }

        let san = move_to_san(&board, mv);
        pgn.push_str(&san);
        pgn.push(' ');

        board = apply_move(&board, mv);
        if board.side_to_move == Color::White {
            move_num += 1;
        }
    }

    // Determine the game result token.
    let result = if let Some(tag) = tags.iter().find(|(k, _)| *k == "Result") {
        tag.1.to_string()
    } else {
        "*".to_string()
    };
    pgn.push_str(&result);
    pgn
}

/// Parses a PGN string and returns the tag pairs and a list of UCI move strings.
pub fn parse_pgn(pgn: &str) -> ChessResult<(Vec<(String, String)>, Vec<String>)> {
    let mut tags = Vec::new();
    let mut moves = Vec::new();

    let mut in_tags = true;

    for line in pgn.lines() {
        let line = line.trim();
        if line.is_empty() {
            if in_tags {
                in_tags = false;
            }
            continue;
        }

        if line.starts_with('[') && in_tags {
            // e.g. [White "Kasparov"]
            if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let mut parts = inner.splitn(2, ' ');
                if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                    let val = val.trim_matches('"').to_string();
                    tags.push((key.to_string(), val));
                }
            }
        } else {
            in_tags = false;
            // Strip comments and annotations, then collect tokens.
            let stripped = strip_pgn_comments(line);
            for token in stripped.split_whitespace() {
                let token = token.trim_end_matches(['!', '?', '+', '#']);
                // Skip move numbers and result tokens.
                if token.ends_with('.') || token.is_empty() {
                    continue;
                }
                if matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*") {
                    continue;
                }
                // Convert SAN token to UCI via the move generator.
                moves.push(token.to_string());
            }
        }
    }

    Ok((tags, moves))
}

/// Replays PGN move tokens from a given starting FEN, returning the UCI move list.
///
/// SAN tokens are resolved against the legal move list at each ply.
pub fn pgn_moves_to_uci(start_fen: &str, san_moves: &[String]) -> ChessResult<Vec<String>> {
    let mut board = parse_fen(start_fen)?;
    let mut uci_moves = Vec::with_capacity(san_moves.len());

    for san in san_moves {
        let legal = generate_legal_moves(&board);
        let mv = legal
            .iter()
            .find(|m| move_to_san(&board, m).trim_end_matches(['!', '?', '+', '#']) == san.as_str())
            .ok_or_else(|| ChessError::new(format!("No legal move matches SAN '{san}'")))?
            .clone();

        uci_moves.push(mv.to_uci());
        board = apply_move(&board, &mv);
    }

    Ok(uci_moves)
}

/// Removes `{...}` and `;` line comments from a PGN line.
fn strip_pgn_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            ';' => break, // Rest of line is a comment.
            _ if depth == 0 => result.push(ch),
            _ => {}
        }
    }
    result
}
