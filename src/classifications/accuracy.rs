use crate::types::Color;
use super::types::Evaluation;
use super::expected_points::get_expected_points_loss;

/// Converts an expected point loss to a move accuracy percentage [0.0, 100.0].
///
/// Uses the same exponential decay formula as chess.com:
/// `accuracy = 103.16 * e^(-4 * point_loss) - 3.17`
///
/// The constants are tuned so that:
/// - A best move (0.0 loss) -> ~100%
/// - A blunder (large loss) -> ~0%
pub fn get_move_accuracy(point_loss: f64) -> f64 {
    (103.16 * (-4.0 * point_loss).exp() - 3.17).clamp(0.0, 100.0)
}

/// Computes the move accuracy directly from two evaluations.
pub fn get_move_accuracy_from_evals(
    before: &Evaluation,
    after: &Evaluation,
    color: Color,
) -> f64 {
    let loss = get_expected_points_loss(before, after, color);
    get_move_accuracy(loss)
}

/// Summary accuracy statistics for one player over a full game.
#[derive(Debug, Clone)]
pub struct PlayerAccuracy {
    /// Average move accuracy across all classified moves [0.0, 100.0].
    pub average: f64,
    /// Individual move accuracies in order.
    pub per_move: Vec<f64>,
}

impl PlayerAccuracy {
    /// Computes accuracy stats from a list of per-move point losses.
    pub fn from_point_losses(losses: &[f64]) -> Self {
        let per_move: Vec<f64> = losses.iter().map(|&l| get_move_accuracy(l)).collect();
        let average = if per_move.is_empty() {
            0.0
        } else {
            per_move.iter().sum::<f64>() / per_move.len() as f64
        };
        Self { average, per_move }
    }
}