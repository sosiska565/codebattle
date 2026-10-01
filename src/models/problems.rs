use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct Solution {
    pub args: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct Problems {
    pub text: String,
    pub solutions: Vec<Solution>,
}
