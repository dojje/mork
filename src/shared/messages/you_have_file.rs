pub struct YouHaveFile {
    pub code: &'static str
}

impl YouHaveFile {
    pub fn new(code: &'static str) -> Self {
        Self {
            code
        }
    }
}
