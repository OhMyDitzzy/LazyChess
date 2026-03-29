use lazychess::{apply_move, board_to_fen, generate_legal_moves, parse_fen, Board};
use std::time::Instant;

fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal_moves(board);

    if depth == 1 {
        return moves.len() as u64;
    }

    moves
        .iter()
        .map(|mv| perft(&apply_move(board, mv), depth - 1))
        .sum()
}

fn perft_divide(board: &Board, depth: u32) -> u64 {
    let moves = generate_legal_moves(board);
    let mut total = 0u64;

    for mv in &moves {
        let child = apply_move(board, mv);
        let nodes = perft(&child, depth - 1);
        println!("{}: {nodes}", mv.to_uci());
        total += nodes;
    }

    total
}

fn main() {
    let mut args = std::env::args().skip(1);

    let depth: u32 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let fen = args
        .next()
        .unwrap_or_else(|| "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string());

    let board = parse_fen(&fen).unwrap_or_else(|e| {
        eprintln!("Invalid FEN: {e}");
        std::process::exit(1);
    });

    println!("FEN   : {}", board_to_fen(&board));
    println!("Depth : {depth}");
    println!();

    let start = Instant::now();
    let nodes = perft_divide(&board, depth);
    let elapsed = start.elapsed();

    println!();
    println!("Nodes : {nodes}");
    println!("Time  : {:.3}s", elapsed.as_secs_f64());
    println!("NPS   : {:.0}", nodes as f64 / elapsed.as_secs_f64());
}
