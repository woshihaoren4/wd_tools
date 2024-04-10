mod to_str;

pub use to_str::*;
use std::any::{TypeId};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(dead_code)]
pub fn unsafe_take<A,B>(a:&mut Option<A>)-> Option<B>{
    unsafe {
        let b = &mut *(a as *mut Option<A> as *mut Option<B>);
        b.take()
    }
}

#[allow(dead_code)]
pub fn unsafe_downcast<A: ?Sized,B>(a: Box<A>)-> Box<B>{
    unsafe {
        let raw = Box::into_raw(a) as *mut B;
        Box::from_raw(raw)
    }
}

#[allow(dead_code)]
pub fn unsafe_must_take<A,B>(a:A)-> B{
    unsafe_take(&mut Some(a)).unwrap()
}

#[allow(dead_code)]
pub fn unsafe_must_downcast<A,B>(a: A)-> B{
    *unsafe_downcast(Box::new(a))
}

#[allow(dead_code)]
pub fn type_id<T: 'static>() -> u64 {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<T>().hash(&mut hasher);
    hasher.finish()
}


#[cfg(test)]
mod test{
    use crate::ptr::{unsafe_must_downcast, unsafe_must_take};

    //cargo test --color=always --package wd_tools --lib ptr::test::test_unsafe_must_take --no-fail-fast --  --exact --show-output
    #[test]
    fn test_unsafe_must_take(){
        let a:usize = 1;
        let b:i64 = unsafe_must_take(a);
        println!("---> {b}");
    }
    //cargo test --color=always --package wd_tools --lib ptr::test::test_unsafe_must_downcast --no-fail-fast --  --exact --show-output
    #[test]
    fn test_unsafe_must_downcast(){
        let a:usize = 1;
        let b:i64 = unsafe_must_downcast(a);
        println!("---> {b}");
    }
}

