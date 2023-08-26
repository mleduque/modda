
use std::fs::File;
use std::io::{BufRead, BufReader, Result};

pub struct BufReadRaw<T: BufRead + Sized>(T);

impl <T: BufRead + Sized> BufReadRaw<T> {
    pub fn new(source: T) -> BufReadRaw<T> {
        BufReadRaw(source)
    }
    pub fn from_file(source: File) -> BufReadRaw<BufReader<File>> {
        BufReadRaw::new(BufReader::new(source))
    }

    pub fn raw_lines(self) -> RawLines<Self>
    where
        Self: Sized,
    {
        RawLines { buf: self }
    }
}

#[derive(Debug)]
pub struct RawLines<B> {
    buf: B,
}

impl <T: BufRead + Sized> Iterator for  RawLines<BufReadRaw<T>> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.0.read_until(b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.ends_with(&[b'\n']) {
                    buf.pop();
                    if buf.ends_with(&[b'\r']) {
                        buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

