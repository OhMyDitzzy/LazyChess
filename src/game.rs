use std::collections::HashMap;

use crate::board::Board;
use crate::fen::{board_to_fen, parse_fen};
use crate::movegen::{apply_move, generate_legal_moves, is_in_check};
use crate::opening::{BUILTIN_OPENINGS_JSON, OpeningBook};
use crate::pgn::{move_to_san, moves_to_pgn, parse_pgn, pgn_moves_to_uci};
use crate::types::*;

/// The `Game` struct is the primary entry point for LazyChess.
///
/// It owns the full game state: current board, history for undo / PGN export,
/// position-repetition counters, and an optional opening book.
///
/// # Example
/// ```rust
/// use lazychess::Game;
///
/// let mut game = Game::new();
/// game.do_move("e2e4").unwrap();
/// game.do_move("e7e5").unwrap();
/// println!("{}", game.display_board());
/// println!("{}", game.get_fen());
/// ```
pub struct Game {
    /// Current board state.
    board: Board,

    /// The FEN of the starting position (used as the PGN FEN tag when it is
    /// not the standard starting position).
    start_fen: String,

    /// Stack of `(board_before_move, move, san)` entries for undo and PGN export.
    history: Vec<(Board, Move, String)>,

    /// Position-repetition counter keyed on the canonical position key.
    position_counts: HashMap<String, u32>,

    /// Optional opening book.
    opening_book: OpeningBook,

    /// The name of the opening detected at game start (may be `None`).
    opening_name: Option<String>,
}

impl Game {
    /// Starts a new game from the standard starting position.
    pub fn new() -> Self {
        let board = Board::starting_position();
        Self::from_board(board)
    }

    /// Starts a new game from a FEN string.
    pub fn from_fen(fen: &str) -> ChessResult<Self> {
        let board = parse_fen(fen)?;
        Ok(Self::from_board(board))
    }

    fn from_board(board: Board) -> Self {
        let start_fen = board_to_fen(&board);
        let mut position_counts = HashMap::new();
        position_counts.insert(board.position_key(), 1);

        let opening_book =
            OpeningBook::from_json(BUILTIN_OPENINGS_JSON).unwrap_or_else(|_| OpeningBook::empty());

        let opening_name = opening_book
            .lookup(&board.fen_piece_placement())
            .map(str::to_owned);

        Self {
            board,
            start_fen,
            history: Vec::new(),
            position_counts,
            opening_book,
            opening_name,
        }
    }

    /// Replaces the built-in opening book with one loaded from a JSON string.
    ///
    /// See [`OpeningBook::from_json`] for the expected format.
    pub fn load_opening_book(&mut self, json: &str) -> ChessResult<()> {
        self.opening_book = OpeningBook::from_json(json)
            .map_err(|e| ChessError::new(format!("Failed to load opening book: {e}")))?;
        // Re-detect the opening for the current position.
        self.opening_name = self
            .opening_book
            .lookup(&self.board.fen_piece_placement())
            .map(str::to_owned);
        Ok(())
    }

    /// Returns the detected opening name for the current position, if known.
    pub fn opening_name(&self) -> Option<&str> {
        // Update lazily on each call so transpositions are detected mid-game.
        self.opening_book
            .lookup(&self.board.fen_piece_placement())
            .or(self.opening_name.as_deref())
    }

    /// Applies a move supplied in UCI (`"e2e4"`) or coordinate notation.
    ///
    /// Returns `Ok(())` if the move was legal and was executed, or a
    /// [`ChessError`] if the move is illegal or the string cannot be parsed.
    pub fn do_move(&mut self, mv_str: &str) -> ChessResult<()> {
        let mv = self.parse_move(mv_str)?;
        let san = move_to_san(&self.board, &mv);
        self.history.push((self.board.clone(), mv.clone(), san));

        self.board = apply_move(&self.board, &mv);

        let key = self.board.position_key();
        *self.position_counts.entry(key).or_insert(0) += 1;

        Ok(())
    }

    /// Undoes the last move and restores the board to the previous state.
    ///
    /// Returns `Ok(())` on success, or a [`ChessError`] if there are no moves
    /// to undo.
    pub fn undo_move(&mut self) -> ChessResult<()> {
        let (prev_board, _, _) = self
            .history
            .pop()
            .ok_or_else(|| ChessError::new("No moves to undo"))?;

        // Decrement the repetition counter for the board we are leaving.
        let key = self.board.position_key();
        if let Some(count) = self.position_counts.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.position_counts.remove(&key);
            }
        }

        self.board = prev_board;
        Ok(())
    }

    /// Returns `true` if the move string is legal in the current position.
    ///
    /// Does **not** modify the game state.
    pub fn is_move_legal(&self, mv_str: &str) -> bool {
        self.parse_move(mv_str).is_ok()
    }

    /// Returns all legal moves for the current position in UCI notation.
    pub fn get_legal_moves(&self) -> Vec<String> {
        generate_legal_moves(&self.board)
            .into_iter()
            .map(|m| m.to_uci())
            .collect()
    }

    /// Returns the current [`GameStatus`].
    pub fn get_game_status(&self) -> GameStatus {
        let legal_moves = generate_legal_moves(&self.board);

        if legal_moves.is_empty() {
            if is_in_check(&self.board, self.board.side_to_move) {
                return GameStatus::Checkmate;
            } else {
                return GameStatus::Stalemate;
            }
        }

        if self.is_draw_by_fifty_moves() {
            return GameStatus::Draw(DrawReason::FiftyMoveRule);
        }
        if self.is_draw_by_repetition() {
            return GameStatus::Draw(DrawReason::ThreefoldRepetition);
        }
        if self.is_draw_by_insufficient_material() {
            return GameStatus::Draw(DrawReason::InsufficientMaterial);
        }

        if is_in_check(&self.board, self.board.side_to_move) {
            return GameStatus::Check;
        }

        GameStatus::Ongoing
    }

    /// Returns a human-readable status string.
    pub fn get_game_status_str(&self) -> String {
        match self.get_game_status() {
            GameStatus::Ongoing => "ongoing".to_string(),
            GameStatus::Check => "check".to_string(),
            GameStatus::Checkmate => "checkmate".to_string(),
            GameStatus::Stalemate => "stalemate".to_string(),
            GameStatus::Draw(DrawReason::FiftyMoveRule) => "draw (50-move rule)".to_string(),
            GameStatus::Draw(DrawReason::ThreefoldRepetition) => {
                "draw (threefold repetition)".to_string()
            }
            GameStatus::Draw(DrawReason::InsufficientMaterial) => {
                "draw (insufficient material)".to_string()
            }
        }
    }

    pub fn is_checkmate(&self) -> bool {
        matches!(self.get_game_status(), GameStatus::Checkmate)
    }

    pub fn is_stalemate(&self) -> bool {
        matches!(self.get_game_status(), GameStatus::Stalemate)
    }

    pub fn is_check(&self) -> bool {
        is_in_check(&self.board, self.board.side_to_move)
    }

    pub fn is_draw(&self) -> bool {
        matches!(self.get_game_status(), GameStatus::Draw(_))
    }

    fn is_draw_by_fifty_moves(&self) -> bool {
        self.board.halfmove_clock >= 100 // 100 half-moves = 50 full moves
    }

    fn is_draw_by_repetition(&self) -> bool {
        self.position_counts.values().any(|&count| count >= 3)
    }

    /// Detects insufficient mating material per FIDE rules:
    /// - K vs K
    /// - K+B vs K
    /// - K+N vs K
    /// - K+B vs K+B (same colour bishops)
    fn is_draw_by_insufficient_material(&self) -> bool {
        let mut white_pieces: Vec<PieceType> = Vec::new();
        let mut black_pieces: Vec<PieceType> = Vec::new();
        let mut white_bishop_sq: Option<Square> = None;
        let mut black_bishop_sq: Option<Square> = None;

        for sq in 0u8..64 {
            if let Some(p) = self.board.piece_at(sq) {
                match p.color {
                    Color::White => {
                        white_pieces.push(p.piece_type);
                        if p.piece_type == PieceType::Bishop {
                            white_bishop_sq = Some(sq);
                        }
                    }
                    Color::Black => {
                        black_pieces.push(p.piece_type);
                        if p.piece_type == PieceType::Bishop {
                            black_bishop_sq = Some(sq);
                        }
                    }
                }
            }
        }

        // Any queens, rooks, or pawns means there is sufficient mating material.
        let has_heavy = |pieces: &[PieceType]| {
            pieces
                .iter()
                .any(|&pt| matches!(pt, PieceType::Queen | PieceType::Rook | PieceType::Pawn))
        };
        if has_heavy(&white_pieces) || has_heavy(&black_pieces) {
            return false;
        }

        let minor_count = |pieces: &[PieceType]| {
            pieces
                .iter()
                .filter(|&&pt| matches!(pt, PieceType::Knight | PieceType::Bishop))
                .count()
        };

        let wm = minor_count(&white_pieces);
        let bm = minor_count(&black_pieces);

        // K vs K
        if wm == 0 && bm == 0 {
            return true;
        }

        // K+minor vs K
        if (wm == 1 && bm == 0) || (wm == 0 && bm == 1) {
            return true;
        }

        // K+B vs K+B (same colour bishops)
        if wm == 1
            && bm == 1
            && white_pieces.contains(&PieceType::Bishop)
            && black_pieces.contains(&PieceType::Bishop)
            && let (Some(wsq), Some(bsq)) = (white_bishop_sq, black_bishop_sq)
        {
            // Squares of the same colour share the parity of (rank + file).
            let w_parity = (rank_of(wsq) + file_of(wsq)) % 2;
            let b_parity = (rank_of(bsq) + file_of(bsq)) % 2;
            if w_parity == b_parity {
                return true;
            }
        }

        false
    }

    /// Returns the current board state as a FEN string.
    pub fn get_fen(&self) -> String {
        board_to_fen(&self.board)
    }

    /// Returns the full game history as a PGN string.
    pub fn get_pgn(&self) -> String {
        self.get_pgn_with_tags(&[])
    }

    /// Returns a PGN string with additional custom tag pairs.
    pub fn get_pgn_with_tags(&self, extra_tags: &[(&str, &str)]) -> String {
        let result = match self.get_game_status() {
            GameStatus::Checkmate => {
                if self.board.side_to_move == Color::White {
                    "0-1"
                } else {
                    "1-0"
                }
            }
            GameStatus::Draw(_) | GameStatus::Stalemate => "1/2-1/2",
            _ => "*",
        };

        let is_standard_start = self.start_fen == board_to_fen(&Board::starting_position());

        let mut tags: Vec<(&str, &str)> = vec![
            ("Event", "?"),
            ("Site", "?"),
            ("Date", "????.??.??"),
            ("Round", "?"),
            ("White", "?"),
            ("Black", "?"),
            ("Result", result),
        ];

        if !is_standard_start {
            tags.push(("SetUp", "1"));
            tags.push(("FEN", &self.start_fen));
        }

        for &(k, v) in extra_tags {
            tags.push((k, v));
        }

        let start_board = parse_fen(&self.start_fen).unwrap_or_else(|_| Board::starting_position());

        let move_history: Vec<(Move, String)> = self
            .history
            .iter()
            .map(|(_, mv, san)| (mv.clone(), san.clone()))
            .collect();

        moves_to_pgn(&start_board, &move_history, &tags)
    }

    /// Loads a PGN string and replays all moves.
    pub fn load_pgn(&mut self, pgn: &str) -> ChessResult<()> {
        let (_, san_moves) =
            parse_pgn(pgn).map_err(|e| ChessError::new(format!("PGN parse error: {e}")))?;

        let start_fen = crate::fen::board_to_fen(&Board::starting_position());
        let uci_moves = pgn_moves_to_uci(&start_fen, &san_moves)?;

        *self = Game::new();
        for mv in &uci_moves {
            self.do_move(mv)?;
        }
        Ok(())
    }

    /// Returns an 8×8 array of `Option<Piece>` representing the current board.
    /// Index `[rank][file]`, rank 0 = rank 1 (White's back rank).
    pub fn board(&self) -> [[Option<Piece>; 8]; 8] {
        let mut arr = [[None; 8]; 8];
        for (rank, row) in arr.iter_mut().enumerate() {
            for (file, square) in row.iter_mut().enumerate() {
                *square = self.board.squares[rank * 8 + file];
            }
        }
        arr
    }

    /// Returns an ASCII/Unicode representation of the board suitable for
    /// terminal output.
    pub fn display_board(&self) -> String {
        let mut s = String::with_capacity(320);
        s.push_str("   +------------------------+\n");
        for rank in (0..8u8).rev() {
            s.push_str(&format!(" {} |", rank + 1));
            for file in 0..8u8 {
                let sq = rank * 8 + file;
                let ch = match self.board.squares[sq as usize] {
                    Some(p) => p.to_fen_char(),
                    None => '.',
                };
                s.push_str(&format!(" {} ", ch));
            }
            s.push_str("|\n");
        }
        s.push_str("   +------------------------+\n");
        s.push_str("     a  b  c  d  e  f  g  h\n");
        s
    }

    /// Returns all past moves as a slice of `(board_before, move, san)`.
    pub fn history(&self) -> &[(Board, Move, String)] {
        &self.history
    }

    /// Returns a reference to the current board.
    pub fn current_board(&self) -> &Board {
        &self.board
    }

    /// Returns the colour of the square (`"light"` or `"dark"`).
    pub fn square_color(sq: &str) -> ChessResult<&'static str> {
        let s =
            parse_square(sq).ok_or_else(|| ChessError::new(format!("Invalid square: '{sq}'")))?;
        let color = if (rank_of(s) + file_of(s)).is_multiple_of(2) {
            "dark"
        } else {
            "light"
        };
        Ok(color)
    }

    /// Returns all legal moves for the current position as `Move` objects.
    pub fn moves(&self) -> Vec<Move> {
        generate_legal_moves(&self.board)
    }

    /// Returns the side whose turn it is to move.
    pub fn side_to_move(&self) -> Color {
        self.board.side_to_move
    }

    /// Parses a move string (UCI or SAN) into a [`Move`] and validates it
    /// against the current legal move list.
    fn parse_move(&self, mv_str: &str) -> ChessResult<Move> {
        let legal = generate_legal_moves(&self.board);

        // Try UCI format first (e.g. "e2e4", "e7e8q").
        if let Some(mv) = try_parse_uci(mv_str, &legal) {
            return Ok(mv);
        }

        // Try SAN format (e.g. "Nf3", "O-O").
        for mv in &legal {
            let san = move_to_san(&self.board, mv);
            let san_clean: &str = san.trim_end_matches(['!', '?', '+', '#']);
            let input_clean: &str = mv_str.trim_end_matches(['!', '?', '+', '#']);
            if san_clean == input_clean {
                return Ok(mv.clone());
            }
        }

        Err(ChessError::new(format!(
            "Illegal or unrecognised move: '{mv_str}'"
        )))
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

/// Tries to match a UCI move string against the list of legal moves.
fn try_parse_uci(mv_str: &str, legal: &[Move]) -> Option<Move> {
    if mv_str.len() < 4 {
        return None;
    }
    let from = parse_square(&mv_str[0..2])?;
    let to = parse_square(&mv_str[2..4])?;
    let promo = mv_str.chars().nth(4).and_then(PieceType::from_char);

    legal
        .iter()
        .find(|m| {
            m.from == from
                && m.to == to
                && match (&m.flag, promo) {
                    (MoveFlag::Promotion(pt), Some(p)) => *pt == p,
                    (MoveFlag::Promotion(_), None) => false,
                    _ => promo.is_none(),
                }
        })
        .cloned()
}
