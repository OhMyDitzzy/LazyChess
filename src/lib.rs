//! # LazyChess
//!
//! A fast, memory-efficient chess engine library for Rust.
//!
//! LazyChess implements all standard FIDE rules, including castling, en passant,
//! pawn promotion, and every draw condition (fifty-move rule, threefold
//! repetition, insufficient material). It also supports FEN / PGN serialisation,
//! UCI communication, and an optional opening book.
//!
//! ## Quick start
//!
//! ```rust
//! use lazychess::Game;
//!
//! let mut game = Game::new();
//! game.do_move("e2e4").unwrap();
//! game.do_move("e7e5").unwrap();
//! game.do_move("g1f3").unwrap(); // Nf3
//!
//! println!("{}", game.display_board());
//! println!("Status : {}", game.get_game_status_str());
//! println!("FEN    : {}", game.get_fen());
//! println!("PGN    : {}", game.get_pgn());
//! ```

pub mod types;
pub mod board;
pub mod movegen;
pub mod fen;
pub mod pgn;
pub mod opening;
pub mod game;
pub mod uci;

// Re-export the most commonly used items at the crate root.
pub use types::{
    ChessError, ChessResult, Color, DrawReason, GameStatus, Move, MoveFlag, Piece, PieceType,
    Square, CastlingRights, file_of, rank_of, make_square, square_name, parse_square,
};
pub use board::Board;
pub use game::Game;
pub use movegen::{generate_legal_moves, is_in_check, is_square_attacked, apply_move};
pub use fen::{board_to_fen, parse_fen};
pub use pgn::{move_to_san, moves_to_pgn, parse_pgn};
pub use opening::OpeningBook;
