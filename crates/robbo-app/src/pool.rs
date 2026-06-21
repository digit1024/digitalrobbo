/// Generic object pool for projectiles and particle effects.
pub struct Pool<T> {
    free: Vec<T>,
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self { free: Vec::new() }
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self
    where
        T: Default,
    {
        Self {
            free: (0..cap).map(|_| T::default()).collect(),
        }
    }

    pub fn acquire<F>(&mut self, factory: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.free.pop().unwrap_or_else(factory)
    }

    pub fn release(&mut self, item: T) {
        self.free.push(item);
    }

    pub fn len(&self) -> usize {
        self.free.len()
    }
}

#[derive(Default, Clone, Copy)]
pub struct PooledProjectile {
    pub active: bool,
}

#[derive(Default, Clone, Copy)]
pub struct PooledParticle {
    pub active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_recycles() {
        let mut pool = Pool::new();
        let a = pool.acquire(|| 1);
        pool.release(a);
        let b = pool.acquire(|| 2);
        assert_eq!(b, 1);
    }
}
