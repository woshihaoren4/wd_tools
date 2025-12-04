
#[macro_export]
macro_rules! share {
    ($obj:tt,$gf:tt) => {
        impl $obj {
            #[allow(dead_code)]
            pub fn lock_ref_mut<T, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> Out,
            {
                let this = $gf();
                let mut binding = this.synchronize();
                let target = std::ops::DerefMut::deref_mut(&mut binding);
                handle(target)
            }
            #[allow(dead_code)]
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
            #[allow(dead_code)]
            pub async fn async_ref<T, Out>(handle: T) -> Out
            where
                T: FnOnce(&mut $obj) -> Out,
            {
                let this = $gf();
                let mut target = this.lock().await;
                handle(std::ops::DerefMut::deref_mut(&mut target))
            }
            #[allow(dead_code)]
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

#[macro_export]
macro_rules! global {
    ($type_name:ident,$init_func:block) => {
         paste::paste! {
             #[allow(non_snake_case,non_upper_case_globals)]
             static mut [<__ $type_name _STRUCT>]: Option<AsyncMutex<$type_name>> = None;
             #[allow(non_snake_case,non_upper_case_globals)]
             static mut [<__ $type_name _ONCE>]: std::sync::Once = std::sync::Once::new();

             #[allow(non_snake_case)]
             fn [<_get_ $type_name>]() -> &'static AsyncMutex<$type_name> {
                unsafe {
                #[allow(static_mut_refs)]
                [<__ $type_name _ONCE>].call_once(|| {
                    let t = $init_func;
                    [<__ $type_name _STRUCT>] = Some(AsyncMutex::new(t))
                });
                match [<__ $type_name _STRUCT>] {
                    Some(ref s) => s,
                    None => {
                    panic!("{} init failed", stringify!($type_name))
                        }
                    }
                }
            }
             share!($type_name, [<_get_ $type_name>]);
         }
    };
}

#[cfg(test)]
mod test {
    use crate::sync::WaitGroup;
    use super::super::AsyncMutex;

    #[derive(Default)]
    struct TestStruct {
        age: i32,
    }

    global!(TestStruct,{
        TestStruct::default()
    });

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
