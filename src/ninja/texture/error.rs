use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum NinjaTexReadError {
    FormatError(&'static str),
    IoError(),
}

impl From<ddsfile::Error> for NinjaTexReadError {
    fn from(_value: ddsfile::Error) -> Self {
        Self::FormatError("invalid DDS file!")
    }
}

impl From<std::io::Error> for NinjaTexReadError {
    fn from(_value: std::io::Error) -> Self {
        Self::IoError()
    }
}

impl From<FromUtf8Error> for NinjaTexReadError {
    fn from(_value: FromUtf8Error) -> Self {
        Self::FormatError("Failed to parse UTF-8 in string")
    }
}
