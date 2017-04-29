use std::sync::atomic::{AtomicUsize, Ordering};

/// An `AtomicBitSet` allowing
/// lock-free concurrent modification.
pub struct AtomicBitSet {
    data: Vec<AtomicUsize>,
}

impl AtomicBitSet {
    pub fn with_size(size: usize) -> Self {
        let elem_size: usize = ::std::mem::size_of::<usize>();
        let size = (size / elem_size) + 1;

        let mut data = Vec::with_capacity(size);

        for _ in 0..size {
            data.push(AtomicUsize::new(0));
        }

        AtomicBitSet { data: data }
    }

    pub fn get(&self, index: usize) -> bool {
        let (index, mask) = Self::mask(index);

        self.data[index].load(Ordering::Acquire) & mask != 0
    }

    pub fn set(&self, index: usize, value: bool) {
        let (index, mask) = Self::mask(index);

        if value {
            self.data[index].fetch_or(mask, Ordering::Release);
        } else {
            self.data[index].fetch_and(!mask, Ordering::Release);
        }
    }

    /// Sets every bit to `0`.
    pub fn clear(&self) {
        for elem in &self.data {
            elem.store(0, Ordering::Release);
        }
    }

    fn mask(index: usize) -> (usize, usize) {
        let elem_size: usize = ::std::mem::size_of::<usize>();

        let data_index = index / elem_size;
        let bit = index % elem_size;

        let mask = 1 << bit;

        (data_index, mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let running = AtomicBitSet::with_size(5);

        running.set(0, false);
        running.set(2, true);
        running.set(3, false);
        running.set(4, false);

        assert!(running.get(0) == false);
        assert!(running.get(1) == false);
        assert!(running.get(2) == true);
        assert!(running.get(3) == false);
        assert!(running.get(4) == false);

        running.set(4, true);

        assert!(running.get(4) == true);
    }
}
