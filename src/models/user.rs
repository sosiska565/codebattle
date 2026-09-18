#[derive(Default, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub elo: i32,
    pub pass_hash: String,
    pub created_at: String,
    pub email: String,
}

impl User {}
