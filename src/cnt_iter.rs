pub(crate) struct CounterIterator<I> {
    inner: I,
    taken: usize,
}

impl<I> CounterIterator<I> {
    pub(crate) fn new(it: I) -> Self {
        Self {
            inner: it,
            taken: 0,
        }
    }

    pub(crate) fn taken(&self) -> usize {
        self.taken
    }
}

impl<T, I: Iterator<Item = T>> Iterator for CounterIterator<I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next();
        if ret.is_some() {
            self.taken += 1;
        }

        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_zero() {
        let ci = CounterIterator::new(std::iter::empty::<i32>());
        assert_eq!(ci.taken(), 0);
    }

    #[test]
    fn taken_increments_per_item() {
        let mut ci = CounterIterator::new(vec![10, 20, 30].into_iter());
        assert_eq!(ci.taken(), 0);
        assert_eq!(ci.next(), Some(10));
        assert_eq!(ci.taken(), 1);
        assert_eq!(ci.next(), Some(20));
        assert_eq!(ci.taken(), 2);
        assert_eq!(ci.next(), Some(30));
        assert_eq!(ci.taken(), 3);
    }

    #[test]
    fn taken_does_not_increment_after_exhausted() {
        let mut ci = CounterIterator::new(vec![1].into_iter());
        ci.next();
        assert_eq!(ci.taken(), 1);
        assert_eq!(ci.next(), None);
        assert_eq!(ci.taken(), 1);
    }
}
