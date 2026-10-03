use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Default, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub output: String,
}
