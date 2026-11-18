use super::{SLIM_ARM_GAPS, SkinModel, SkinPixels};

fn texture(width: u16, height: u16, mut paint: impl FnMut(u16, u16) -> [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&paint(x, y));
        }
    }
    rgba
}

fn infer(width: u16, height: u16, rgba: &[u8]) -> Option<SkinModel> {
    SkinModel::infer(SkinPixels {
        width,
        height,
        rgba,
    })
}

#[test]
fn infers_classic_for_64x32_textures() {
    let rgba = texture(64, 32, |_, _| [64, 128, 192, 255]);
    assert_eq!(infer(64, 32, &rgba), Some(SkinModel::Classic));
}

#[test]
fn infers_classic_for_opaque_64x64_textures_without_gap_markers() {
    let rgba = texture(64, 64, |_, _| [64, 128, 192, 255]);
    assert_eq!(infer(64, 64, &rgba), Some(SkinModel::Classic));
}

#[test]
fn infers_slim_when_any_arm_gap_is_transparent() {
    for (gap_x, gap_y, gap_width, gap_height) in SLIM_ARM_GAPS {
        let rgba = texture(64, 64, |x, y| {
            let in_gap =
                (gap_x..gap_x + gap_width).contains(&x) && (gap_y..gap_y + gap_height).contains(&y);
            if in_gap {
                [64, 128, 192, 0]
            } else {
                [64, 128, 192, 255]
            }
        });
        assert_eq!(infer(64, 64, &rgba), Some(SkinModel::Slim));
    }
}

#[test]
fn infers_slim_for_solid_black_or_white_arm_gaps() {
    let black = texture(64, 64, |_, _| [0, 0, 0, 255]);
    let white = texture(64, 64, |_, _| [255, 255, 255, 255]);
    assert_eq!(infer(64, 64, &black), Some(SkinModel::Slim));
    assert_eq!(infer(64, 64, &white), Some(SkinModel::Slim));
}

#[test]
fn ignores_transparency_outside_arm_gaps() {
    let rgba = texture(64, 64, |x, y| {
        if (x, y) == (0, 0) {
            [255, 255, 255, 0]
        } else {
            [64, 128, 192, 255]
        }
    });
    assert_eq!(infer(64, 64, &rgba), Some(SkinModel::Classic));
}

#[test]
fn rejects_unsupported_dimensions_and_buffers() {
    let rgba = texture(64, 64, |_, _| [64, 128, 192, 255]);
    assert_eq!(infer(32, 32, &rgba[..32 * 32 * 4]), None);
    assert_eq!(infer(64, 64, &rgba[..rgba.len() - 1]), None);
}
