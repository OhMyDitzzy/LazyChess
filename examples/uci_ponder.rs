//! # Example: UCI Ponder – Engine-Suggested Ponder Move
//!
//! Mirrors the python-chess pondering workflow:
//!
//! ```python
//! result = await engine.play(board, limit, ponder=True)
//! # result.move   -> engine's chosen move
//! # result.ponder -> engine's suggested ponder move (opponent's expected reply)
//! ```
//!
//! In lazychess the equivalent is:
//!
//! ```rust,no_run
//! let result = engine.play(&game, &config)?;
//! // result.best_move   -> engine's chosen move
//! // result.ponder_move -> engine's suggested ponder move (if any)
//! ```
//!
//! The key difference from the basic ponder example: we no longer hardcode
//! which move to ponder. The engine itself tells us via the
//! `bestmove e2e4 ponder e7e5` response, and we use that suggestion directly.
//!
//! Run with:
//!   cargo run --example uci_ponder_auto -- /path/to/stockfish
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

    println!("=== LazyChess – Auto Ponder Example (python-chess style) ===\n");
    println!("Spawning engine: {engine_path}\n");

    let mut engine = UciEngine::with_options(
        &engine_path,
        &[
            ("Threads", "2"),
            ("Hash",    "64"),
            ("Ponder",  "true"),
        ],
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to start engine: {e}");
        eprintln!("Make sure Stockfish (or another UCI engine) is installed.");
        eprintln!("Usage: cargo run --example uci_ponder_auto -- /path/to/engine");
        std::process::exit(1);
    });

    if let Some(name) = &engine.info.name {
        println!("Engine : {name}");
    }
    if let Some(author) = &engine.info.author {
        println!("Author : {author}");
    }
    println!();

    engine.new_game().expect("ucinewgame failed");

    let mut game = Game::new();
    println!("*** Starting Position ***");
    println!("{}", game.display_board());
    println!("FEN : {}\n", game.get_fen());

    let config = SearchConfig::depth(14);

    // Ask the engine for its move. play() returns a PlayResult containing both
    // the chosen move and the engine's own ponder suggestion extracted from the
    // `bestmove <move> ponder <ponder>` response. We never hardcode which move
    // to ponder — the engine decides based on its own analysis.
    println!("--- Step 1: Engine chooses its move (depth 14) ---");

    let result = engine
        .play(&game, &config)
        .expect("play() failed");

    println!("Engine plays : {}", result.best_move);
    match &result.ponder_move {
        Some(pm) => println!("Ponder hint  : {pm}  (suggested by the engine)"),
        None     => println!("Ponder hint  : none"),
    }
    println!();

    game.do_move(&result.best_move).expect("engine move should be legal");
    println!("{}", game.display_board());
    println!("FEN : {}\n", game.get_fen());

    // If the engine provided a ponder suggestion, start background thinking
    // immediately while the opponent is deciding. sync_game sets the position
    // after the engine's move; ponder() appends the expected opponent reply and
    // sends "go ponder depth 14" so the engine searches with the same depth
    // limit used during the main search — this ensures it stops on its own
    // and ponderhit() returns without needing to send a separate stop first.
    if let Some(ref ponder_mv) = result.ponder_move {
        println!("--- Step 2: Engine ponders '{ponder_mv}' in the background ---");
        engine.sync_game(&game).expect("sync_game failed");
        engine.ponder(ponder_mv, &config).expect("ponder failed");
        println!("is_pondering = {}\n", engine.is_pondering());
    } else {
        println!("(No ponder suggestion from engine, skipping background search)\n");
    }

    println!("(Simulating opponent think time – 400 ms ...)");
    std::thread::sleep(std::time::Duration::from_millis(400));

    // Scenario A: the opponent plays exactly the move the engine was pondering.
    // ponderhit() sends "stop" + "ponderhit" and waits for bestmove. It returns
    // a PlayResult so we immediately have both the reply move and the engine's
    // ponder suggestion for the *next* turn — no extra round-trip needed.
    if let Some(ref ponder_mv) = result.ponder_move {
        println!("\n=== Scenario A: Ponder HIT (opponent played '{ponder_mv}') ===\n");

        let hit_result = engine.ponderhit().expect("ponderhit failed");
        println!("Engine reply (warm hash) : {}", hit_result.best_move);
        match &hit_result.ponder_move {
            Some(pm) => println!("Next ponder hint         : {pm}"),
            None     => println!("Next ponder hint         : none"),
        }
        println!("is_pondering             = {}\n", engine.is_pondering());

        game.do_move(ponder_mv).expect("ponder move should be legal");
        game.do_move(&hit_result.best_move).expect("reply should be legal");

        println!("{}", game.display_board());
        println!("FEN    : {}", game.get_fen());
        println!("Status : {}\n", game.get_game_status_str());

        game.undo_move().expect("undo reply");
        game.undo_move().expect("undo ponder move");
    }

    // Scenario B: the opponent plays a different move than the engine expected.
    // ponder_miss() sends "stop" and drains the bestmove response, returning
    // the engine to an idle state. We then apply the actual opponent move and
    // call play() again for a fresh search. Even a miss is not entirely wasted:
    // the engine's hash table retains useful entries from the background search,
    // so the fresh search is still faster than a completely cold start.
    engine.new_game().expect("ucinewgame failed");

    let mut game2 = Game::new();
    game2.do_move(&result.best_move).expect("replay engine move");

    let actual_move = if result.ponder_move.as_deref() == Some("e7e5") {
        "c7c5"
    } else {
        "e7e5"
    };

    println!("=== Scenario B: Ponder MISS (opponent played '{actual_move}') ===\n");

    if let Some(ref ponder_mv) = result.ponder_move {
        engine.sync_game(&game2).expect("sync_game failed");
        engine.ponder(ponder_mv, &config).expect("ponder failed");
        println!("Pondering '{ponder_mv}' ...");

        std::thread::sleep(std::time::Duration::from_millis(300));

        engine.ponder_miss().expect("ponder_miss failed");
        println!("Ponder miss! Opponent played '{actual_move}' instead of '{ponder_mv}'.");
        println!("is_pondering = {}", engine.is_pondering());
    }

    game2.do_move(actual_move).expect("actual move should be legal");

    println!("\n--- Fresh search on the real position (depth 14) ---");

    // play() is used again here so we automatically capture the engine's next
    // ponder suggestion for the following round of the game loop.
    let result2 = engine
        .play(&game2, &config)
        .expect("play() fresh search failed");

    println!("Engine reply   : {}", result2.best_move);
    match &result2.ponder_move {
        Some(pm) => println!("Next ponder    : {pm}"),
        None     => println!("Next ponder    : none"),
    }
    println!();

    game2.do_move(&result2.best_move).expect("fresh reply should be legal");

    println!("{}", game2.display_board());
    println!("FEN    : {}", game2.get_fen());
    println!("Status : {}\n", game2.get_game_status_str());

    engine.quit().expect("quit failed");
    println!("Engine shut down. Done!");
}
