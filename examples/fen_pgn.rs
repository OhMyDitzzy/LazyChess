/// FEN and PGN import / export with lazychess.
use lazychess::{Game, board_to_fen, parse_fen};

fn main() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let board = parse_fen(fen).expect("FEN must be valid");
    let back  = board_to_fen(&board);

    println!("*** FEN Round-trip ***");
    println!("Input : {fen}");
    println!("Output: {back}");
    println!("Match : {}", fen == back);

    println!("\n*** Game from FEN ***");
    let mid_game_fen = "rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/2N2N2/PP2PPPP/R1BQKB1R w KQkq - 0 5";
    let game = Game::from_fen(mid_game_fen).expect("FEN must be valid");
    println!("{}", game.display_board());
    println!("FEN: {}", game.get_fen());
    
    println!("*** PGN Export ***");
    let mut game = Game::new();
    for mv in &["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6"] {
        game.do_move(mv).unwrap();
    }
    println!("{}", game.get_pgn());

    println!("*** PGN Import ***");
    let pgn = r#"[Event "Immortal Game"]
[White "Anderssen"]
[Black "Kieseritzky"]
[Result "1-0"]

1. e4 e5 2. f4 exf4 3. Bc4 Qh4+ 4. Kf1 b5 5. Bxb5 Nf6 *"#;

    let mut game = Game::new();
    game.load_pgn(pgn).expect("PGN must load cleanly");
    println!("Replayed {} plies", game.history().len());
    println!("{}", game.display_board());
    println!("FEN: {}", game.get_fen());
}
