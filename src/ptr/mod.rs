use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

#[allow(dead_code)]
pub fn try_any_to_var<A:Any,T:Any>(any:A)->Result<T,A>{
    if TypeId::of::<A>() != TypeId::of::<T>() {
        return Err(any)
    }
    let mut any_opt = Some(any);
    unsafe {
        let opt = std::ptr::read(&*(&any_opt as *const Option<A> as *const Option<T>));
        std::ptr::write(&mut any_opt,None);
        match opt {
            None => panic!("force_box_to_var failed"),
            Some(s) => return Ok(s),
        }
    }
}
#[allow(dead_code)]
pub fn force_arc_to_var<A: ?Sized,T:Clone>(any:Arc<A>)-> T{
    let des = unsafe {
        let r = any.deref();
        let mut opt = Some(r);

        let des = std::ptr::read(&*(&opt as *const Option<&A> as *const Option<&T>));
        std::ptr::write(&mut opt,None);
        des
    };
    match des {
        None => panic!("force_arc_to_var failed"),
        Some(s) => return s.clone(),
    }
}
#[allow(dead_code)]
pub fn force_box_to_var<A: ?Sized ,T>(any:Box<A>) -> T{
    let mut any = Some(any);
    unsafe {
        let opt = std::ptr::read(&*(&any as *const Option<Box<A>> as *const Option<Box<T>>));
        std::ptr::write(&mut any,None);
        match opt {
            None => panic!("force_box_to_var failed"),
            Some(s) => return *s,
        }
    }
}
#[allow(dead_code)]
pub fn type_id<T:'static>()->u64{
    let mut hasher = DefaultHasher::new();
    TypeId::of::<T>().hash(&mut hasher);
    hasher.finish()
}