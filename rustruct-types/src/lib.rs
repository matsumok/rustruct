//! 共有ドメイン型クレート
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
/// 階のデータ
pub struct Story {
    /// 質量 ton
    pub mass: f64,
    /// 剛性 kN/m
    pub stiffness: f64,
    /// 高さ m
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// 解析モデル
pub struct SeismicModel {
    /// モデル名
    pub name: String,
    /// 階データ　最下層がindex0
    pub stories: Vec<Story>,
}

impl SeismicModel {
    pub fn story_count(&self) -> usize {
        self.stories.len()
    }
    pub fn add_story(&mut self, story: Story) {
        self.stories.push(story);
    }
    pub fn remove_story(&mut self, index: usize) {
        self.stories.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let model = SeismicModel {
            name: "Sample".to_string(),
            stories: vec![
                Story {
                    mass: 1.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
                Story {
                    mass: 1.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
                Story {
                    mass: 1.0,
                    stiffness: 5.0,
                    height: 4.0,
                },
            ],
        };

        let json = dbg!(serde_json::to_string(&model).unwrap());
        let restored: SeismicModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, restored);
    }
}
