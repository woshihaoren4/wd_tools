use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Mutex, Once};

static mut VARS:Option<HashMap<TypeId,Box<dyn Any>>> = None;
static mut VARS_LOCK: Mutex<()> = Mutex::new(());
static START: Once = Once::new();

pub fn get()-> &'static mut HashMap<TypeId,Box<dyn Any>>{
    START.call_once(||{
        unsafe {
            VARS = Some(HashMap::new());
        }
    });
    unsafe {
        if let Some(ref mut s) = VARS{
            return s
        }else{
            panic!("wd_tool VARS not init!!!");
        }
    }
}

pub fn unsafe_init<T:Any>(t:T){
    let id = t.type_id();
    get().insert(id,Box::new(t));
}

pub fn init<T:Any>(t:T){
    unsafe {
        let _lock = VARS_LOCK.lock().unwrap();
        unsafe_init(t);
    }
}

pub fn unsafe_fetch<T:Any,Out>(handle:impl FnOnce(Option<&mut T>)->Out)->Out{
    let id = TypeId::of::<T>();
    let t = if let Some(s) = get().get_mut(&id){
        s
    }else{
        return handle(None)
    };
    let t = t.downcast_mut::<T>();
    handle(t)
}


#[cfg(test)]
mod test{

    #[test]
    fn test_vars(){
        super::init(1u32);
        let i = super::unsafe_fetch(|f|{
            if let Some(s) = f{
                *s += 1;
                *s
            }else{
                0u32
            }
        });
        assert_eq!(2,i);
        let i = super::unsafe_fetch(|f|{
            if let Some(s) = f{
                *s += 1;
                *s
            }else{
                0u32
            }
        });
        assert_eq!(3,i);
    }
}