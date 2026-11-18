//! How often each rule of a Player fired, for runners and tests. Counting never changes a choice.

pub const MAX_RULES: usize = 40;

#[derive(Clone, Debug)]
pub struct Fired {
    pub names: &'static [&'static str],
    pub n: [u64; MAX_RULES],
}

impl Fired {
    pub fn new(names: &'static [&'static str]) -> Fired {
        assert!(names.len() <= MAX_RULES);
        Fired { names, n: [0; MAX_RULES] }
    }
    #[inline]
    pub fn hit(&mut self, i: usize) {
        self.n[i] += 1;
    }
    pub fn get(&self, name: &str) -> u64 {
        self.names.iter().position(|x| *x == name).map_or(0, |i| self.n[i])
    }
    pub fn list(&self) -> Vec<(&'static str, u64)> {
        self.names.iter().enumerate().map(|(i, s)| (*s, self.n[i])).collect()
    }
}
