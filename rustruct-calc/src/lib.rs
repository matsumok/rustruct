use rustruct_types::SeismicModel;

pub fn story_drift_ratios(relative_displacements: &[f64], model: &SeismicModel) -> Vec<f64> {
    debug_assert_eq!(relative_displacements.len(), model.stories.len());
    let ratios: Vec<f64> = relative_displacements
        .iter()
        .zip(model.stories.iter())
        .map(|(d, s)| d / s.height)
        .collect();

    ratios
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustruct_types::Story;

    fn assert_close(left: &[f64], right: &[f64]) {
        assert_eq!(left.len(), right.len());
        for (l, r) in left.iter().zip(right.iter()) {
            assert!((l - r).abs() < 1e-9, "left={left:?}, right={right:?}")
        }
    }

    #[test]
    fn test_story_drift_ratios() {
        let model = SeismicModel {
            name: "Sample".to_string(),
            stories: vec![
                Story {
                    mass: 1.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
                Story {
                    mass: 2.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
                Story {
                    mass: 3.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
            ],
        };
        let relative_displacements = vec![0.2, 0.2, 0.2];
        let result = story_drift_ratios(&relative_displacements, &model);
        assert_close(&result, &vec![0.05, 0.05, 0.05]);
    }
}
