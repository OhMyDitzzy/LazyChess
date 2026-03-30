use lazychess::{
    analyzer::{AnalyzerConfig, MoveAnalyzer},
    uci::UciEngine,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "stockfish".to_string());

    println!("Spawning engine: {engine_path}\n");

    let mut engine = UciEngine::with_options(&engine_path, &[("Threads", "4"), ("Hash", "128")])
        .unwrap_or_else(|e| {
            eprintln!("Failed to start engine: {e}");
            eprintln!("Make sure Stockfish (or another UCI engine) is installed.");
            eprintln!("Usage: cargo run --example analysis -- /path/to/engine");
            std::process::exit(1);
        });

    if let Some(name) = &engine.info.name {
        println!("Engine : {name}");
    }
    if let Some(author) = &engine.info.author {
        println!("Author : {author}");
    }
    println!();

    let config = AnalyzerConfig::depth(15).with_timeout(u64::MAX);
    let mut analyzer = MoveAnalyzer::new(&mut engine, config);
    
    // Blunder PGN :)
    let pgn = r#"[Event "Im_Wayzzy vs. maxvela07"]
[Site "Chess.com"]
[Date "2026-03-30"]
[White "Im_Wayzzy"]
[Black "maxvela07"]
[Result "0-1"]
[WhiteElo "297"]
[BlackElo "349"]
[TimeControl "180"]
[Termination "maxvela07 won by resignation"]
1. e4 e5 2. Nf3 Nf6 3. Nxe5 d6 4. Nc4 Nxe4 5. Nc3 Bf5 6. f3 Nxc3 7. dxc3 Qe7+ 8.
Be2 Nc6 9. b3 O-O-O 10. O-O Ne5 11. Ne3 Nc6 12. Bd2 d5 13. c4 dxc4 14. Nxc4 Qe6
15. Bg5 Rxd1 0-1
"#;

    let report = match analyzer
        .analyze_pgn(pgn)
        .on_progress(|current, total, mv| {
            println!("Analysing move {current}/{total}: {mv}");
        })
        .run_partial()
    {
        Ok(report) => report,
        Err(partial) => {
            eprintln!("\nEngine crashed: {}", partial.error);
            if partial.report.moves.is_empty() {
                return Err(partial.error.into());
            }
            eprintln!(
                "Showing partial results ({} moves analysed):\n",
                partial.report.moves.len()
            );
            partial.report
        }
    };

    for mv in &report.moves {
        println!("Played : {}", mv.san);

        if let Some(best) = mv.suggested_move() {
            if best != mv.played_move {
                println!("  Best was : {best}");
            }
        }

        if !mv.alternatives.is_empty() {
            let alts = mv.alternatives.join(", ");
            println!("  Alternatives : {alts}");
        }
    }

    println!("{}", report.to_table());
    println!("Annotated PGN: {}", report.to_pgn());

    Ok(())
}
