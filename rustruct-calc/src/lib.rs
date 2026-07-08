use rustruct_types::SeismicModel;

pub fn story_drift_ratios(relative_displacements: &[f64], model: &SeismicModel) -> Vec<f64> {
    debug_assert_eq!(relative_displacements.len(), model.story_heights.len());
    let ratios: Vec<f64> = relative_displacements
        .iter()
        .zip(model.story_heights.iter())
        .map(|(d, h)| d / h)
        .collect();

    ratios
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: &[f64], right: &[f64]) {
        assert_eq!(left.len(), right.len());
        for (l, r) in left.iter().zip(right.iter()) {
            assert!((l - r).abs() < 1e-9, "left={left:?}, right={right:?}")
        }
    }

    #[test]
    fn test_story_deift_rations() {
        let model = SeismicModel {
            name: "Sample".to_string(),
            story_masses: vec![1.0, 2.0, 3.0],
            story_stiffnesses: vec![4.0, 5.0, 6.0],
            story_heights: vec![7.0, 8.0, 9.0],
        };
        let relative_displacements = vec![0.7, 0.8, 0.9];
        let result = story_drift_ratios(&relative_displacements, &model);
        assert_close(&result, &vec![0.1, 0.1, 0.1]);
    }
}
