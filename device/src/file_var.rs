use crate::{
    stack_string::StackString,
    wrappers::{File, FilePath, OpenMode},
};
use std::marker::PhantomData;

pub struct FileVariable<T> {
    file: File,
    mode: Mode,
    _marker: PhantomData<T>,
}

impl<T: ToAndFromFile> FileVariable<T> {
    pub fn new(file: &FilePath, mode: Mode) -> Self {
        let open_mode = match mode {
            Mode::ReadOnly => OpenMode::ReadOnly,
            Mode::ReadWrite => OpenMode::ReadWrite,
            Mode::Create => OpenMode::Create,
        };
        Self {
            file: File::open(file, open_mode),
            mode,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> T {
        self.file.seek(0);
        T::from_file(&self.file)
    }

    pub fn static_get(file: &FilePath) -> T {
        let file = File::open(file, OpenMode::ReadOnly);
        let value = T::from_file(&file);
        file.close();
        value
    }

    pub fn set(&self, value: &T) {
        if self.mode != Mode::ReadOnly {
            self.file.seek(0);
            T::to_file(&self.file, value);
        }
    }

    pub fn static_set(file: &FilePath, value: &T) {
        let file = File::open(file, OpenMode::ReadWrite);
        T::to_file(&file, value);
        file.close();
    }

    pub fn lock(&self) -> bool {
        self.file.lock()
    }
}

#[derive(PartialEq)]
pub enum Mode {
    ReadOnly,
    ReadWrite,
    Create,
}

pub trait ToAndFromFile {
    fn from_file(file: &File) -> Self;
    fn to_file(file: &File, value: &Self);
}

impl ToAndFromFile for u8 {
    fn from_file(file: &File) -> Self {
        let mut buf = [0u8; 3];
        file.read(&mut buf);
        StackString::<3>::from(buf).trim_end().as_str().parse::<u8>().unwrap()
    }

    fn to_file(file: &File, value: &Self) {
        file.write(StackString::<3>::from(*value).as_str().as_bytes());
    }
}

impl ToAndFromFile for u32 {
    fn from_file(file: &File) -> Self {
        let mut buf = [0u8; 10];
        file.read(&mut buf);
        StackString::<10>::from(buf).trim_end().as_str().parse::<u32>().unwrap()
    }

    fn to_file(file: &File, value: &Self) {
        file.write(StackString::<10>::from(*value).as_str().as_bytes());
    }
}

impl<const N: usize> ToAndFromFile for StackString<N> {
    fn from_file(file: &File) -> Self {
        let mut buf = [0u8; N];
        file.read(&mut buf);
        StackString::<N>::from(buf).trim_end()
    }

    fn to_file(file: &File, value: &Self) {
        file.write(value.as_str().as_bytes());
    }
}
