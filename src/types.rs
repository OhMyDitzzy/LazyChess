use std::fmt;

/// A board square represented as a linear index in the range `[0, 63]`.
/// Index 0 = a1, index 7 = h1, index 56 = a8, index 63 = h8.
pub type Square = u8;

#[inline(always)]
pub fn file_of(sq: Square) -> u8 {
    sq & 7
}

#[inline(always)]
pub fn rank_of(sq: Square) -> u8 {
    sq >> 3
}

#[inline(always)]
pub fn make_square(file: u8, rank: u8) -> Square {
    rank * 8 + file
}

/// Returns the algebraic name of a square (e.g. `"e4"`).
pub fn square_name(sq: Square) -> String {
    format!("{}{}", (b'a' + file_of(sq)) as char, rank_of(sq) + 1)
}

/// Parses an algebraic square name into an index. Returns `None` on failure.
pub fn parse_square(s: &str) -> Option<Square> {
    let b = s.as_bytes();
    if b.len() < 2 {
        return None;
    }
    let file = b[0].wrapping_sub(b'a');
    let rank = b[1].wrapping_sub(b'1');
    if file > 7 || rank > 7 {
        return None;
    }
    Some(make_square(file, rank))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[inline(always)]
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    /// Returns the uppercase FEN/SAN character for this piece type.
    pub fn to_char(self) -> char {
        match self {
            PieceType::Pawn => 'P',
            PieceType::Knight => 'N',
            PieceType::Bishop => 'B',
            PieceType::Rook => 'R',
            PieceType::Queen => 'Q',
            PieceType::King => 'K',
        }
    }

    /// Parses a FEN/SAN character (case-insensitive) into a piece type.
    pub fn from_char(c: char) -> Option<PieceType> {
        match c.to_ascii_uppercase() {
            'P' => Some(PieceType::Pawn),
            'N' => Some(PieceType::Knight),
            'B' => Some(PieceType::Bishop),
            'R' => Some(PieceType::Rook),
            'Q' => Some(PieceType::Queen),
            'K' => Some(PieceType::King),
            _ => None,
        }
    }

    /// Conventional centipawn value used for material counting.
    pub fn value(self) -> i32 {
        match self {
            PieceType::Pawn => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook => 500,
            PieceType::Queen => 900,
            PieceType::King => 20_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

impl Piece {
    #[inline(always)]
    pub fn new(piece_type: PieceType, color: Color) -> Self {
        Self { piece_type, color }
    }

    /// Returns the FEN character for this piece (uppercase = White, lowercase = Black).
    pub fn to_fen_char(self) -> char {
        let c = self.piece_type.to_char();
        if self.color == Color::White {
            c
        } else {
            c.to_ascii_lowercase()
        }
    }

    /// Parses a single FEN character into a `Piece`.
    pub fn from_fen_char(c: char) -> Option<Piece> {
        let color = if c.is_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        let piece_type = PieceType::from_char(c)?;
        Some(Piece::new(piece_type, color))
    }

    /// Returns the Unicode figurine for this piece.
    /// TODO: Remove this, Some terminals may not support unicode
    /// Use letters only
    pub fn unicode(self) -> char {
        match (self.color, self.piece_type) {
            (Color::White, PieceType::King) => '♔',
            (Color::White, PieceType::Queen) => '♕',
            (Color::White, PieceType::Rook) => '♖',
            (Color::White, PieceType::Bishop) => '♗',
            (Color::White, PieceType::Knight) => '♘',
            (Color::White, PieceType::Pawn) => '♙',
            (Color::Black, PieceType::King) => '♚',
            (Color::Black, PieceType::Queen) => '♛',
            (Color::Black, PieceType::Rook) => '♜',
            (Color::Black, PieceType::Bishop) => '♝',
            (Color::Black, PieceType::Knight) => '♞',
            (Color::Black, PieceType::Pawn) => '♟',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl CastlingRights {
    pub fn none() -> Self {
        Self {
            white_kingside: false,
            white_queenside: false,
            black_kingside: false,
            black_queenside: false,
        }
    }

    pub fn all() -> Self {
        Self {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    }

    /// Serialises castling rights as a FEN field (e.g. `"KQkq"` or `"-"`).
    pub fn to_fen_str(self) -> String {
        let mut s = String::with_capacity(4);
        if self.white_kingside {
            s.push('K');
        }
        if self.white_queenside {
            s.push('Q');
        }
        if self.black_kingside {
            s.push('k');
        }
        if self.black_queenside {
            s.push('q');
        }
        if s.is_empty() {
            s.push('-');
        }
        s
    }
}

/// Extra information encoded alongside the (from, to) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveFlag {
    Normal,
    DoublePawnPush,
    EnPassant,
    CastleKingside,
    CastleQueenside,
    Promotion(PieceType),
}

/// A single chess move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub flag: MoveFlag,
}

impl Move {
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            flag: MoveFlag::Normal,
        }
    }

    pub fn with_flag(from: Square, to: Square, flag: MoveFlag) -> Self {
        Self { from, to, flag }
    }

    /// Serialises the move in long algebraic (UCI) format (e.g. `"e2e4"`, `"e7e8q"`).
    pub fn to_uci(&self) -> String {
        let mut s = format!("{}{}", square_name(self.from), square_name(self.to));
        if let MoveFlag::Promotion(pt) = self.flag {
            s.push(pt.to_char().to_ascii_lowercase());
        }
        s
    }

    /// Returns the promotion piece type if this is a promotion move.
    pub fn promotion_piece(&self) -> Option<PieceType> {
        if let MoveFlag::Promotion(pt) = self.flag {
            Some(pt)
        } else {
            None
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Check,
    Checkmate,
    Stalemate,
    Draw(DrawReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawReason {
    FiftyMoveRule,
    ThreefoldRepetition,
    InsufficientMaterial,
}

#[derive(Debug, Clone)]
pub struct ChessError {
    pub message: String,
}

impl ChessError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChessError: {}", self.message)
    }
}

impl std::error::Error for ChessError {}

pub type ChessResult<T> = Result<T, ChessError>;
