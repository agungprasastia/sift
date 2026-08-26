pub struct Session {
    pub id: String,
}

pub fn cleanup_session(session: &Session) {
    drop(session);
}
