/// Game-status detection: check, checkmate, stalemate, and draws.
use lazychess::{Game, GameStatus, DrawReason};

fn print_status(label: &str, game: &Game) {
    println!("\n*** {label} ***");
    println!("{}", game.display_board());
    println!("Status      : {}", game.get_game_status_str());
    println!("In check    : {}", game.is_check());
    println!("Checkmate   : {}", game.is_checkmate());
    println!("Stalemate   : {}", game.is_stalemate());
    println!("Draw        : {}", game.is_draw());
}

fn main() {
    print_status("Starting Position", &Game::new());
    
    // White rook on e2 gives check to black king on e8.
    let game = Game::from_fen("4k3/8/8/8/8/8/4R3/4K3 b - - 0 1").unwrap();
    print_status("Check (Black king on e8, White rook on e2)", &game);

    let mut game = Game::new();
    for mv in &["f2f3", "e7e5", "g2g4", "d8h4"] {
        game.do_move(mv).unwrap();
    }
    print_status("Fool's Mate", &game);

    // Black king on a8, White queen on c7, White king on b6.
    let game = Game::from_fen("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1").unwrap();
    print_status("Stalemate", &game);

    // Half-move clock already at 99; one more quiet king move triggers it.
    let mut game = Game::from_fen("8/8/8/8/8/8/8/K6k w - - 99 1").unwrap();
    game.do_move("a1b1").unwrap();
    print_status("50-Move Rule", &game);
    assert!(matches!(game.get_game_status(), GameStatus::Draw(DrawReason::FiftyMoveRule)));

    let game = Game::from_fen("8/8/8/8/8/8/8/K6k w - - 0 1").unwrap();
    print_status("Insufficient Material (K vs K)", &game);

    let game = Game::from_fen("8/8/8/8/8/8/8/KB5k w - - 0 1").unwrap();
    print_status("Insufficient Material (K+B vs K)", &game);

    let mut game = Game::new();
    for _ in 0..3 {
        game.do_move("g1f3").unwrap();
        game.do_move("g8f6").unwrap();
        game.do_move("f3g1").unwrap();
        game.do_move("f6g8").unwrap();
    }
    print_status("Threefold Repetition", &game);
    assert!(matches!(game.get_game_status(), GameStatus::Draw(DrawReason::ThreefoldRepetition)));
}
