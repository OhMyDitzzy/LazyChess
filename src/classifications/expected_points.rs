use crate::types::Color;
use super::types::Evaluation;

/// The centipawn-to-win-probability gradient used in the sigmoid function.
/// Matches the value used by Lichess / chess.com analysis.
const CP_GRADIENT: f64 = 0.0035;

/// Converts an engine evaluation to an expected points value in [0.0, 1.0].
///
/// Uses a logistic (sigmoid) function for centipawn evaluations:
/// `P(win) = 1 / (1 + e^(-0.0035 * cp))`
///
/// This means the same raw centipawn loss carries more weight near equality
/// (e.g. +0.5 -> -0.5) than it does in already-winning positions
/// (e.g. +5.0 -> +4.0), which matches human intuition.
///
/// For mate evaluations:
/// - Mate > 0 (mating) -> 1.0
/// - Mate < 0 (being mated) -> 0.0
/// - Mate == 0 (stalemated / already mated) -> depends on side to move
pub fn get_expected_points(eval: &Evaluation, color: Option<Color>) -> f64 {
    match eval {
        Evaluation::Mate(0) => {
            // Stalemate or already mated 
            // result depends on who just moved.
            color.map(|c| if c == Color::White { 1.0 } else { 0.0 })
                 .unwrap_or(0.5)
        }
        Evaluation::Mate(v) => {
            if *v > 0 { 1.0 } else { 0.0 }
        }
        Evaluation::Centipawn(cp) => {
            1.0 / (1.0 + (-CP_GRADIENT * *cp as f64).exp())
        }
    }
}

/// Returns the expected point loss when moving from `before` to `after`,
/// from the perspective of `color`.
///
/// The result is clamped to [0.0, 1.0]. A loss of 0.0 means the move was
/// at least as good as the previous position; 1.0 means a full win was lost.
pub fn get_expected_points_loss(
    before: &Evaluation,
    after: &Evaluation,
    color: Color,
) -> f64 {
    let before_pts = get_expected_points(before, Some(color.opposite()));
    let after_pts  = get_expected_points(after,  Some(color));

    // From White's perspective: loss = before_white - after_white.
    // Flip sign for Black since evaluations are from White's perspective.
    let raw_loss = match color {
        Color::White => before_pts - after_pts,
        Color::Black => after_pts  - before_pts,
    };

    raw_loss.clamp(0.0, 1.0)
}
