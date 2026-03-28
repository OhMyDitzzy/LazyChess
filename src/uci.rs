//! # UCI Engine Communication
//!
//! This module provides [`UciEngine`], a thread-safe wrapper that spawns an
//! external UCI-compatible chess engine process (e.g. Stockfish) and
//! communicates with it over stdin/stdout.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use lazychess::{Game, uci::{UciEngine, SearchConfig}};
//!
//! let mut engine = UciEngine::new("/usr/bin/stockfish").unwrap();
//!
//! // Optional: tweak engine options before use.
//! engine.set_option("Threads", "4").unwrap();
//! engine.set_option("Hash", "128").unwrap();
//!
//! // Sync with a Game and ask for the best move.
//! let mut game = Game::new();
//! game.do_move("e2e4").unwrap();
//! game.do_move("e7e5").unwrap();
//!
//! engine.sync_game(&game).unwrap();
//!
//! let config = SearchConfig::depth(15);
//! let best = engine.best_move(&config).unwrap();
//! println!("Best move: {best}");
//!
//! // Ask for a full analysis with PV lines.
//! let infos = engine.analyze(&config).unwrap();
//! for info in &infos {
//!     println!("{info}");
//! }
//! ```

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::game::Game;
use crate::types::ChessError;
use crate::Board;

/// All errors that can occur while communicating with a UCI engine.
#[derive(Debug)]
pub enum UciError {
    /// The engine process could not be started.
    SpawnFailed(io::Error),
    /// Writing to the engine's stdin failed.
    WriteFailed(io::Error),
    /// The engine did not respond within the expected time.
    Timeout,
    /// The engine process terminated unexpectedly.
    ProcessDied,
    /// A response line could not be parsed.
    ParseError(String),
    /// The engine reported a specific error string.
    EngineError(String),
}

impl fmt::Display for UciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UciError::SpawnFailed(e) => write!(f, "Failed to spawn engine: {e}"),
            UciError::WriteFailed(e) => write!(f, "Failed to write to engine: {e}"),
            UciError::Timeout => write!(f, "Engine did not respond in time"),
            UciError::ProcessDied => write!(f, "Engine process terminated unexpectedly"),
            UciError::ParseError(s) => write!(f, "Parse error: {s}"),
            UciError::EngineError(s) => write!(f, "Engine error: {s}"),
        }
    }
}

impl std::error::Error for UciError {}

impl From<UciError> for ChessError {
    fn from(e: UciError) -> Self {
        ChessError::new(e.to_string())
    }
}

pub type UciResult<T> = Result<T, UciError>;

/// Evaluation score returned by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Score {
    /// Centipawn evaluation (positive = good for the side to move).
    Centipawns(i32),
    /// Forced mate in N half-moves (negative = being mated).
    Mate(i32),
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Score::Centipawns(cp) => write!(f, "{cp:+} cp"),
            Score::Mate(n) if *n > 0 => write!(f, "Mate in {n}"),
            Score::Mate(n) => write!(f, "Being mated in {}", n.abs()),
        }
    }
}

/// A single `info` line emitted by the engine during search.
#[derive(Debug, Clone)]
pub struct AnalysisInfo {
    /// Search depth reached.
    pub depth: Option<u32>,
    /// Selective search depth.
    pub seldepth: Option<u32>,
    /// Evaluation score.
    pub score: Option<Score>,
    /// Principal variation (list of UCI moves).
    pub pv: Vec<String>,
    /// Number of nodes searched.
    pub nodes: Option<u64>,
    /// Nodes per second.
    pub nps: Option<u64>,
    /// Time spent searching (milliseconds).
    pub time_ms: Option<u64>,
    /// Hash table usage in permille (0–1000).
    pub hashfull: Option<u32>,
    /// Multi-PV line index (1-based).
    pub multipv: Option<u32>,
    /// Free-text message from `info string …` lines (e.g. NNUE file name).
    pub message: Option<String>,
}

impl AnalysisInfo {
    fn empty() -> Self {
        Self {
            depth: None,
            seldepth: None,
            score: None,
            pv: Vec::new(),
            nodes: None,
            nps: None,
            time_ms: None,
            hashfull: None,
            multipv: None,
            message: None,
        }
    }
}

impl fmt::Display for AnalysisInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Pure text message lines (info string …) — print as-is.
        if let Some(ref msg) = self.message {
            return write!(f, "[engine] {msg}");
        }
        if let Some(mpv) = self.multipv {
            write!(f, "[pv {mpv}] ")?;
        }
        if let Some(d) = self.depth {
            write!(f, "depth {d} ")?;
        }
        if let Some(ref s) = self.score {
            write!(f, "score {s} ")?;
        }
        if !self.pv.is_empty() {
            write!(f, "pv {}", self.pv.join(" "))?;
        }
        Ok(())
    }
}

/// The result of a [`UciEngine::play`] call.
///
/// Contains the engine's chosen move and, if the engine provided one, a
/// suggestion for which opponent move to ponder on next. Pass `ponder_move`
/// directly to [`UciEngine::ponder`] to start background thinking.
///
/// # Example
/// ```rust,no_run
/// use lazychess::{Game, uci::{UciEngine, SearchConfig}};
///
/// let mut engine = UciEngine::new("/usr/bin/stockfish").unwrap();
/// let mut game = Game::new();
///
/// let config = SearchConfig::depth(15);
///
/// // Engine plays and suggests a ponder move.
/// let result = engine.play(&game, &config).unwrap();
/// game.do_move(&result.best_move).unwrap();
///
/// // Use the same config so the ponder search has the same depth limit.
/// if let Some(ref pm) = result.ponder_move {
///     engine.sync_game(&game).unwrap();
///     engine.ponder(pm, &config).unwrap(); // think while opponent decides
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PlayResult {
    /// The best move chosen by the engine (UCI notation, e.g. `"e2e4"`).
    pub best_move: String,
    /// The move the engine suggests pondering on next (opponent's expected
    /// reply). `None` if the engine did not provide a ponder suggestion.
    pub ponder_move: Option<String>,
}

/// Parameters that control the engine's search.
///
/// # Examples
///
/// ```rust
/// use lazychess::uci::SearchConfig;
///
/// // Search to depth 20
/// let cfg = SearchConfig::depth(20);
///
/// // Think for 2 seconds
/// let cfg = SearchConfig::movetime(2000);
///
/// // Full time-control
/// let cfg = SearchConfig::builder()
///     .wtime(60_000).btime(60_000)
///     .winc(1_000).binc(1_000)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct SearchConfig {
    /// Fixed search depth (plies).
    pub depth: Option<u32>,
    /// Fixed time per move (milliseconds).
    pub movetime: Option<u64>,
    /// White's remaining time (ms).
    pub wtime: Option<u64>,
    /// Black's remaining time (ms).
    pub btime: Option<u64>,
    /// White's increment per move (ms).
    pub winc: Option<u64>,
    /// Black's increment per move (ms).
    pub binc: Option<u64>,
    /// Restrict search to these moves (UCI notation).
    pub searchmoves: Vec<String>,
    /// Number of PV lines to return (default 1).
    pub multipv: Option<u32>,
    /// Search until [`UciEngine::stop`] is called.
    pub infinite: bool,
    /// Per-search read timeout (ms). Overrides the engine's global timeout
    /// for this search only. Useful for long searches (depth 25+) that would
    /// otherwise trip the default 10 s timeout.
    pub read_timeout_ms: Option<u64>,
    /// The move the engine should ponder on (opponent's expected reply).
    ///
    /// When set, the `go` command is sent as `go ponder …` and the engine
    /// searches the position after this move in the background. Call
    /// [`UciEngine::ponderhit`] if the opponent plays this move, or
    /// [`UciEngine::ponder_miss`] to abort and search from scratch.
    pub ponder_move: Option<String>,
}

impl SearchConfig {
    /// Search to a fixed depth.
    pub fn depth(d: u32) -> Self {
        Self { depth: Some(d), ..Default::default() }
    }

    /// Think for a fixed number of milliseconds.
    pub fn movetime(ms: u64) -> Self {
        Self { movetime: Some(ms), ..Default::default() }
    }

    /// Infinite search — call [`UciEngine::stop`] to retrieve the result.
    pub fn infinite() -> Self {
        Self { infinite: true, ..Default::default() }
    }

    /// Returns a [`SearchConfigBuilder`] for fluent construction.
    pub fn builder() -> SearchConfigBuilder {
        SearchConfigBuilder::default()
    }

    /// Serialises the config into a `go` command string (without the trailing
    /// newline).
    fn to_go_command(&self) -> String {
        let mut cmd = String::from("go");

        // Ponder mode: append "ponder" flag, then fall through to add any
        // time-control / depth limits so the engine knows when to stop after
        // a ponderhit (e.g. `go ponder depth 15` or `go ponder wtime 60000 …`).
        // Without at least one limit the search is infinite and ponderhit()
        // will never receive a bestmove response.
        if self.ponder_move.is_some() {
            cmd.push_str(" ponder");
            // Fall through — do NOT early-return here.
        }

        if self.infinite {
            cmd.push_str(" infinite");
            return cmd;
        }
        if let Some(d) = self.depth {
            cmd.push_str(&format!(" depth {d}"));
        }
        if let Some(ms) = self.movetime {
            cmd.push_str(&format!(" movetime {ms}"));
        }
        if let Some(t) = self.wtime {
            cmd.push_str(&format!(" wtime {t}"));
        }
        if let Some(t) = self.btime {
            cmd.push_str(&format!(" btime {t}"));
        }
        if let Some(i) = self.winc {
            cmd.push_str(&format!(" winc {i}"));
        }
        if let Some(i) = self.binc {
            cmd.push_str(&format!(" binc {i}"));
        }
        if !self.searchmoves.is_empty() {
            cmd.push_str(" searchmoves ");
            cmd.push_str(&self.searchmoves.join(" "));
        }
        cmd
    }
}

/// Builder for [`SearchConfig`].
#[derive(Debug, Clone, Default)]
pub struct SearchConfigBuilder(SearchConfig);

impl SearchConfigBuilder {
    pub fn depth(mut self, d: u32) -> Self { self.0.depth = Some(d); self }
    pub fn movetime(mut self, ms: u64) -> Self { self.0.movetime = Some(ms); self }
    pub fn wtime(mut self, ms: u64) -> Self { self.0.wtime = Some(ms); self }
    pub fn btime(mut self, ms: u64) -> Self { self.0.btime = Some(ms); self }
    pub fn winc(mut self, ms: u64) -> Self { self.0.winc = Some(ms); self }
    pub fn binc(mut self, ms: u64) -> Self { self.0.binc = Some(ms); self }
    pub fn multipv(mut self, n: u32) -> Self { self.0.multipv = Some(n); self }
    pub fn infinite(mut self) -> Self { self.0.infinite = true; self }
    pub fn searchmoves(mut self, moves: Vec<String>) -> Self {
        self.0.searchmoves = moves; self
    }
    /// Override the engine's global read timeout for this search only (ms).
    pub fn read_timeout_ms(mut self, ms: u64) -> Self {
        self.0.read_timeout_ms = Some(ms); self
    }
    /// Set the move to ponder on (opponent's expected reply in UCI notation).
    pub fn ponder_move(mut self, mv: impl Into<String>) -> Self {
        self.0.ponder_move = Some(mv.into()); self
    }
    pub fn build(self) -> SearchConfig { self.0 }
}

/// An option advertised by the engine during the UCI handshake.
#[derive(Debug, Clone)]
pub struct UciOption {
    pub name: String,
    pub kind: String,
    pub default: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub vars: Vec<String>,
}

/// Basic identity information returned by the engine.
#[derive(Debug, Clone, Default)]
pub struct EngineInfo {
    pub name: Option<String>,
    pub author: Option<String>,
    pub options: Vec<UciOption>,
}

/// Messages forwarded from the reader thread to the main thread.
#[derive(Debug)]
enum EngineMessage {
    Line(String),
    Eof,
}

fn spawn_reader(stdout: ChildStdout, tx: Sender<EngineMessage>) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(EngineMessage::Line(l)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(EngineMessage::Eof);
                    break;
                }
            }
        }
        let _ = tx.send(EngineMessage::Eof);
    });
}

/// A wrapper around an external UCI-compatible chess engine process.
///
/// ## Thread safety
///
/// `UciEngine` itself is **not** `Send`/`Sync` — it owns a child process and
/// should be used from a single thread. Wrap it in `Arc<Mutex<>>` if you need
/// to share it across threads.
pub struct UciEngine {
    /// The engine child process.
    child: Child,
    /// Handle to the engine's stdin.
    stdin: ChildStdin,
    /// Channel through which the background reader thread forwards lines.
    rx: Receiver<EngineMessage>,
    /// Identity and options reported during the UCI handshake.
    pub info: EngineInfo,
    /// The FEN of the position that was last sent to the engine.
    current_fen: Option<String>,
    /// Default timeout for blocking reads (milliseconds).
    timeout_ms: u64,
    /// Whether the engine is currently in a ponder search.
    is_pondering: bool,
    /// The move currently being pondered, if any.
    ponder_move: Option<String>,
}

impl UciEngine {
    /// Spawns the engine at `path` and performs the UCI handshake.
    ///
    /// Equivalent to `UciEngine::with_options(path, &[])`.
    pub fn new(path: &str) -> UciResult<Self> {
        Self::with_options(path, &[])
    }

    /// Spawns the engine at `path`, performs the UCI handshake, then applies
    /// each `(name, value)` option pair.
    ///
    /// ```rust,no_run
    /// use lazychess::uci::UciEngine;
    ///
    /// let engine = UciEngine::with_options(
    ///     "/usr/bin/stockfish",
    ///     &[("Threads", "4"), ("Hash", "256")],
    /// ).unwrap();
    /// ```
    pub fn with_options(path: &str, options: &[(&str, &str)]) -> UciResult<Self> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(UciError::SpawnFailed)?;

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");

        let (tx, rx) = mpsc::channel();
        spawn_reader(stdout, tx);

        let mut engine = Self {
            child,
            stdin,
            rx,
            info: EngineInfo::default(),
            current_fen: None,
            timeout_ms: 10_000,
            is_pondering: false,
            ponder_move: None,
        };

        // Perform the UCI handshake.
        engine.handshake()?;

        // Apply caller-supplied options.
        for (name, value) in options {
            engine.set_option(name, value)?;
        }

        Ok(engine)
    }
    
    /// Overrides the default read timeout (milliseconds, default 10 000).
    pub fn set_timeout(&mut self, ms: u64) {
        self.timeout_ms = ms;
    }

    /// Sends a `setoption` command to the engine.
    ///
    /// The engine must support the option; unknown options are silently ignored
    /// by most engines.
    pub fn set_option(&mut self, name: &str, value: &str) -> UciResult<()> {
        self.send(&format!("setoption name {name} value {value}"))
    }

    /// Sends `position startpos` to the engine.
    pub fn set_startpos(&mut self) -> UciResult<()> {
        self.send("position startpos")?;
        self.current_fen = None;
        Ok(())
    }

    /// Sends `position fen <fen>` to the engine.
    pub fn set_position_fen(&mut self, fen: &str) -> UciResult<()> {
        self.send(&format!("position fen {fen}"))?;
        self.current_fen = Some(fen.to_owned());
        Ok(())
    }

    /// Synchronises the engine's position from a [`Game`] instance.
    ///
    /// Internally this sends `position fen <current_fen>`.
    pub fn sync_game(&mut self, game: &Game) -> UciResult<()> {
        self.set_position_fen(&game.get_fen())
    }

    /// Returns the FEN most recently sent to the engine, if any.
    pub fn current_fen(&self) -> Option<&str> {
        self.current_fen.as_deref()
    }

    /// Asks the engine for the best move and waits for `bestmove`.
    ///
    /// Returns the move in UCI notation (e.g. `"e2e4"`, `"e7e8q"`).
    pub fn best_move(&mut self, config: &SearchConfig) -> UciResult<String> {
        // Apply MultiPV setting if requested.
        if let Some(n) = config.multipv {
            self.set_option("MultiPV", &n.to_string())?;
        } else {
            let _ = self.set_option("MultiPV", "1");
        }

        self.send(&config.to_go_command())?;
        let timeout = config.read_timeout_ms.unwrap_or(self.timeout_ms);
        let (best, _, _) = self.wait_for_bestmove_with_timeout(timeout)?;
        Ok(best)
    }

    /// Starts a ponder search in the background using the given search limits.
    ///
    /// The engine searches the position after `ponder_move` (the opponent's
    /// expected reply) in the background. Providing a `config` with `depth` or
    /// `movetime` is strongly recommended — without a limit the engine searches
    /// infinitely and [`ponderhit`] must send `stop` before it can retrieve a
    /// result, which adds latency. With a bounded config (e.g. `depth(16)`) the
    /// engine will stop on its own and [`ponderhit`] returns almost instantly.
    ///
    /// Call [`ponderhit`] if the opponent plays this move, or [`ponder_miss`]
    /// to abort and search from scratch.
    ///
    /// The current position must already be set via [`sync_game`] or
    /// [`set_position_fen`] **before** calling this method — the ponder move
    /// is appended automatically.
    ///
    /// # Example
    /// ```rust,no_run
    /// use lazychess::{Game, uci::{UciEngine, SearchConfig}};
    ///
    /// let mut engine = UciEngine::new("/usr/bin/stockfish").unwrap();
    /// let mut game = Game::new();
    ///
    /// // Engine plays and gives us its ponder suggestion.
    /// let result = engine.play(&game, &SearchConfig::depth(15)).unwrap();
    /// game.do_move(&result.best_move).unwrap();
    ///
    /// // Use the same depth limit for pondering so the engine stops on its own.
    /// if let Some(ref pm) = result.ponder_move {
    ///     engine.sync_game(&game).unwrap();
    ///     engine.ponder(pm, &SearchConfig::depth(15)).unwrap();
    /// }
    ///
    /// // ... opponent moves ...
    ///
    /// // Opponent played the expected move — get the result immediately.
    /// let reply = engine.ponderhit().unwrap();
    /// ```
    pub fn ponder(&mut self, ponder_move: &str, config: &SearchConfig) -> UciResult<()> {
        if self.is_pondering {
            return Err(UciError::EngineError(
                "already pondering; call ponderhit() or ponder_miss() first".into(),
            ));
        }

        // Build a ponder-flavoured go command from the config. to_go_command()
        // prepends "ponder" and then appends depth/movetime/wtime/… so the
        // engine has a concrete stopping condition after ponderhit.
        let ponder_config = SearchConfig {
            ponder_move: Some(ponder_move.to_owned()),
            ..config.clone()
        };

        let pos_cmd = match &self.current_fen {
            Some(fen) => format!("position fen {fen} moves {ponder_move}"),
            None      => format!("position startpos moves {ponder_move}"),
        };
        self.send(&pos_cmd)?;
        self.send(&ponder_config.to_go_command())?;
        self.is_pondering = true;
        self.ponder_move  = Some(ponder_move.to_owned());
        Ok(())
    }

    /// Notifies the engine that the opponent played the expected ponder move.
    ///
    /// Sends `stop` to end the background search, then `ponderhit` so the
    /// engine knows the guess was correct, and finally waits for `bestmove`.
    /// Because the hash table is already warm from pondering, the reply is
    /// returned almost instantly at a much higher effective depth than a cold
    /// search of the same duration.
    ///
    /// Returns a [`PlayResult`] containing the engine's best reply **and** its
    /// ponder suggestion for the next move, so you can chain pondering across
    /// multiple turns without any extra round-trips.
    ///
    /// > **Note for GUI / time-control use:** If you are driving the engine
    /// > from a full GUI that sends `wtime`/`btime` alongside `go ponder`, the
    /// > engine stops on its own when the clock runs out. In that case send
    /// > `ponderhit` without `stop` first. This implementation targets the
    /// > common standalone / programmatic use case where no external clock is
    /// > present.
    pub fn ponderhit(&mut self) -> UciResult<PlayResult> {
        if !self.is_pondering {
            return Err(UciError::EngineError(
                "not currently pondering".into(),
            ));
        }
        // Send stop first so the engine emits bestmove, then ponderhit so it
        // knows the ponder guess was correct (affects internal scoring/stats).
        self.send("stop")?;
        self.send("ponderhit")?;
        self.is_pondering = false;
        self.ponder_move  = None;
        let (best_move, ponder_move, _) = self.wait_for_bestmove()?;
        Ok(PlayResult { best_move, ponder_move })
    }

    /// Aborts the current ponder search (opponent did not play the expected move).
    ///
    /// Sends `stop`, drains the `bestmove` response, and returns the engine to
    /// an idle state. You should then set the new position and start a fresh
    /// search.
    pub fn ponder_miss(&mut self) -> UciResult<()> {
        if !self.is_pondering {
            return Ok(()); // nothing to abort
        }
        self.send("stop")?;
        self.is_pondering = false;
        self.ponder_move = None;
        // Drain the bestmove response the engine sends after stop.
        let _ = self.wait_for_bestmove();
        Ok(())
    }

    /// Returns `true` if the engine is currently in a ponder search.
    pub fn is_pondering(&self) -> bool {
        self.is_pondering
    }

    /// Runs the search and collects all `info` lines, then returns them together
    /// with the best move.
    ///
    /// The returned tuple is `(best_move_uci, analysis_lines)`.
    pub fn best_move_with_analysis(
        &mut self,
        config: &SearchConfig,
    ) -> UciResult<(String, Vec<AnalysisInfo>)> {
        if let Some(n) = config.multipv {
            self.set_option("MultiPV", &n.to_string())?;
        }

        self.send(&config.to_go_command())?;

        // Use per-search timeout if set, otherwise fall back to global.
        let timeout = config.read_timeout_ms.unwrap_or(self.timeout_ms);
        let (best, _, infos) = self.wait_for_bestmove_with_timeout(timeout)?;
        Ok((best, infos))
    }
    
    /// Returns the top-N best moves for a raw board position.
    ///
    /// Equivalent to [`top_moves`] but accepts a `&Board` directly, which is
    /// useful when analysing temporary positions inside the move classifier.
    pub fn top_moves_from_board(
        &mut self,
        board: &Board,
        n: u32,
        config: &SearchConfig,
    ) -> UciResult<Vec<(String, Option<Score>)>> {
        let n = n.clamp(1, 500);
        let cfg = SearchConfig {
            multipv: Some(n),
            ..config.clone()
        };
        let fen = crate::fen::board_to_fen(board);
        self.set_position_fen(&fen)?;
        let (_, infos) = self.best_move_with_analysis(&cfg)?;

        let mut by_line: std::collections::HashMap<u32, AnalysisInfo> =
            std::collections::HashMap::new();
        for info in infos {
            let key = info.multipv.unwrap_or(1);
            let deeper = by_line.get(&key).and_then(|p| p.depth).unwrap_or(0);
            if info.depth.unwrap_or(0) >= deeper {
                by_line.insert(key, info);
            }
        }

        let mut keys: Vec<u32> = by_line.keys().copied().collect();
        keys.sort_unstable();

        Ok(keys
            .into_iter()
            .filter_map(|k| {
                let info = by_line.remove(&k)?;
                let mv = info.pv.first()?.clone();
                Some((mv, info.score))
            })
            .collect())
    }
    
    /// Evaluates a raw board position and returns a single score.
    ///
    /// Equivalent to [`evaluate`] but accepts a `&Board` directly.
    pub fn evaluate_board(
        &mut self,
        board: &Board,
        config: &SearchConfig,
    ) -> UciResult<Option<Score>> {
        let fen = crate::fen::board_to_fen(board);
        self.set_position_fen(&fen)?;
        let infos = self.analyze(config)?;
        Ok(infos
            .iter()
            .filter(|i| i.multipv.unwrap_or(1) == 1)
            .max_by_key(|i| i.depth.unwrap_or(0))
            .and_then(|i| i.score.clone()))
    }

    /// Runs a full analysis and returns all `info` lines (excluding the final
    /// `bestmove` line).
    pub fn analyze(&mut self, config: &SearchConfig) -> UciResult<Vec<AnalysisInfo>> {
        let (_, infos) = self.best_move_with_analysis(config)?;
        Ok(infos)
    }

    /// Sends `stop` to the engine, ending an ongoing infinite search.
    ///
    /// After calling this you should call [`best_move`] with
    /// `SearchConfig::infinite()` to retrieve the result, or use
    /// [`best_move_with_analysis`] which calls `stop` internally when
    /// `infinite` is set.
    pub fn stop(&mut self) -> UciResult<()> {
        self.send("stop")
    }

    /// Sends `ucinewgame` to reset the engine's internal state (hash tables,
    /// history, etc.) between games.
    pub fn new_game(&mut self) -> UciResult<()> {
        self.send("ucinewgame")?;
        self.current_fen = None;
        self.wait_for_readyok()?;
        Ok(())
    }

    /// Returns the best move for the position in `game` using the given config.
    ///
    /// This is a convenience wrapper around `sync_game` + `best_move`.
    pub fn best_move_for_game(
        &mut self,
        game: &Game,
        config: &SearchConfig,
    ) -> UciResult<String> {
        self.sync_game(game)?;
        self.best_move(config)
    }

    /// Plays a move for the current position and returns a [`PlayResult`].
    ///
    /// Unlike [`best_move_for_game`], this method also captures the engine's
    /// **ponder suggestion** — the opponent move the engine expects next. You
    /// can pass `result.ponder_move` straight to [`ponder`] to start background
    /// thinking while waiting for the opponent.
    ///
    /// This mirrors the python-chess `engine.play(..., ponder=True)` workflow.
    ///
    /// # Example
    /// ```rust,no_run
    /// use lazychess::{Game, uci::{UciEngine, SearchConfig}};
    ///
    /// let mut engine = UciEngine::new("/usr/bin/stockfish").unwrap();
    /// let mut game = Game::new();
    ///
    /// let config = SearchConfig::depth(15);
    ///
    /// let result = engine.play(&game, &config).unwrap();
    /// println!("Best move : {}", result.best_move);
    /// if let Some(ref pm) = result.ponder_move {
    ///     println!("Ponder on : {pm}");
    /// }
    /// game.do_move(&result.best_move).unwrap();
    ///
    /// // Start pondering while waiting for the opponent, using the same depth.
    /// if let Some(ref pm) = result.ponder_move {
    ///     engine.sync_game(&game).unwrap();
    ///     engine.ponder(pm, &config).unwrap();
    /// }
    /// ```
    pub fn play(&mut self, game: &Game, config: &SearchConfig) -> UciResult<PlayResult> {
        self.sync_game(game)?;

        if let Some(n) = config.multipv {
            self.set_option("MultiPV", &n.to_string())?;
        } else {
            let _ = self.set_option("MultiPV", "1");
        }

        self.send(&config.to_go_command())?;
        let timeout = config.read_timeout_ms.unwrap_or(self.timeout_ms);
        let (best_move, ponder_move, _) = self.wait_for_bestmove_with_timeout(timeout)?;
        Ok(PlayResult { best_move, ponder_move })
    }

    /// Runs a full analysis for the position in `game`.
    pub fn analyze_game(
        &mut self,
        game: &Game,
        config: &SearchConfig,
    ) -> UciResult<Vec<AnalysisInfo>> {
        self.sync_game(game)?;
        self.analyze(config)
    }

    /// Returns the top-N best moves with their scores using MultiPV.
    ///
    /// `n` is clamped to the range `[1, 500]`.
    ///
    /// Returns a `Vec` of `(move_uci, score)` sorted by the engine's ranking
    /// (best first).
    pub fn top_moves(
        &mut self,
        game: &Game,
        n: u32,
        config: &SearchConfig,
    ) -> UciResult<Vec<(String, Option<Score>)>> {
        let n = n.clamp(1, 500);
        let cfg = SearchConfig {
            multipv: Some(n),
            ..config.clone()
        };

        self.sync_game(game)?;
        let (_, infos) = self.best_move_with_analysis(&cfg)?;

        // Keep only the deepest info for each multipv line.
        let mut by_line: std::collections::HashMap<u32, AnalysisInfo> =
            std::collections::HashMap::new();
        for info in infos {
            let key = info.multipv.unwrap_or(1);
            let deeper = by_line
                .get(&key)
                .and_then(|prev| prev.depth)
                .unwrap_or(0);
            if info.depth.unwrap_or(0) >= deeper {
                by_line.insert(key, info);
            }
        }

        let mut keys: Vec<u32> = by_line.keys().copied().collect();
        keys.sort_unstable();

        let result = keys
            .into_iter()
            .filter_map(|k| {
                let info = by_line.remove(&k)?;
                let mv = info.pv.first()?.clone();
                Some((mv, info.score))
            })
            .collect();

        Ok(result)
    }

    /// Evaluates the current position and returns a single score.
    ///
    /// Equivalent to running a depth-limited search and returning its score.
    pub fn evaluate(
        &mut self,
        game: &Game,
        config: &SearchConfig,
    ) -> UciResult<Option<Score>> {
        let infos = self.analyze_game(game, config)?;
        // Return the score from the deepest line (multipv 1).
        Ok(infos
            .iter()
            .filter(|i| i.multipv.unwrap_or(1) == 1)
            .max_by_key(|i| i.depth.unwrap_or(0))
            .and_then(|i| i.score.clone()))
    }

    /// Sends `quit` and waits for the process to exit.
    pub fn quit(mut self) -> UciResult<()> {
        let _ = self.send("quit");
        let _ = self.child.wait();
        Ok(())
    }

    /// Sends a line to the engine's stdin (newline appended automatically).
    fn send(&mut self, line: &str) -> UciResult<()> {
        writeln!(self.stdin, "{line}").map_err(UciError::WriteFailed)?;
        self.stdin.flush().map_err(UciError::WriteFailed)
    }

    /// Reads one line from the engine, honouring the timeout.
    fn read_line(&self) -> UciResult<Option<String>> {
        self.read_line_timeout(self.timeout_ms)
    }

    /// Reads one line with an explicit timeout (milliseconds).
    fn read_line_timeout(&self, timeout_ms: u64) -> UciResult<Option<String>> {
        match self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(EngineMessage::Line(l)) => Ok(Some(l)),
            Ok(EngineMessage::Eof) => Ok(None),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(UciError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(UciError::ProcessDied),
        }
    }

    /// Drains lines until `readyok` is received.
    fn wait_for_readyok(&mut self) -> UciResult<()> {
        self.send("isready")?;
        loop {
            match self.read_line()? {
                Some(line) if line.trim() == "readyok" => return Ok(()),
                Some(_) => {} // ignore other lines
                None => return Err(UciError::ProcessDied),
            }
        }
    }

    /// Performs the full UCI handshake: sends `uci`, collects `id`/`option`
    /// lines, waits for `uciok`, then confirms readiness via `isready`.
    fn handshake(&mut self) -> UciResult<()> {
        self.send("uci")?;

        loop {
            match self.read_line()? {
                Some(line) => {
                    let trimmed = line.trim();
                    if trimmed == "uciok" {
                        break;
                    }
                    if let Some(rest) = trimmed.strip_prefix("id name ") {
                        self.info.name = Some(rest.to_owned());
                    } else if let Some(rest) = trimmed.strip_prefix("id author ") {
                        self.info.author = Some(rest.to_owned());
                    } else if trimmed.starts_with("option ")
                        && let Some(opt) = parse_option_line(trimmed) {
                            self.info.options.push(opt);
                        }
                }
                None => return Err(UciError::ProcessDied),
            }
        }

        self.wait_for_readyok()
    }

    /// Waits for `bestmove` and collects all preceding `info` lines.
    ///
    /// Returns `(best_move, ponder_suggestion, info_lines)`.
    /// `ponder_suggestion` is the move the engine recommends pondering on next
    /// (parsed from `bestmove e2e4 ponder e7e5`), if the engine provided one.
    fn wait_for_bestmove_with_timeout(
        &mut self,
        timeout_ms: u64,
    ) -> UciResult<(String, Option<String>, Vec<AnalysisInfo>)> {
        let mut infos: Vec<AnalysisInfo> = Vec::new();

        loop {
            match self.read_line_timeout(timeout_ms)? {
                Some(line) => {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("bestmove ") {
                        // Parse `bestmove e2e4 ponder e7e5`
                        let mut tokens = rest.split_whitespace();
                        let best = tokens.next().unwrap_or("").to_owned();
                        if best.is_empty() || best == "(none)" {
                            return Err(UciError::EngineError(
                                "engine returned no bestmove".into(),
                            ));
                        }
                        // Extract the engine's ponder suggestion if present.
                        let ponder_suggestion = if tokens.next() == Some("ponder") {
                            tokens.next().map(str::to_owned)
                        } else {
                            None
                        };
                        return Ok((best, ponder_suggestion, infos));
                    } else if trimmed.starts_with("info ")
                        && let Some(info) = parse_info_line(trimmed) {
                            infos.push(info);
                        }
                }
                None => return Err(UciError::ProcessDied),
            }
        }
    }

    /// Waits for `bestmove` and collects all preceding `info` lines.
    fn wait_for_bestmove(&mut self) -> UciResult<(String, Option<String>, Vec<AnalysisInfo>)> {
        self.wait_for_bestmove_with_timeout(self.timeout_ms)
    }
}

impl Drop for UciEngine {
    /// Attempts a graceful shutdown when the engine is dropped.
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let _ = self.child.wait();
    }
}

/// Parses an `option name … type … default … min … max … var …` line.
fn parse_option_line(line: &str) -> Option<UciOption> {
    // Strip the leading "option " token.
    let rest = line.strip_prefix("option ")?;

    let mut name = String::new();
    let mut kind = String::new();
    let mut default = None;
    let mut min = None;
    let mut max = None;
    let mut vars = Vec::new();

    // Tokenise by known keywords; each keyword ends the previous value.
    let keywords = ["name", "type", "default", "min", "max", "var"];

    let tokens: Vec<&str> = rest.splitn(2, "name ").collect();
    // Re-join and parse properly using a keyword-state machine.
    let mut current_key: Option<&str> = None;
    let mut current_val: Vec<&str> = Vec::new();

    let flush = |key: &str, val: &[&str],
                 name: &mut String,
                 kind: &mut String,
                 default: &mut Option<String>,
                 min: &mut Option<String>,
                 max: &mut Option<String>,
                 vars: &mut Vec<String>| {
        let v = val.join(" ");
        match key {
            "name" => *name = v,
            "type" => *kind = v,
            "default" => *default = Some(v),
            "min" => *min = Some(v),
            "max" => *max = Some(v),
            "var" => vars.push(v),
            _ => {}
        }
    };

    for word in rest.split_whitespace() {
        if keywords.contains(&word) {
            if let Some(key) = current_key {
                flush(key, &current_val, &mut name, &mut kind,
                      &mut default, &mut min, &mut max, &mut vars);
                current_val.clear();
            }
            current_key = Some(word);
        } else {
            current_val.push(word);
        }
    }
    if let Some(key) = current_key {
        flush(key, &current_val, &mut name, &mut kind,
              &mut default, &mut min, &mut max, &mut vars);
    }

    // Silence an unused-variable warning for the old `tokens` binding.
    drop(tokens);

    if name.is_empty() {
        return None;
    }

    Some(UciOption { name, kind, default, min, max, vars })
}

/// Parses an `info …` line into an [`AnalysisInfo`].
///
/// Unknown tokens are silently skipped.
fn parse_info_line(line: &str) -> Option<AnalysisInfo> {
    // Must start with "info".
    let rest = line.strip_prefix("info")?;

    let mut info = AnalysisInfo::empty();
    let mut words: std::iter::Peekable<std::str::SplitWhitespace> =
        rest.split_whitespace().peekable();

    while let Some(token) = words.next() {
        match token {
            "depth" => {
                info.depth = words.next().and_then(|w| w.parse().ok());
            }
            "seldepth" => {
                info.seldepth = words.next().and_then(|w| w.parse().ok());
            }
            "multipv" => {
                info.multipv = words.next().and_then(|w| w.parse().ok());
            }
            "nodes" => {
                info.nodes = words.next().and_then(|w| w.parse().ok());
            }
            "nps" => {
                info.nps = words.next().and_then(|w| w.parse().ok());
            }
            "time" => {
                info.time_ms = words.next().and_then(|w| w.parse().ok());
            }
            "hashfull" => {
                info.hashfull = words.next().and_then(|w| w.parse().ok());
            }
            "score" => {
                if let Some(kind) = words.next()
                    && let Some(val) = words.next().and_then(|w| w.parse::<i32>().ok()) {
                        info.score = Some(match kind {
                            "cp" => Score::Centipawns(val),
                            "mate" => Score::Mate(val),
                            _ => continue,
                        });
                        // Consume optional "lowerbound" / "upperbound" tokens.
                        if let Some(&next) = words.peek()
                            && (next == "lowerbound" || next == "upperbound") {
                                words.next();
                            }
                    }
            }
            "pv" => {
                // All remaining tokens belong to the PV.
                info.pv = words.by_ref().map(str::to_owned).collect();
            }
            "string" => {
                // `info string <free text>` — collect all remaining tokens.
                let msg: Vec<&str> = words.by_ref().collect();
                if !msg.is_empty() {
                    info.message = Some(msg.join(" "));
                }
            }
            // Ignored tokens: "currmove", "currmovenumber", "cpuload", "tbhits", …
            _ => {}
        }
    }

    Some(info)
}
