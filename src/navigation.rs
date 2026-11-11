/// Ephemeral page navigation, independent of downloads and saved records.
pub struct History<T> {
    back: Vec<T>,
    forward: Vec<T>,
}
impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            back: Vec::new(),
            forward: Vec::new(),
        }
    }
}
impl<T> History<T> {
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }
    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
    pub fn push(&mut self, current: T) {
        self.back.push(current);
        if self.back.len() > 24 {
            self.back.remove(0);
        }
        self.forward.clear();
    }
    pub fn back(&mut self, current: T) -> Option<T> {
        let previous = self.back.pop()?;
        self.forward.push(current);
        Some(previous)
    }
    pub fn forward(&mut self, current: T) -> Option<T> {
        let next = self.forward.pop()?;
        self.back.push(current);
        Some(next)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returning_then_branching_discards_the_old_forward_path() {
        let mut h = History::default();
        for i in 0..30 {
            h.push(i);
        }
        assert_eq!(h.back.len(), 24);
        assert_eq!(h.back(30), Some(29));
        assert_eq!(h.forward(29), Some(30));
        assert_eq!(h.back(30), Some(29));
        h.push(29);
        assert!(!h.can_forward());
        assert_eq!(h.back(99), Some(29));
    }
}
