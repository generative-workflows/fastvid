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
}
