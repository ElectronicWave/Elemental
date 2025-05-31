use std::io::{self, Error, ErrorKind};

pub trait UnifiedResult<T> {
    fn to_stdio(self) -> io::Result<T>;
}

impl<T, E: ToString> UnifiedResult<T> for Result<T, E> {
    fn to_stdio(self) -> io::Result<T> {
        self.map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
    }
}
