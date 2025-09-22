#[derive(Debug)]
pub enum NinjaParseError {
    FormatError(&'static str),
    IoError(),
}

impl From<std::io::Error> for NinjaParseError {
    fn from(_value: std::io::Error) -> Self {
        Self::IoError()
    }
}
