

pub enum RunResult {
    Dry(String),
    Real(std::process::Output)
}

impl RunResult {
    pub fn status_code(&self) -> Option<i32> {
        match self {
            RunResult::Dry(_) => Some(0),
            RunResult::Real(output) => output.status.code(),
        }
    }
    pub fn success(&self) -> bool {
        match self {
            RunResult::Dry(_) => true,
            RunResult::Real(output) => output.status.success(),
        }
    }
}
