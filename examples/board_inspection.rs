/// Board inspection: accessing pieces by square, iterating the board array,
/// piece type / color checks, and pawn promotion.
use lazychess::{Game, Color, PieceType};

fn main() {
    println!("=== Board Array Inspection ===");
    let game = Game::new();
    let board = game.board(); // [[Option<Piece>; 8]; 8], rank 0 = rank 1

    // White back rank pieces (rank index 0).
    print!("White back rank: ");
    for file in 0..8 {
        if let Some(p) = board[0][file] {
            print!("{} ", p.to_fen_char());
        }
    }
    println!();

    // Black back rank pieces (rank index 7).
    print!("Black back rank: ");
    for file in 0..8 {
        if let Some(p) = board[7][file] {
            print!("{} ", p.to_fen_char());
        }
    }
    println!();

    println!("\n*** Piece Count (starting position) ***");
    let mut white = 0u32;
    let mut black = 0u32;
    for rank in 0..8 {
        for file in 0..8 {
            if let Some(p) = board[rank][file] {
                match p.color {
                    Color::White => white += 1,
                    Color::Black => black += 1,
                }
            }
        }
    }
    println!("White pieces: {white}"); // 16
    println!("Black pieces: {black}"); // 16

    println!("\n*** Specific Square Queries ***");
    // d1 = rank 0, file 3
    if let Some(q) = board[0][3] {
        println!("d1: {:?} {:?}", q.color, q.piece_type);
        assert_eq!(q.piece_type, PieceType::Queen);
        assert_eq!(q.color, Color::White);
    }

    // e1 = rank 0, file 4
    if let Some(k) = board[0][4] {
        println!("e1: {:?} {:?}", k.color, k.piece_type);
    }

    println!("\n*** Pawn Promotion ***");
    // White pawn on e7, both kings present.
    let fen = "8/4P3/8/8/8/8/8/4K1k1 w - - 0 1";
    let mut game = Game::from_fen(fen).unwrap();
    println!("Before: {}", game.get_fen());
    println!("{}", game.display_board());

    game.do_move("e7e8q").expect("promotion to queen must be legal");
    println!("After promoting to queen: {}", game.get_fen());
    println!("{}", game.display_board());

    // Verify the promoted piece.
    let board = game.board();
    // e8 = rank 7, file 4
    if let Some(p) = board[7][4] {
        println!("e8 now holds: {:?} {:?}", p.color, p.piece_type);
        assert_eq!(p.piece_type, PieceType::Queen);
    }

    println!("\n*** Side to Move ***");
    let mut game = Game::new();
    println!("Start      : {}", game.side_to_move()); // w
    game.do_move("e2e4").unwrap();
    println!("After e4   : {}", game.side_to_move()); // b
    game.do_move("e7e5").unwrap();
    println!("After e5   : {}", game.side_to_move()); // w
    game.undo_move().unwrap();
    println!("After undo : {}", game.side_to_move()); // b
}
