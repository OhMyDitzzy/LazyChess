<div align="center">
  <img src="assets/lazychess_icon.png" width="150" alt="LazyChess">

  <h1>LazyChess</h1>

  <p>A fast, memory-efficient chess engine library for Rust.</p>

  [![Crates.io](https://img.shields.io/crates/v/lazychess)](https://crates.io/crates/lazychess)
  [![docs.rs](https://img.shields.io/docsrs/lazychess)](https://docs.rs/lazychess)
  [![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
  [![CI](https://github.com/OhMyDitzzy/LazyChess/actions/workflows/ci.yml/badge.svg)](https://github.com/OhMyDitzzy/LazyChess/actions/workflows/ci.yml)
</div>

---

LazyChess implements the full FIDE ruleset — castling, en passant, pawn promotion, and every draw condition — along with FEN/PGN serialisation, opening detection, and UCI engine communication.

> [!NOTE]
> LazyChess is still in its early stages of development. Your feedback and suggestions are very helpful for this library! Don't hesitate to visit the issues page if you have any problems with LazyChess!

## Installation

```toml
[dependencies]
lazychess = "0.1"
```

## Quick Start

```rust
use lazychess::Game;

fn main() {
    let mut game = Game::new();

    let moves = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"];
    for mv in &moves {
        game.do_move(mv).expect("move should be legal");
    }

    println!("{}", game.display_board());
    println!("Opening : {:?}", game.opening_name());
    println!("Status  : {}", game.get_game_status_str());
    println!("FEN     : {}", game.get_fen());
    println!("PGN     : {}", game.get_pgn());
}
```

Output:
```
   +------------------------+
 8 | r  .  b  q  k  b  n  r |
 7 | p  p  p  p  .  p  p  p |
 6 | .  .  n  .  .  .  .  . |
 5 | .  B  .  .  p  .  .  . |
 4 | .  .  .  .  P  .  .  . |
 3 | .  .  .  .  .  N  .  . |
 2 | P  P  P  P  .  P  P  P |
 1 | R  N  B  Q  K  .  .  R |
   +------------------------+
     a  b  c  d  e  f  g  h

Opening : Some("Ruy Lopez")
Status  : ongoing
FEN     : r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3
PGN     : 1. e4 e5 2. Nf3 Nc6 3. Bb5 *
```

## Features

- [x] **Full FIDE rules**: all piece types, castling, en passant, promotion
- [x] **Draw detection** — 50-move rule, threefold repetition, insufficient material
- [x] **Notation** — FEN & PGN import/export, SAN generation, UCI move format
- [x] **Opening book** — built-in ECO table, load your own `openings.json` at runtime
- [x] **UCI** — spawn and communicate with any UCI-compatible engine (Stockfish, etc.)
- [x] **Undo** — full move history stack

## Examples

More complete examples are available in the [`examples/`](examples/) folder:

| Example | Description |
|---|---|
| `basic` | New game, make moves, display board, undo |
| `board_inspection` | Access the board array and piece data |
| `fen_pgn` | FEN/PGN import and export |
| `game_status` | Checkmate, stalemate, and draw detection |
| `move_validation` | Legal move generation and validation |
| `uci_engine` | Connect to Stockfish, get best move, MultiPV analysis |

```bash
cargo run --example basic
cargo run --example uci_engine -- /path/to/stockfish
```

## License
MIT License

Copyright (c) 2026 Ditzzy LazyChess Authors
Copyright (c) 2026 LazyChess Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.