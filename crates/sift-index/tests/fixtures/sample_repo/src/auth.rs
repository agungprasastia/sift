pub struct User {
    pub name: String,
}

pub fn login(user: &User) -> bool {
    user.name == "admin"
}

pub fn authenticate(token: &str) -> bool {
    !token.is_empty()
}
