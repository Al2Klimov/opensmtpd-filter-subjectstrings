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
