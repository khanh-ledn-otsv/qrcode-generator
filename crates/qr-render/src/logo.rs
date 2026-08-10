use qr_core::Version;
use qr_core::matrix::{ModuleKind, ModuleMatrix};

use crate::{ProfileId, RenderError};

pub const BUNDLED_LOGO_SVG: &str = include_str!("../../../assets/RGB-one-lettermark-magenta.svg");

// PNG rendering uses a compact, compiled hit test for this exact sanitized SVG.
// Pinning the bundled bytes makes an asset edit fail at compile time until that
// geometry has been deliberately regenerated and independently audited.
const BUNDLED_LOGO_FNV1A64: u64 = fnv1a64(BUNDLED_LOGO_SVG.as_bytes());
const EXPECTED_BUNDLED_LOGO_FNV1A64: u64 = 0xecc8_cea6_484e_3bc8;
const _: () = assert!(BUNDLED_LOGO_FNV1A64 == EXPECTED_BUNDLED_LOGO_FNV1A64);

const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

pub(crate) fn bundled_logo_body() -> Result<&'static str, RenderError> {
    let root_end = BUNDLED_LOGO_SVG
        .find('>')
        .and_then(|index| index.checked_add(1))
        .ok_or(RenderError::RenderFailure)?;
    let close_start = BUNDLED_LOGO_SVG
        .rfind("</svg>")
        .ok_or(RenderError::RenderFailure)?;
    BUNDLED_LOGO_SVG
        .get(root_end..close_start)
        .ok_or(RenderError::RenderFailure)
}

const SOURCE_VIEW_BOX_LEFT: u32 = 180;
const SOURCE_VIEW_BOX_TOP: u32 = 180;
const SOURCE_VIEW_BOX_WIDTH: u32 = 640;
const SOURCE_VIEW_BOX_HEIGHT: u32 = 240;
const TEN_THOUSANDTHS_PER_MODULE: u32 = 10_000;
const SOURCE_WIDTH_TEN_THOUSANDTHS: u32 = 130_000;
const MINIMUM_ADAPTIVE_SOURCE_WIDTH_TEN_THOUSANDTHS: u32 = 100_000;

pub const BRANDED_LOGO_VERSION: Version = match Version::new(6) {
    Ok(version) => version,
    Err(_) => panic!("the approved branded logo version must be a valid QR version"),
};

/// Highest adaptive logo version backed by the committed decode campaign.
pub const MAXIMUM_ADAPTIVE_LOGO_VERSION: Version = match Version::new(11) {
    Ok(version) => version,
    Err(_) => panic!("the approved adaptive logo maximum must be a valid QR version"),
};

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
    left_ten_thousandths: u32,
    top_ten_thousandths: u32,
    width_ten_thousandths: u32,
    height_ten_thousandths: u32,
}

impl LogoSourceBounds {
    #[must_use]
    pub const fn left_ten_thousandths(self) -> u32 {
        self.left_ten_thousandths
    }

    #[must_use]
    pub const fn top_ten_thousandths(self) -> u32 {
        self.top_ten_thousandths
    }

    #[must_use]
    pub const fn width_ten_thousandths(self) -> u32 {
        self.width_ten_thousandths
    }

    #[must_use]
    pub const fn height_ten_thousandths(self) -> u32 {
        self.height_ten_thousandths
    }

    #[must_use]
    pub const fn right_ten_thousandths(self) -> u32 {
        self.left_ten_thousandths + self.width_ten_thousandths
    }

    #[must_use]
    pub const fn bottom_ten_thousandths(self) -> u32 {
        self.top_ten_thousandths + self.height_ten_thousandths
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoPlacement {
    source: LogoSourceBounds,
    knockout: LogoKnockoutBounds,
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
    profile_id: ProfileId,
) -> Result<LogoPlacement, RenderError> {
    if profile_id == ProfileId::Adaptive {
        return calculate_adaptive_logo_placement(matrix);
    }
    if matrix.version() != BRANDED_LOGO_VERSION {
        return Err(RenderError::UnsafeLogoGeometry);
    }
    centered_logo_placement(matrix, SOURCE_WIDTH_TEN_THOUSANDTHS)
}

fn centered_logo_placement(
    matrix: &ModuleMatrix,
    source_width: u32,
) -> Result<LogoPlacement, RenderError> {
    let matrix_width = u32::from(matrix.size());
    let source_height = source_width
        .checked_mul(SOURCE_VIEW_BOX_HEIGHT)
        .and_then(|height| height.checked_div(SOURCE_VIEW_BOX_WIDTH))
        .ok_or(RenderError::DimensionOverflow)?;
    let centered_left = matrix_width
        .checked_mul(TEN_THOUSANDTHS_PER_MODULE)
        .and_then(|width| width.checked_sub(source_width))
        .map(|difference| difference / 2)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let centered_top = matrix_width
        .checked_mul(TEN_THOUSANDTHS_PER_MODULE)
        .and_then(|width| width.checked_sub(source_height))
        .map(|difference| difference / 2)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    placement_for_source(matrix, centered_left, centered_top, source_width)
}

fn calculate_adaptive_logo_placement(matrix: &ModuleMatrix) -> Result<LogoPlacement, RenderError> {
    if matrix.version() < BRANDED_LOGO_VERSION || matrix.version() > MAXIMUM_ADAPTIVE_LOGO_VERSION {
        return Err(RenderError::UnsafeLogoGeometry);
    }
    let matrix_width = u32::from(matrix.size());
    let matrix_width_units = matrix_width
        .checked_mul(TEN_THOUSANDTHS_PER_MODULE)
        .ok_or(RenderError::DimensionOverflow)?;
    let maximum_offset = i32::try_from(matrix_width).map_err(|_| RenderError::DimensionOverflow)?;
    let mut source_width = SOURCE_WIDTH_TEN_THOUSANDTHS;

    loop {
        let source_height = source_width
            .checked_mul(SOURCE_VIEW_BOX_HEIGHT)
            .and_then(|height| height.checked_div(SOURCE_VIEW_BOX_WIDTH))
            .ok_or(RenderError::DimensionOverflow)?;
        let centered_left = matrix_width_units
            .checked_sub(source_width)
            .map(|difference| difference / 2)
            .ok_or(RenderError::UnsafeLogoGeometry)?;
        let centered_top = matrix_width_units
            .checked_sub(source_height)
            .map(|difference| difference / 2)
            .ok_or(RenderError::UnsafeLogoGeometry)?;
        let mut best: Option<((u64, u8, u32, u32), LogoPlacement)> = None;

        for vertical_offset in -maximum_offset..=maximum_offset {
            for horizontal_offset in -maximum_offset..=maximum_offset {
                let Some(left) = shifted_coordinate(centered_left, horizontal_offset) else {
                    continue;
                };
                let Some(top) = shifted_coordinate(centered_top, vertical_offset) else {
                    continue;
                };
                if left
                    .checked_add(source_width)
                    .is_none_or(|right| right > matrix_width_units)
                    || top
                        .checked_add(source_height)
                        .is_none_or(|bottom| bottom > matrix_width_units)
                {
                    continue;
                }
                let Ok(placement) = placement_for_source(matrix, left, top, source_width) else {
                    continue;
                };
                let score = placement_score(horizontal_offset, vertical_offset);
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _)| score < *best_score)
                {
                    best = Some((score, placement));
                }
            }
        }
        if let Some((_, placement)) = best {
            return Ok(placement);
        }
        if source_width == MINIMUM_ADAPTIVE_SOURCE_WIDTH_TEN_THOUSANDTHS {
            break;
        }
        source_width = source_width
            .checked_sub(TEN_THOUSANDTHS_PER_MODULE)
            .ok_or(RenderError::DimensionOverflow)?;
    }
    Err(RenderError::UnsafeLogoGeometry)
}

fn shifted_coordinate(coordinate: u32, offset_modules: i32) -> Option<u32> {
    let offset = i64::from(offset_modules) * i64::from(TEN_THOUSANDTHS_PER_MODULE);
    u32::try_from(i64::from(coordinate) + offset).ok()
}

fn placement_score(horizontal_offset: i32, vertical_offset: i32) -> (u64, u8, u32, u32) {
    let horizontal = horizontal_offset.unsigned_abs();
    let vertical = vertical_offset.unsigned_abs();
    let distance =
        u64::from(horizontal) * u64::from(horizontal) + u64::from(vertical) * u64::from(vertical);
    let direction = match (horizontal_offset, vertical_offset) {
        (0, value) if value < 0 => 0,
        (0, value) if value > 0 => 1,
        (value, 0) if value < 0 => 2,
        (value, 0) if value > 0 => 3,
        (_, value) if value < 0 => 4,
        _ => 5,
    };
    (distance, direction, vertical, horizontal)
}

fn placement_for_source(
    matrix: &ModuleMatrix,
    source_left: u32,
    source_top: u32,
    source_width: u32,
) -> Result<LogoPlacement, RenderError> {
    let matrix_width = u32::from(matrix.size());
    let source_height = source_width
        .checked_mul(SOURCE_VIEW_BOX_HEIGHT)
        .and_then(|height| height.checked_div(SOURCE_VIEW_BOX_WIDTH))
        .ok_or(RenderError::DimensionOverflow)?;
    let knockout_padding = TEN_THOUSANDTHS_PER_MODULE;
    let knockout = knockout_for_source(
        source_left,
        source_top,
        source_width,
        source_height,
        knockout_padding,
    )?;

    let (data_count, remainder_count, clearance) =
        analyze_knockout(matrix, knockout).ok_or(RenderError::UnsafeLogoGeometry)?;
    let placement = LogoPlacement {
        source: LogoSourceBounds {
            left_ten_thousandths: source_left,
            top_ten_thousandths: source_top,
            width_ten_thousandths: source_width,
            height_ten_thousandths: source_height,
        },
        knockout,
        protected_clearance: clearance,
        obscured_data_modules: data_count,
        obscured_remainder_modules: remainder_count,
    };
    let knockout = placement.knockout;
    if knockout.width.0 * 5 > matrix_width * 2 || knockout.height.0 * 5 > matrix_width * 2 {
        return Err(RenderError::UnsafeLogoGeometry);
    }
    Ok(placement)
}

fn knockout_for_source(
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    padding: u32,
) -> Result<LogoKnockoutBounds, RenderError> {
    let padded_left = left
        .checked_sub(padding)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let padded_top = top
        .checked_sub(padding)
        .ok_or(RenderError::UnsafeLogoGeometry)?;
    let right = left
        .checked_add(width)
        .and_then(|value| value.checked_add(padding))
        .ok_or(RenderError::DimensionOverflow)?;
    let bottom = top
        .checked_add(height)
        .and_then(|value| value.checked_add(padding))
        .ok_or(RenderError::DimensionOverflow)?;
    let knockout_left = padded_left / TEN_THOUSANDTHS_PER_MODULE;
    let knockout_top = padded_top / TEN_THOUSANDTHS_PER_MODULE;
    let knockout_right = div_ceil(right, TEN_THOUSANDTHS_PER_MODULE);
    let knockout_bottom = div_ceil(bottom, TEN_THOUSANDTHS_PER_MODULE);
    Ok(LogoKnockoutBounds {
        left: ModuleCoordinate(knockout_left),
        top: ModuleCoordinate(knockout_top),
        width: ModuleCoordinate(knockout_right - knockout_left),
        height: ModuleCoordinate(knockout_bottom - knockout_top),
    })
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
pub(crate) const fn source_view_box() -> (u32, u32, u32, u32) {
    (
        SOURCE_VIEW_BOX_LEFT,
        SOURCE_VIEW_BOX_TOP,
        SOURCE_VIEW_BOX_WIDTH,
        SOURCE_VIEW_BOX_HEIGHT,
    )
}
