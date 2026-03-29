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
    
    // Partial run will capture the analysis results if the engine error occurs.
    // try to make it timeout on purpose
    let config = AnalyzerConfig::depth(15).with_timeout(10_000);
    let mut analyzer = MoveAnalyzer::new(&mut engine, config);

    let pgn = r#"[Event "Morphy's Opera Game: Morphy's Classic Opera Game"]
[Site "Paris"]
[Date "1858"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]
[Annotator "Collection"]
[Variant "Standard"]
[ECO "C41"]
[Opening "Philidor Defense"]
[StudyName "Morphy's Opera Game"]
[ChapterName "Morphy's Classic Opera Game"]
[ChapterURL "https://lichess.org/study/aVdQHxwx/jHDna5ZE"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
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

    println!("{}", report.to_table());
    println!("Annotated PGN: {}", report.to_pgn());

    Ok(())
}