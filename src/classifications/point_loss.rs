use super::types::{ClassificationContext, ClassificationKind, Evaluation};

/// Classifies a move using expected point loss and mate-distance logic.
///
/// This is the primary classification path for most moves (everything except
/// Brilliant, Great, Forced, Book, and Risky which require board-level context).
/// handling all four
/// combinations of evaluation types (mate↔mate, mate↔cp, cp↔mate, cp↔cp).
pub fn point_loss_classify(ctx: &ClassificationContext) -> ClassificationKind {
    let prev_subj = ctx.eval_before.subjective(ctx.color);
    let curr_subj = ctx.eval_after.subjective(ctx.color);

    match (&prev_subj, &curr_subj) {
        (Evaluation::Mate(prev_m), Evaluation::Mate(curr_m)) => {
            let prev_v = *prev_m;
            let curr_v = *curr_m;

            // Winning mate flipped to losing mate. severe mistake or blunder.
            if prev_v > 0 && curr_v < 0 {
                return if curr_v < -3 {
                    ClassificationKind::Mistake
                } else {
                    ClassificationKind::Blunder
                };
            }

            // For the losing side, keeping mate the same is best.
            // For the winning side, mate distance should decrease by 1 per move.
            let raw_before = ctx.eval_before.value();
            let raw_after  = ctx.eval_after.value();
            let mate_loss = match ctx.color {
                crate::types::Color::White => raw_after - raw_before,
                crate::types::Color::Black => raw_before - raw_after,
            };

            if mate_loss < 0 || (mate_loss == 0 && curr_v < 0) {
                ClassificationKind::Best
            } else if mate_loss < 2 {
                ClassificationKind::Excellent
            } else if mate_loss < 7 {
                ClassificationKind::Okay
            } else {
                ClassificationKind::Inaccuracy
            }
        }

        (Evaluation::Mate(_), Evaluation::Centipawn(cp)) => {
            let v = *cp;
            if v >= 800 {
                ClassificationKind::Excellent
            } else if v >= 400 {
                ClassificationKind::Okay
            } else if v >= 200 {
                ClassificationKind::Inaccuracy
            } else if v >= 0 {
                ClassificationKind::Mistake
            } else {
                ClassificationKind::Blunder
            }
        }

        (Evaluation::Centipawn(_), Evaluation::Mate(m)) => {
            let v = *m;
            if v > 0 {
                ClassificationKind::Best
            } else if v >= -2 {
                ClassificationKind::Blunder
            } else if v >= -5 {
                ClassificationKind::Mistake
            } else {
                ClassificationKind::Inaccuracy
            }
        }

        (Evaluation::Centipawn(_), Evaluation::Centipawn(_)) => {
            let loss = ctx.point_loss;

            if loss < 0.01 {
                ClassificationKind::Best
            } else if loss < 0.045 {
                ClassificationKind::Excellent
            } else if loss < 0.08 {
                ClassificationKind::Okay
            } else if loss < 0.12 {
                ClassificationKind::Inaccuracy
            } else if loss < 0.22 {
                ClassificationKind::Mistake
            } else {
                ClassificationKind::Blunder
            }
        }
    }
}
