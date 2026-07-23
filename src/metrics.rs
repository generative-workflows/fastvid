#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityMetrics {
    pub samples: usize,
    pub max_error: u8,
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
}
