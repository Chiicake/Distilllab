pub struct AppRuntime {
    pub database_path: String,
}
impl AppRuntime {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }
}