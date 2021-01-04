pub const KIBIBYTE: u64 = 1024;
pub const MEBIBYTE: u64 = KIBIBYTE * 1024;

fn fdivide(length: u64, unit: u64) -> f64 {
    length as f64 / unit as f64
}

pub enum FileSize {
    Int(u64),
    Float(f64),
}

impl FileSize {
    pub fn from(length: u64, unit: u64) -> Self {
        let result = fdivide(length, unit);
        if (result.trunc() - result).abs() < f64::EPSILON {
            Self::Int(result as u64)
        } else {
            Self::Float(result)
        }
    }
}
