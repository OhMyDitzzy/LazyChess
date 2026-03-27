//! # Example: UCI Engine Communication
//!
//! Demonstrates how to use `UciEngine` together with `Game` in lazychess.
//!
//! Run with:
//!   cargo run --example uci_engine -- /path/to/stockfish
//!
//! If no path is given, the example tries "stockfish" from $PATH.

use lazychess::{
    uci::{SearchConfig, UciEngine},
    Game,
};

fn main() {
    let engine_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "stockfish".to_string());

    println!("=== LazyChess – UCI Engine Example ===\n");
    println!("Spawning engine: {engine_path}\n");

    let mut engine = UciEngine::with_options(
        &engine_path,
        &[
            ("Threads", "2"),   // use 2 CPU threads
            ("Hash",    "64"),  // 64 MB hash table
        ],
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to start engine: {e}");
        eprintln!("Make sure Stockfish (or another UCI engine) is installed.");
        eprintln!("Usage: cargo run --example uci_engine -- /path/to/engine");
        std::process::exit(1);
    });

    // Print engine identity from the UCI handshake.
    if let Some(name) = &engine.info.name {
        println!("Engine   : {name}");
    }
    if let Some(author) = &engine.info.author {
        println!("Author   : {author}");
    }
    println!("Options  : {} advertised\n", engine.info.options.len());

    engine.new_game().expect("ucinewgame failed");

    let mut game = Game::new();
    println!("*** Starting Position ***");
    println!("{}", game.display_board());
    println!("FEN : {}\n", game.get_fen());

    let opening = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"];
    for mv in &opening {
        game.do_move(mv).expect("move should be legal");
    }

    println!("*** After 1. e4 e5 2. Nf3 Nc6 3. Bb5 (Ruy López) ***");
    println!("{}", game.display_board());
    if let Some(name) = game.opening_name() {
        println!("Opening  : {name}");
    }
    println!("Status   : {}", game.get_game_status_str());
    println!("FEN      : {}\n", game.get_fen());

    println!("--- Best Move (depth 15) ---");
    let config_depth = SearchConfig::depth(15);

    let best = engine
        .best_move_for_game(&game, &config_depth)
        .expect("best_move failed");

    println!("Best move: {best}\n");

    println!("--- Analysis (depth 15) ---");
    let infos = engine
        .analyze_game(&game, &config_depth)
        .expect("analyze failed");

    // Print only the deepest info line for cleanliness.
    if let Some(deepest) = infos
        .iter()
        .filter(|i| i.multipv.unwrap_or(1) == 1)
        .max_by_key(|i| i.depth.unwrap_or(0))
    {
        println!("Depth    : {}", deepest.depth.unwrap_or(0));
        if let Some(ref score) = deepest.score {
            println!("Score    : {score}");
        }
        if !deepest.pv.is_empty() {
            println!("PV       : {}", deepest.pv.join(" "));
        }
        if let Some(nodes) = deepest.nodes {
            println!("Nodes    : {nodes}");
        }
        if let Some(nps) = deepest.nps {
            println!("NPS      : {nps}");
        }
    }
    println!();

    println!("--- Top 3 Moves (MultiPV, depth 12) ---");
    let config_mpv = SearchConfig::builder().depth(12).build();

    let top = engine
        .top_moves(&game, 3, &config_mpv)
        .expect("top_moves failed");

    for (i, (mv, score)) in top.iter().enumerate() {
        let score_str = score
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".into());
        println!("  {}. {mv}  ({})", i + 1, score_str);
    }
    println!();

    println!("--- Playing engine's best move: {best} ---");
    game.do_move(&best).expect("engine move should be legal");
    println!("{}", game.display_board());
    println!("Status   : {}", game.get_game_status_str());
    println!("FEN      : {}\n", game.get_fen());
    
    println!("--- Evaluating resulting position (movetime 500 ms) ---");
    let config_time = SearchConfig::movetime(500);

    let score = engine
        .evaluate(&game, &config_time)
        .expect("evaluate failed");

    match score {
        Some(s) => println!("Score    : {s}"),
        None    => println!("Score    : (not available)"),
    }
    println!();
    
    game.undo_move().expect("undo failed");
    println!("--- PGN after undoing engine move ---");
    println!("{}\n", game.get_pgn());

    // Graceful shutdown
    engine.quit().expect("quit failed");
    println!("Engine shut down. Done!");
}
