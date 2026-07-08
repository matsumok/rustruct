use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeismicModel {
    pub name: String,
    pub story_masses: Vec<f64>,
    pub story_stiffnesses: Vec<f64>,
    pub story_heights: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let model = SeismicModel {
            name: "Sample".to_string(),
            story_masses: vec![1.0, 2.0, 3.0],
            story_stiffnesses: vec![4.0, 5.0, 6.0],
            story_heights: vec![7.0, 8.0, 9.0],
        };

        let json = dbg!(serde_json::to_string(&model).unwrap());
        let restored: SeismicModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, restored);
    }
}
