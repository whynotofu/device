pub struct StackString<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> StackString<N> {
    pub fn new() -> Self {
        Self { bytes: [0; N] }
    }

    pub fn add(mut self, s: &str) -> Self {
        let source = s.as_bytes();
        let len = self.len();
        let new_len = len + source.len();
        assert!(new_len < N, "StackString<{}> capacity exceeded", N);
        self.bytes[len..new_len].copy_from_slice(source);
        self
    }

    pub fn len(&self) -> usize {
        self.bytes.iter().position(|&b| b == 0).unwrap_or(N)
    }

    pub fn trim_end(mut self) -> Self {
        for i in (0..N).rev() {
            if self.bytes[i] != 0 {
                if b" \n".contains(&self.bytes[i]) {
                    self.bytes[i] = 0;
                } else {
                    return self;
                }
            }
        }
        self
    }

    pub fn into_lowercase(mut self) -> Self {
        self.bytes.iter_mut().for_each(|c| *c = c.to_ascii_lowercase());
        self
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[0..self.len()]).expect("Invalid UTF-8")
    }
}

impl<const N: usize> From<&str> for StackString<N> {
    fn from(s: &str) -> Self {
        StackString::<N>::new().add(s)
    }
}

impl<const N: usize> From<u8> for StackString<N> {
    fn from(n: u8) -> Self {
        StackString::<N>::from(number_to_base_10_bytes::<N>(n as u32))
    }
}

impl<const N: usize> From<u32> for StackString<N> {
    fn from(n: u32) -> Self {
        StackString::<N>::from(number_to_base_10_bytes::<N>(n))
    }
}

fn number_to_base_10_bytes<const N: usize>(n: u32) -> [u8; N] {
    let mut l = 1;
    let mut n = n;
    let mut x = 9;
    while n > x && l < 10 {
        x = x * 10 + 9;
        l += 1;
    }
    let mut buffer = [0u8; N];
    let mut i = l;
    if n == 0 {
        i -= 1;
        buffer[i] = b'0';
    } else {
        while n > 0 && i > 0 {
            i -= 1;
            buffer[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    buffer
}

impl<const N: usize> From<[u8; N]> for StackString<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self { bytes }
    }
}
