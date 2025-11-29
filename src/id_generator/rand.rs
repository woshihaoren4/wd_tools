use rand::distr;
use rand::distr::{Distribution, StandardUniform};

#[allow(unused)]
pub fn random<T>() -> T
where
    StandardUniform: Distribution<T>,
{
    rand::random::<T>()
}

#[allow(unused)]
pub fn random_in_between<T, R>(range: R) -> T
where
    T: distr::uniform::SampleUniform,
    R: distr::uniform::SampleRange<T>,
{
    rand::random_range::<T, _>(range)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_random() {
        let i = random::<u8>();
        println!("{}", i);
        let i = random::<u8>();
        println!("{}", i);
        let i = random_in_between::<u32, _>(2..4);
        println!("{}", i);
    }
}
