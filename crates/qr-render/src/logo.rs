use qr_core::matrix::{ModuleKind, ModuleMatrix};

use crate::RenderError;

pub const BUNDLED_LOGO_SVG: &str = include_str!("../../../assets/RGB-one-lettermark-magenta.svg");

const SOURCE_VIEW_BOX_WIDTH: u32 = 1_000;
const SOURCE_VIEW_BOX_HEIGHT: u32 = 602;
const MODULE_UNITS: u32 = 1_000;

// Largest source-box widths that passed the committed H-level profile/version
// decode matrix. Odd widths allow exact visual centering in every odd-width QR
// matrix while the one-module knockout edges remain on the module grid.
const SOURCE_WIDTH_MODULES: [u32; 13] = [1, 3, 3, 3, 5, 5, 7, 7, 7, 9, 9, 9, 11];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleCoordinate(u32);

impl ModuleCoordinate {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoKnockoutBounds {
    left: ModuleCoordinate,
    top: ModuleCoordinate,
    width: ModuleCoordinate,
    height: ModuleCoordinate,
}

impl LogoKnockoutBounds {
    #[must_use]
    pub const fn left(self) -> ModuleCoordinate {
        self.left
    }

    #[must_use]
    pub const fn top(self) -> ModuleCoordinate {
        self.top
    }

    #[must_use]
    pub const fn width(self) -> ModuleCoordinate {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> ModuleCoordinate {
        self.height
    }

    #[must_use]
    pub(crate) const fn contains(self, x: u32, y: u32) -> bool {
        x >= self.left.0
            && x < self.left.0 + self.width.0
            && y >= self.top.0
            && y < self.top.0 + self.height.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoSourceBounds {
    left_thousandths: u32,
    top_thousandths: u32,
    width_thousandths: u32,
    height_thousandths: u32,
}

impl LogoSourceBounds {
    #[must_use]
    pub const fn left_thousandths(self) -> u32 {
        self.left_thousandths
    }

    #[must_use]
    pub const fn top_thousandths(self) -> u32 {
        self.top_thousandths
    }

    #[must_use]
    pub const fn width_thousandths(self) -> u32 {
        self.width_thousandths
    }

    #[must_use]
    pub const fn height_thousandths(self) -> u32 {
        self.height_thousandths
    }

    #[must_use]
    pub const fn right_thousandths(self) -> u32 {
        self.left_thousandths + self.width_thousandths
    }

    #[must_use]
    pub const fn bottom_thousandths(self) -> u32 {
        self.top_thousandths + self.height_thousandths
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoPlacement {
    source: LogoSourceBounds,
    knockout: LogoKnockoutBounds,
    offset_x: i32,
    offset_y: i32,
    protected_clearance: u32,
    obscured_data_modules: u32,
    obscured_remainder_modules: u32,
}

impl LogoPlacement {
    #[must_use]
    pub const fn source_bounds(self) -> LogoSourceBounds {
        self.source
    }

    #[must_use]
    pub const fn knockout_bounds(self) -> LogoKnockoutBounds {
        self.knockout
    }

    #[must_use]
    pub const fn offset(self) -> (i32, i32) {
        (self.offset_x, self.offset_y)
    }

    #[must_use]
    pub const fn protected_clearance(self) -> u32 {
        self.protected_clearance
    }

    #[must_use]
    pub const fn obscured_data_modules(self) -> u32 {
        self.obscured_data_modules
    }

    #[must_use]
    pub const fn obscured_remainder_modules(self) -> u32 {
        self.obscured_remainder_modules
    }

    #[must_use]
    pub const fn obscured_modules(self) -> u32 {
        self.obscured_data_modules + self.obscured_remainder_modules
    }
}

pub(crate) fn calculate_logo_placement(
    matrix: &ModuleMatrix,
) -> Result<LogoPlacement, RenderError> {
    let version_index = usize::from(matrix.version().number() - 1);
    let source_width_modules = SOURCE_WIDTH_MODULES
        .get(version_index)
        .copied()
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let matrix_width = u32::from(matrix.size());
    let source_width = source_width_modules
        .checked_mul(MODULE_UNITS)
        .ok_or(RenderError::DimensionOverflow)?;
    let source_height = source_width_modules
        .checked_mul(SOURCE_VIEW_BOX_HEIGHT)
        .ok_or(RenderError::DimensionOverflow)?;
    let centered_left = matrix_width
        .checked_mul(MODULE_UNITS)
        .and_then(|width| width.checked_sub(source_width))
        .map(|difference| difference / 2)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let centered_top = matrix_width
        .checked_mul(MODULE_UNITS)
        .and_then(|width| width.checked_sub(source_height))
        .map(|difference| difference / 2)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let centered_knockout =
        knockout_for_source(centered_left, centered_top, source_width, source_height)?;

    let search_limit = i32::try_from(matrix_width).map_err(|_| RenderError::DimensionOverflow)?;
    let mut selected: Option<((i64, i32, i32), LogoPlacement)> = None;
    for offset_y in -search_limit..=search_limit {
        for offset_x in -search_limit..=search_limit {
            let Some(knockout) =
                shifted_knockout(centered_knockout, offset_x, offset_y, matrix_width)
            else {
                continue;
            };
            let Some((data_count, remainder_count, clearance)) = analyze_knockout(matrix, knockout)
            else {
                continue;
            };
            let source_left = shifted_units(centered_left, offset_x)?;
            let source_top = shifted_units(centered_top, offset_y)?;
            let distance = i64::from(offset_x).pow(2) + i64::from(offset_y).pow(2);
            let key = (distance, offset_y, offset_x);
            let placement = LogoPlacement {
                source: LogoSourceBounds {
                    left_thousandths: source_left,
                    top_thousandths: source_top,
                    width_thousandths: source_width,
                    height_thousandths: source_height,
                },
                knockout,
                offset_x,
                offset_y,
                protected_clearance: clearance,
                obscured_data_modules: data_count,
                obscured_remainder_modules: remainder_count,
            };
            if selected
                .as_ref()
                .is_none_or(|(selected_key, _)| key < *selected_key)
            {
                selected = Some((key, placement));
            }
        }
    }

    let placement = selected
        .map(|(_, placement)| placement)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let knockout = placement.knockout;
    if knockout.width.0 * 5 > matrix_width || knockout.height.0 * 5 > matrix_width {
        return Err(RenderError::UnsafeLogoGeometry);
    }
    Ok(placement)
}

fn knockout_for_source(
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> Result<LogoKnockoutBounds, RenderError> {
    let padded_left = left
        .checked_sub(MODULE_UNITS)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let padded_top = top
        .checked_sub(MODULE_UNITS)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let right = left
        .checked_add(width)
        .and_then(|value| value.checked_add(MODULE_UNITS))
        .ok_or(RenderError::DimensionOverflow)?;
    let bottom = top
        .checked_add(height)
        .and_then(|value| value.checked_add(MODULE_UNITS))
        .ok_or(RenderError::DimensionOverflow)?;
    let knockout_left = padded_left / MODULE_UNITS;
    let knockout_top = padded_top / MODULE_UNITS;
    let knockout_right = div_ceil(right, MODULE_UNITS);
    let knockout_bottom = div_ceil(bottom, MODULE_UNITS);
    Ok(LogoKnockoutBounds {
        left: ModuleCoordinate(knockout_left),
        top: ModuleCoordinate(knockout_top),
        width: ModuleCoordinate(knockout_right - knockout_left),
        height: ModuleCoordinate(knockout_bottom - knockout_top),
    })
}

fn shifted_knockout(
    centered: LogoKnockoutBounds,
    offset_x: i32,
    offset_y: i32,
    matrix_width: u32,
) -> Option<LogoKnockoutBounds> {
    let left = i64::from(centered.left.0) + i64::from(offset_x);
    let top = i64::from(centered.top.0) + i64::from(offset_y);
    let right = left + i64::from(centered.width.0);
    let bottom = top + i64::from(centered.height.0);
    if left < 0 || top < 0 || right > i64::from(matrix_width) || bottom > i64::from(matrix_width) {
        return None;
    }
    Some(LogoKnockoutBounds {
        left: ModuleCoordinate(u32::try_from(left).ok()?),
        top: ModuleCoordinate(u32::try_from(top).ok()?),
        ..centered
    })
}

fn shifted_units(centered: u32, offset: i32) -> Result<u32, RenderError> {
    let shifted = i64::from(centered) + i64::from(offset) * i64::from(MODULE_UNITS);
    u32::try_from(shifted).map_err(|_| RenderError::UnsafeLogoGeometry)
}

fn analyze_knockout(
    matrix: &ModuleMatrix,
    knockout: LogoKnockoutBounds,
) -> Option<(u32, u32, u32)> {
    let mut data_count = 0_u32;
    let mut remainder_count = 0_u32;
    for y in knockout.top.0..knockout.top.0 + knockout.height.0 {
        for x in knockout.left.0..knockout.left.0 + knockout.width.0 {
            match matrix
                .module(u16::try_from(x).ok()?, u16::try_from(y).ok()?)?
                .kind()
            {
                ModuleKind::Data => data_count += 1,
                ModuleKind::Remainder => remainder_count += 1,
                _ => return None,
            }
        }
    }

    let mut clearance = u32::MAX;
    for y in 0..u32::from(matrix.size()) {
        for x in 0..u32::from(matrix.size()) {
            let module = matrix.module(u16::try_from(x).ok()?, u16::try_from(y).ok()?)?;
            if matches!(module.kind(), ModuleKind::Data | ModuleKind::Remainder) {
                continue;
            }
            let horizontal_gap = axis_gap(x, knockout.left.0, knockout.width.0);
            let vertical_gap = axis_gap(y, knockout.top.0, knockout.height.0);
            clearance = clearance.min(horizontal_gap.max(vertical_gap));
        }
    }
    Some((data_count, remainder_count, clearance))
}

fn axis_gap(point: u32, start: u32, length: u32) -> u32 {
    if point < start {
        start - point - 1
    } else {
        point.saturating_sub(start + length)
    }
}

const fn div_ceil(value: u32, divisor: u32) -> u32 {
    value / divisor + if value.is_multiple_of(divisor) { 0 } else { 1 }
}

#[must_use]
pub(crate) fn logo_contains_source_point(x: f64, y: f64) -> bool {
    let in_o_outer = (191.6667..=383.3334).contains(&x) && (192.6667..=409.3334).contains(&y);
    let in_o_hole = (237.5..=337.5).contains(&x) && (234.3334..=367.6667).contains(&y);
    let e = [
        (808.3333, 234.3333),
        (808.3333, 192.6667),
        (641.6667, 192.6667),
        (641.6667, 409.3333),
        (808.3333, 409.3333),
        (808.3333, 367.6667),
        (687.5, 367.6667),
        (687.5, 321.8333),
        (808.3333, 321.8333),
        (808.3333, 280.1667),
        (687.5, 280.1667),
        (687.5, 234.3333),
    ];
    let n = [
        (566.6667, 334.3333),
        (454.1667, 192.6667),
        (412.5, 192.6667),
        (412.5, 409.3333),
        (458.3333, 409.3333),
        (458.3333, 267.6667),
        (570.8333, 409.3333),
        (612.5, 409.3333),
        (612.5, 192.6667),
        (566.6667, 192.6667),
    ];
    (in_o_outer && !in_o_hole) || point_in_polygon(x, y, &e) || point_in_polygon(x, y, &n)
}

fn point_in_polygon(x: f64, y: f64, points: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let (current_x, current_y) = points[current];
        let (previous_x, previous_y) = points[previous];
        if (current_y > y) != (previous_y > y)
            && x < (previous_x - current_x) * (y - current_y) / (previous_y - current_y) + current_x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

#[must_use]
pub(crate) const fn source_view_box() -> (u32, u32) {
    (SOURCE_VIEW_BOX_WIDTH, SOURCE_VIEW_BOX_HEIGHT)
}
