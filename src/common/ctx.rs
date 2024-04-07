use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

#[derive(Default,Clone)]
pub struct Ctx {
    map:Arc<RwLock<HashMap<String,Box<dyn Any+Send+Sync>>>>
}
impl Ctx {
    #[allow(dead_code)]
    pub fn insert<K:Into<String>,V:Any+ Send+Sync>(&self,key:K,val:V)->Option<Box<dyn Any+Send+Sync>>{
        self.ref_inner_mut(|map|{
            map.insert(key.into(),Box::new(val))
        })
    }
    #[allow(dead_code)]
    pub fn remove<K:AsRef<str>,V:Any>(&self,key:K)->Option<V>{
        self.ref_inner_mut(|map|{
            if let Some(s) = map.get(key.as_ref()) {
                if s.downcast_ref::<V>().is_none() {
                    return None
                }
            }else{
                return None
            }
            if let Some(s) = map.remove(key.as_ref()){
                let val = Box::into_raw(s) as *mut V;
                unsafe {
                    let a = Box::from_raw(val);
                    return Some(*a)
                }
            }
            None
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle<K:AsRef<str>,V:Any,O>(&self,key:K,handle:impl FnOnce(Option<&V>)->O)->O{
        self.ref_inner(|map|{
            let opt = map.get(key.as_ref());
            let res = match opt {
                None => None,
                Some(a) => {
                    a.downcast_ref::<V>()
                }
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle_mut<K:AsRef<str>,V:Any,O>(&self,key:K,handle:impl FnOnce(Option<&mut V>)->O)->O{
        self.ref_inner_mut(|map|{
            let opt = map.get_mut(key.as_ref());
            let res = match opt {
                None => None,
                Some(a) => {
                    a.downcast_mut::<V>()
                }
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_inner<O>(&self, handle: impl FnOnce(&HashMap<String, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let reader = self.map.read().unwrap();
        handle(reader.deref())
    }
    #[allow(dead_code)]
    pub fn ref_inner_mut<O>(&self, handle: impl FnOnce(&mut HashMap<String, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let mut write = self.map.write().unwrap();
        handle(write.deref_mut())
    }
}




#[cfg(test)]
mod test{
    use std::time::Duration;
    use crate::common::ctx::{Ctx};

    #[tokio::test]
    async fn test_context(){
        let ctx = Ctx::default();
        ctx.insert("hello",true);
        let c = ctx.clone();
        tokio::spawn(async move {
            let x = c.remove::<_,bool>("hello").unwrap();
            assert_eq!(x,true);
        });

        tokio::time::sleep(Duration::from_secs(1)).await;
        let res = ctx.remove::<_,bool>("hello");
        assert_eq!(None,res)
    }
}