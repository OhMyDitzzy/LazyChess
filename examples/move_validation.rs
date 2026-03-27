/// Move validation: legal-move enumeration, legality checks,
/// UCI vs SAN input, and square utilities.
use lazychess::{Game, parse_square};

fn main() {
    let mut game = Game::new();

    println!("*** Legal Moves (starting position) ***");
    let moves = game.get_legal_moves();
    println!("Total: {}", moves.len()); // always 20
    println!("Moves: {:?}", moves);

    println!("\n*** Legality Checks (UCI) ***");
    let tests = [
        ("e2e4", true),
        ("e2e5", false), // pawn can't jump two squares from empty board
        ("d1d4", false), // queen blocked by pawn
        ("a1a3", false), // rook blocked
        ("g1f3", true),  // knight jump
    ];
    for (mv, expected) in &tests {
        let legal = game.is_move_legal(mv);
        let mark  = if legal == *expected { "✓" } else { "✗" };
        println!("{mark} {mv:5} legal={legal}");
    }

    println!("\n*** Legality Checks (SAN) ***");
    let san_tests = [
        ("e4",  true),
        ("Nf3", true),
        ("Qd4", false), // queen blocked
        ("e5",  false), // wrong pawn move for white
    ];
    for (mv, expected) in &san_tests {
        let legal = game.is_move_legal(mv);
        let mark  = if legal == *expected { "✓" } else { "✗" };
        println!("{mark} {mv:5} legal={legal}");
    }

    println!("\n*** Legal Moves After 1.e4 e5 ***");
    game.do_move("e4").unwrap();
    game.do_move("e5").unwrap();
    let moves = game.get_legal_moves();
    println!("Total: {}", moves.len());
    println!("First 10: {:?}", &moves[..moves.len().min(10)]);
    
    println!("\n*** Square Utilities ***");
    for sq in &["a1", "h8", "e4", "d5"] {
        let idx   = parse_square(sq).unwrap();
        let color = Game::square_color(sq).unwrap();
        println!("{sq} → index={idx:2}, color={color}");
    }
    println!("invalid 'z9' → {:?}", parse_square("z9"));
}
