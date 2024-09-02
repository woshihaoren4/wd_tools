#[macro_export]
macro_rules! share {
    ($obj:tt) => {
impl AsyncMutex<$obj>{
    pub fn lock_ref_mut<T,Out>(&self,handle:T)->Out
    where T: FnOnce(&mut $obj) -> Out
    {
        let mut binding = self.synchronize();
        let target =std::ops::DerefMut::deref_mut(&mut binding);
        handle(target)
    }
    pub fn unsafe_mut_ptr<T,Out>(&self, handle:T) ->Out
    where T: FnOnce(&mut $obj) -> Out
    {
       unsafe{
           let target = self.raw_ptr_mut();
           return handle(&mut *target)
       };
    }
    pub async fn async_ref<T,Out>(&self,handle:T)->Out
    where T: FnOnce(&mut $obj) -> Out
    {
        let mut target = self.lock().await;
        handle(std::ops::DerefMut::deref_mut(&mut target))
    }
    pub async fn async_ref_handle<T,F,Out>(&self,handle:T)->Out
    where T:FnOnce(&mut $obj) -> F,
        F:std::future::Future<Output=Out>
    {
        let mut target = self.lock().await;
        handle(std::ops::DerefMut::deref_mut(&mut target)).await
    }
}
    };
}

#[cfg(test)]
mod test{
    use crate::PFArc;
    use crate::sync::async_mutex::AsyncMutex;
    use crate::sync::WaitGroup;

    struct Target{
        name:String,
        age:i32,
    }
    share!(Target);

    #[tokio::test(flavor ="multi_thread", worker_threads = 2)]
    async fn test_global(){
        let tg = Target{name:"teshin".into(),age:0};
        let tg = AsyncMutex::new(tg).arc();
        let t1 = tg.clone();

        let use_time = std::time::Instant::now();
        let wg = WaitGroup::default();
        wg.defer(move ||async move{
            for _ in 0..1000 {
                let _:() = t1.lock_ref_mut(|x|{
                    x.age += 1;
                });
            }
        });
        let t2 = tg.clone();
        wg.defer(move ||async move{
            for _ in 0..1000 {
                let _:() = t2.lock_ref_mut(|x|{
                    x.age += 1;
                });
            }
        });
        let t3 = tg.clone();
        wg.defer(move ||async move{
            for _ in 0..1000 {
                let _:() = t3.async_ref(|x|{
                    x.age += 1;
                }).await;
            }
        });
        let t4 = tg.clone();
        wg.defer(move ||async move{
            for _ in 0..1000 {
                let _:i32 = t4.async_ref_handle(|x|{
                    x.age += 1;
                    let a =x.age;
                    async move {
                        return a;
                    }
                }).await;
            }
        });
        wg.wait().await;
        println!("age result = {}",tg.unsafe_mut_ptr(|x|x.age));
        println!("use time = {}ms",use_time.elapsed().as_millis())
    }
}