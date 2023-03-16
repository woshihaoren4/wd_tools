use std::sync::Arc;

/// Point-Free
pub trait PFOk<Err> {
    #[inline]
    fn ok(self) -> Result<Self, Err>
    where
        Self: Sized,
    {
        Ok(self)
    }
}

impl<Err, T> PFOk<Err> for T {}

pub trait PFErr<Ok> {
    #[inline]
    fn err(self) -> Result<Ok, Self>
    where
        Self: Sized,
    {
        Err(self)
    }
}

impl<Ok, T> PFErr<Ok> for T {}

pub trait PFArc<T> {
    fn arc(self) -> Arc<T>;
}

impl<T> PFArc<T> for T {
    #[inline]
    fn arc(self) -> Arc<T> {
        Arc::new(self)
    }
}

pub trait PFBox<T> {
    fn to_box(self) -> Box<T>;
}

impl<T> PFBox<T> for T {
    #[inline]
    fn to_box(self) -> Box<T> {
        Box::new(self)
    }
}

pub trait PFSome<T> {
    fn some(self) -> Option<T>;
}

impl<T> PFSome<T> for T {
    fn some(self) -> Option<T> {
        Some(self)
    }
}
