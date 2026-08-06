//! Penalty scoring for completed QR matrices.
//!
//! ISO/IEC 18004:2024 mask evaluation penalty rules; 2024 clause mapping
//! pending audit. Corroborated by Nayuki 1.8.0
//! `rust/src/lib.rs::{get_penalty_score,FinderPenalty}` and python-qrcode 8.2
//! `qrcode/util.py::lost_point`. Evidence is `public-corroborated,
//! non-normative` pending a complete 2024 audit.

use crate::matrix::ModuleMatrix;

#[must_use]
pub fn penalty_score(matrix: &ModuleMatrix) -> u32 {
    let size = usize::from(matrix.size());
    let values = matrix
        .modules()
        .map(|module| module.is_dark())
        .collect::<Vec<_>>();
    let rows = values
        .chunks(size)
        .map(<[bool]>::to_vec)
        .collect::<Vec<_>>();

    let mut score = block_penalty(&rows);
    for row in &rows {
        score = score
            .saturating_add(run_penalty(row))
            .saturating_add(finder_like_penalty(row));
    }
    for x in 0..size {
        let column = rows
            .iter()
            .filter_map(|row| row.get(x).copied())
            .collect::<Vec<_>>();
        score = score
            .saturating_add(run_penalty(&column))
            .saturating_add(finder_like_penalty(&column));
    }
    score.saturating_add(balance_penalty(
        values
            .iter()
            .filter(|&&dark| dark)
            .fold(0_u32, |count, _| count.saturating_add(1)),
        u32::from(matrix.size()).saturating_mul(u32::from(matrix.size())),
    ))
}

fn run_penalty(line: &[bool]) -> u32 {
    let Some(&first) = line.first() else {
        return 0;
    };
    let mut current = first;
    let mut length = 0_u32;
    let mut score = 0_u32;
    for &value in line {
        if value == current {
            length = length.saturating_add(1);
        } else {
            score = score.saturating_add(run_cost(length));
            current = value;
            length = 1;
        }
    }
    score.saturating_add(run_cost(length))
}

fn run_cost(length: u32) -> u32 {
    if length < 5 { 0 } else { length - 2 }
}

fn block_penalty(modules: &[Vec<bool>]) -> u32 {
    modules
        .windows(2)
        .map(|rows| {
            let [top_row, bottom_row] = rows else {
                return 0;
            };
            top_row
                .windows(2)
                .zip(bottom_row.windows(2))
                .filter(|(top, bottom)| {
                    matches!((*top, *bottom), ([a, b], [c, d]) if a == b && a == c && a == d)
                })
                .fold(0_u32, |count, _| count.saturating_add(1))
        })
        .sum::<u32>()
        .saturating_mul(3)
}

fn finder_like_penalty(line: &[bool]) -> u32 {
    const LEADING_LIGHT: [bool; 11] = [
        false, false, false, false, true, false, true, true, true, false, true,
    ];
    const TRAILING_LIGHT: [bool; 11] = [
        true, false, true, true, true, false, true, false, false, false, false,
    ];
    line.windows(11)
        .filter(|window| *window == LEADING_LIGHT || *window == TRAILING_LIGHT)
        .fold(0_u32, |score, _| score.saturating_add(40))
}

fn balance_penalty(dark: u32, total: u32) -> u32 {
    if total == 0 {
        return 0;
    }
    let deviation = (dark.saturating_mul(20)).abs_diff(total.saturating_mul(10));
    (deviation / total).saturating_mul(10)
}

#[cfg(test)]
mod tests {
    use super::{balance_penalty, block_penalty, finder_like_penalty, run_penalty};

    #[test]
    fn runs_start_at_five_and_grow_one_point_per_module() {
        assert_eq!(run_penalty(&[true; 4]), 0);
        assert_eq!(run_penalty(&[true; 5]), 3);
        assert_eq!(run_penalty(&[false; 7]), 5);
        assert_eq!(
            run_penalty(&[
                true, true, true, true, true, false, false, false, false, false
            ]),
            6
        );
    }

    #[test]
    fn uniform_two_by_two_blocks_each_cost_three_points() {
        assert_eq!(block_penalty(&vec![vec![true; 3]; 3]), 12);
        assert_eq!(block_penalty(&[vec![true, false], vec![false, true]]), 0);
    }

    #[test]
    fn finder_like_patterns_require_four_light_modules_of_context() {
        let leading_context = [
            false, false, false, false, true, false, true, true, true, false, true, true,
        ];
        let trailing_context = [
            true, true, false, true, true, true, false, true, false, false, false, false,
        ];
        assert_eq!(finder_like_penalty(&leading_context), 40);
        assert_eq!(finder_like_penalty(&trailing_context), 40);
        assert_eq!(
            finder_like_penalty(&[true, true, false, true, true, true, false, true, true, true]),
            0
        );
    }

    #[test]
    fn dark_balance_rounds_down_to_complete_five_percent_steps() {
        assert_eq!(balance_penalty(50, 100), 0);
        assert_eq!(balance_penalty(54, 100), 0);
        assert_eq!(balance_penalty(55, 100), 10);
        assert_eq!(balance_penalty(45, 100), 10);
        assert_eq!(balance_penalty(0, 100), 100);
    }
}
