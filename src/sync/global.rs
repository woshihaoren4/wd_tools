#[derive(Default)]
struct TestStruct {
    age: i32,
}

#[macro_export]
macro_rules! share {
    ($obj:tt,$gf:tt) => {
        impl $obj {
            pub fn lock_ref_mut<T, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> Out,
            {
                let this = $gf();
                let mut binding = this.synchronize();
                let target = std::ops::DerefMut::deref_mut(&mut binding);
                handle(target)
            }
            pub fn unsafe_mut_ptr<T, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> Out,
            {
                let this = $gf();
                unsafe {
                    let target = this.raw_ptr_mut();
                    return handle(&mut *target);
                };
            }
            pub async fn async_ref<T, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> Out,
            {
                let this = $gf();
                let mut target = this.lock().await;
                handle(std::ops::DerefMut::deref_mut(&mut target))
            }
            pub async fn async_ref_handle<T, F, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> F,
                F: std::future::Future<Output = Out>,
            {
                let this = $gf();
                let mut target = this.lock().await;
                handle(std::ops::DerefMut::deref_mut(&mut target)).await
            }
        }
    };
}

#[cfg(test)]
mod test {
    use crate::sync::async_mutex::AsyncMutex;
    use crate::sync::global::TestStruct;
    use crate::sync::WaitGroup;

    static mut __TEST_STRUCT: Option<AsyncMutex<TestStruct>> = None;
    static mut __TEST_ONCE: std::sync::Once = std::sync::Once::new();

    fn get_test_struct() -> &'static AsyncMutex<TestStruct> {
        unsafe {
            __TEST_ONCE.call_once(|| {
                __TEST_STRUCT = Some(AsyncMutex::new(std::default::Default::default()))
            });
            match __TEST_STRUCT {
                Some(ref s) => s,
                None => {
                    panic!("__TEST_STRUCT init failed")
                }
            }
        }
    }

    share!(TestStruct, get_test_struct);

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_global() {
        let use_time = std::time::Instant::now();
        let wg = WaitGroup::default();
        wg.defer(move || async move {
            for _ in 0..1000 {
                let _: () = TestStruct::lock_ref_mut(|x| {
                    x.age += 1;
                });
            }
        });
        wg.defer(move || async move {
            for _ in 0..1000 {
                let _: () = TestStruct::lock_ref_mut(|x| {
                    x.age += 1;
                });
            }
        });
        wg.defer(move || async move {
            for _ in 0..1000 {
                let _: () = TestStruct::async_ref(|x| {
                    x.age += 1;
                })
                .await;
            }
        });
        wg.defer(move || async move {
            for _ in 0..1000 {
                let _: i32 = TestStruct::async_ref_handle(|x| {
                    x.age += 1;
                    let a = x.age;
                    async move {
                        return a;
                    }
                })
                .await;
            }
        });
        wg.wait().await;
        println!("age result = {}", TestStruct::unsafe_mut_ptr(|x| x.age));
        println!("use time = {}ms", use_time.elapsed().as_millis())
    }
}
