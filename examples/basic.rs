/// Basic usage of lazychess:
/// - Start a new game
/// - Make moves
/// - Display the board
/// - Undo moves
use lazychess::Game;

fn main() {
    let mut game = Game::new();
    println!("*** Starting Position ***");
    println!("{}", game.display_board());
    println!("Side to move : {}", game.side_to_move());
    println!("FEN          : {}", game.get_fen());

    let moves = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"];
    for mv in &moves {
        game.do_move(mv).expect("move should be legal");
    }

    println!("\n*** After 1. e4 e5 2. Nf3 Nc6 3. Bb5 ***");
    println!("{}", game.display_board());

    if let Some(name) = game.opening_name() {
        println!("Opening : {name}");
    }
    
    game.undo_move().unwrap();
    println!("\n*** After undoing Bb5 ***");
    println!("{}", game.display_board());
    println!("FEN : {}", game.get_fen());
}
