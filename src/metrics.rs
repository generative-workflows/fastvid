#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityMetrics {
    pub samples: usize,
    pub max_error: u8,
    pub mse: f64,
    pub psnr_db: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityMetrics16 {
    pub samples: usize,
    pub max_error: u16,
    pub mse: f64,
    pub psnr_db: f64,
}

pub fn compare_plane(reference: &[u8], decoded: &[u8]) -> Option<QualityMetrics> {
    if reference.len() != decoded.len() || reference.is_empty() {
        return None;
    }
    let mut squared_error = 0u64;
    let mut max_error = 0u8;
    for (&expected, &actual) in reference.iter().zip(decoded) {
        let difference = expected.abs_diff(actual);
        max_error = max_error.max(difference);
        squared_error += u64::from(difference) * u64::from(difference);
    }
    let mse = squared_error as f64 / reference.len() as f64;
    let psnr_db = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };
    Some(QualityMetrics {
        samples: reference.len(),
        max_error,
        mse,
        psnr_db,
    })
}

pub fn compare_plane16(
    reference: &[u16],
    decoded: &[u16],
    bit_depth: u8,
) -> Option<QualityMetrics16> {
    if reference.len() != decoded.len() || reference.is_empty() {
        return None;
    }
    let peak = metric_peak(bit_depth)?;
    let mut squared_error = 0u64;
    let mut max_error = 0u16;
    for (&expected, &actual) in reference.iter().zip(decoded) {
        if expected > peak || actual > peak {
            return None;
        }
        let difference = expected.abs_diff(actual);
        max_error = max_error.max(difference);
        squared_error = squared_error.checked_add(u64::from(difference) * u64::from(difference))?;
    }
    let mse = squared_error as f64 / reference.len() as f64;
    let psnr_db = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (f64::from(peak).powi(2) / mse).log10()
    };
    Some(QualityMetrics16 {
        samples: reference.len(),
        max_error,
        mse,
        psnr_db,
    })
}

/// Computes mean 8x8-window SSIM with the standard 8-bit stabilization
/// constants. Edge windows are clipped to the plane dimensions.
pub fn ssim_plane(reference: &[u8], decoded: &[u8], width: usize, height: usize) -> Option<f64> {
    if width == 0
        || height == 0
        || width.checked_mul(height)? != reference.len()
        || reference.len() != decoded.len()
    {
        return None;
    }
    const WINDOW: usize = 8;
    const C1: f64 = 6.5025; // (0.01 * 255)^2
    const C2: f64 = 58.5225; // (0.03 * 255)^2
    let mut score_sum = 0.0;
    let mut window_count = 0usize;
    for top in (0..height).step_by(WINDOW) {
        for left in (0..width).step_by(WINDOW) {
            let bottom = (top + WINDOW).min(height);
            let right = (left + WINDOW).min(width);
            let samples = (bottom - top) * (right - left);
            let mut reference_sum = 0.0;
            let mut decoded_sum = 0.0;
            let mut reference_squared_sum = 0.0;
            let mut decoded_squared_sum = 0.0;
            let mut product_sum = 0.0;
            for y in top..bottom {
                for x in left..right {
                    let expected = f64::from(reference[y * width + x]);
                    let actual = f64::from(decoded[y * width + x]);
                    reference_sum += expected;
                    decoded_sum += actual;
                    reference_squared_sum += expected * expected;
                    decoded_squared_sum += actual * actual;
                    product_sum += expected * actual;
                }
            }
            let count = samples as f64;
            let reference_mean = reference_sum / count;
            let decoded_mean = decoded_sum / count;
            let reference_variance =
                (reference_squared_sum / count - reference_mean * reference_mean).max(0.0);
            let decoded_variance =
                (decoded_squared_sum / count - decoded_mean * decoded_mean).max(0.0);
            let covariance = product_sum / count - reference_mean * decoded_mean;
            let numerator = (2.0 * reference_mean * decoded_mean + C1) * (2.0 * covariance + C2);
            let denominator = (reference_mean * reference_mean + decoded_mean * decoded_mean + C1)
                * (reference_variance + decoded_variance + C2);
            score_sum += numerator / denominator;
            window_count += 1;
        }
    }
    Some(score_sum / window_count as f64)
}

pub fn ssim_plane16(
    reference: &[u16],
    decoded: &[u16],
    width: usize,
    height: usize,
    bit_depth: u8,
) -> Option<f64> {
    let peak = f64::from(metric_peak(bit_depth)?);
    if width == 0
        || height == 0
        || width.checked_mul(height)? != reference.len()
        || reference.len() != decoded.len()
        || reference
            .iter()
            .chain(decoded)
            .any(|&sample| f64::from(sample) > peak)
    {
        return None;
    }
    const WINDOW: usize = 8;
    let c1 = (0.01 * peak).powi(2);
    let c2 = (0.03 * peak).powi(2);
    let mut score_sum = 0.0;
    let mut window_count = 0usize;
    for top in (0..height).step_by(WINDOW) {
        for left in (0..width).step_by(WINDOW) {
            let bottom = (top + WINDOW).min(height);
            let right = (left + WINDOW).min(width);
            let count = ((bottom - top) * (right - left)) as f64;
            let mut reference_sum = 0.0;
            let mut decoded_sum = 0.0;
            let mut reference_squared_sum = 0.0;
            let mut decoded_squared_sum = 0.0;
            let mut product_sum = 0.0;
            for y in top..bottom {
                for x in left..right {
                    let expected = f64::from(reference[y * width + x]);
                    let actual = f64::from(decoded[y * width + x]);
                    reference_sum += expected;
                    decoded_sum += actual;
                    reference_squared_sum += expected * expected;
                    decoded_squared_sum += actual * actual;
                    product_sum += expected * actual;
                }
            }
            let reference_mean = reference_sum / count;
            let decoded_mean = decoded_sum / count;
            let reference_variance =
                (reference_squared_sum / count - reference_mean * reference_mean).max(0.0);
            let decoded_variance =
                (decoded_squared_sum / count - decoded_mean * decoded_mean).max(0.0);
            let covariance = product_sum / count - reference_mean * decoded_mean;
            score_sum += ((2.0 * reference_mean * decoded_mean + c1) * (2.0 * covariance + c2))
                / ((reference_mean * reference_mean + decoded_mean * decoded_mean + c1)
                    * (reference_variance + decoded_variance + c2));
            window_count += 1;
        }
    }
    Some(score_sum / window_count as f64)
}

fn metric_peak(bit_depth: u8) -> Option<u16> {
    match bit_depth {
        8 | 10 | 12 => Some((1u16 << bit_depth) - 1),
        16 => Some(u16::MAX),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_planes_have_infinite_psnr() {
        let metrics = compare_plane(&[0, 1, 255], &[0, 1, 255]).unwrap();
        assert_eq!(metrics.max_error, 0);
        assert_eq!(metrics.mse, 0.0);
        assert!(metrics.psnr_db.is_infinite());
    }

    #[test]
    fn exact_planes_have_unit_ssim() {
        let plane: Vec<u8> = (0..117).map(|value| (value * 17) as u8).collect();
        let score = ssim_plane(&plane, &plane, 13, 9).unwrap();
        assert!((score - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ssim_rejects_dimension_mismatches_and_detects_error() {
        let reference = vec![128; 64];
        let decoded = vec![120; 64];
        assert!(ssim_plane(&reference, &decoded, 8, 8).unwrap() < 1.0);
        assert!(ssim_plane(&reference, &decoded, 7, 8).is_none());
    }

    #[test]
    fn high_bit_metrics_use_the_signaled_peak() {
        let exact = [0, 512, 1023];
        let metrics = compare_plane16(&exact, &exact, 10).unwrap();
        assert!(metrics.psnr_db.is_infinite());
        assert_eq!(metrics.max_error, 0);
        assert!((ssim_plane16(&exact, &exact, 3, 1, 10).unwrap() - 1.0).abs() < 1e-12);

        let decoded = [1, 511, 1022];
        let metrics = compare_plane16(&exact, &decoded, 10).unwrap();
        assert_eq!(metrics.max_error, 1);
        assert!((metrics.mse - 1.0).abs() < f64::EPSILON);
        assert!((metrics.psnr_db - 60.197_512_674_243_2).abs() < 1e-10);
        assert!(compare_plane16(&[1024], &[0], 10).is_none());
    }
}
