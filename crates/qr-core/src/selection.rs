//! Deterministic QR mask selection over completed candidates.
//!
//! ISO/IEC 18004:2024 mask evaluation and selection.
//! 2024 clause mapping pending audit.
//! Compared against Nayuki 1.8.0
//! `rust/src/lib.rs::{draw_format_bits,draw_version,get_penalty_score}` and
//! python-qrcode 8.2 `qrcode/main.py::{makeImpl,best_mask_pattern}` plus
//! `qrcode/util.py::lost_point`. Their completed matrices agree but exposed
//! penalty totals do not. The owner-approved interpretation selects literal
//! complete Rule 3 windows without virtual quiet-zone padding; all differing
//! totals remain recorded in
//! `.scratch/qrcode-generator/penalty-oracle-disagreement.md`.

use crate::codeword_stream::InterleavedCodewords;
use crate::matrix::{
    InformationError, MaskError, MaskId, MatrixError, ModuleMatrix, PlacementError,
    build_function_matrix, finalize_information, place_data,
};
use crate::penalty::penalty_score;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedMask {
    mask: MaskId,
    penalty: u32,
    matrix: ModuleMatrix,
}

impl SelectedMask {
    #[must_use]
    pub const fn mask(&self) -> MaskId {
        self.mask
    }

    #[must_use]
    pub const fn penalty(&self) -> u32 {
        self.penalty
    }

    #[must_use]
    pub const fn matrix(&self) -> &ModuleMatrix {
        &self.matrix
    }

    #[must_use]
    pub fn into_matrix(self) -> ModuleMatrix {
        self.matrix
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionError {
    NoCandidates,
    Mask(MaskError),
    Matrix(MatrixError),
    Placement(PlacementError),
    Information(InformationError),
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCandidates => formatter.write_str("QR mask selection produced no candidates"),
            Self::Mask(error) => error.fmt(formatter),
            Self::Matrix(error) => error.fmt(formatter),
            Self::Placement(error) => error.fmt(formatter),
            Self::Information(error) => error.fmt(formatter),
        }
    }
}

impl Error for SelectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoCandidates => None,
            Self::Mask(error) => Some(error),
            Self::Matrix(error) => Some(error),
            Self::Placement(error) => Some(error),
            Self::Information(error) => Some(error),
        }
    }
}

impl From<MaskError> for SelectionError {
    fn from(error: MaskError) -> Self {
        Self::Mask(error)
    }
}

impl From<MatrixError> for SelectionError {
    fn from(error: MatrixError) -> Self {
        Self::Matrix(error)
    }
}

impl From<PlacementError> for SelectionError {
    fn from(error: PlacementError) -> Self {
        Self::Placement(error)
    }
}

impl From<InformationError> for SelectionError {
    fn from(error: InformationError) -> Self {
        Self::Information(error)
    }
}

pub fn select_mask(stream: &InterleavedCodewords) -> Result<SelectedMask, SelectionError> {
    let version = stream.version();
    let mut selected = None;
    for mask_number in MaskId::MIN..=MaskId::MAX {
        let mask = MaskId::new(mask_number)?;
        let matrix = place_data(build_function_matrix(version)?, stream, mask)?;
        let matrix = finalize_information(matrix)?;
        let penalty = penalty_score(&matrix);
        if is_better_candidate(
            mask,
            penalty,
            selected
                .as_ref()
                .map(|current: &SelectedMask| (current.mask, current.penalty)),
        ) {
            selected = Some(SelectedMask {
                mask,
                penalty,
                matrix,
            });
        }
    }
    selected.ok_or(SelectionError::NoCandidates)
}

fn is_better_candidate(mask: MaskId, penalty: u32, selected: Option<(MaskId, u32)>) -> bool {
    selected.is_none_or(|(selected_mask, selected_penalty)| {
        penalty < selected_penalty
            || (penalty == selected_penalty && mask.number() < selected_mask.number())
    })
}

#[cfg(test)]
mod tests {
    use super::is_better_candidate;
    use crate::matrix::MaskId;

    #[test]
    fn lower_mask_id_wins_equal_penalties() {
        let low = MaskId::new(1).expect("mask 1 is valid");
        let high = MaskId::new(5).expect("mask 5 is valid");
        assert!(is_better_candidate(low, 20, Some((high, 20))));
        assert!(!is_better_candidate(high, 20, Some((low, 20))));
    }
}
