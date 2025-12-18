use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Acl<T> {
    inner: Arc<ArcSwap<T>>,
}

impl<T> Acl<T> {
    pub fn new(val: T) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(val)),
        }
    }

    pub fn share(&self) -> Arc<T> {
        self.inner.load_full()
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(Arc<T>) -> T,
    {
        let old = self.share();
        let new = f(old);
        self.inner.store(Arc::new(new))
    }

    pub fn set(&self, val: T) {
        self.inner.store(Arc::new(val));
    }
}

#[cfg(test)]
mod test {
    use crate::sync::Acl;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_acl_update() {
        let acl = Acl::new(1usize);
        acl.update(|o| &*o + 1);
        acl.update(|o| &*o + 1);
        acl.update(|o| &*o + 1);
        acl.update(|o| &*o + 1);
        let arc = acl.share();
        assert_eq!(*arc, 5);
        println!("success");
    }

    //cargo test --color=always -p wd_tools --lib sync::copy_lock::test::test_acl --no-fail-fast -- --exact  unstable-options --show-output --nocapture
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_acl() {
        let index = Arc::new(AtomicUsize::new(1));
        let wcp = Acl::new(1usize);
        for _ in 0..40 {
            let acl = wcp.clone();
            tokio::spawn(async move {
                loop {
                    let a = acl.share();
                    if *a > 400000 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                println!("---> read over");
            });
        }
        for _ in 0..4 {
            let acl = wcp.clone();
            let nb = index.clone();
            tokio::spawn(async move {
                for _ in 0..100000 {
                    let _i = nb.fetch_add(1, Ordering::SeqCst);
                    acl.update(|o| &*o + 1)
                }
                println!("---> write over");
            });
        }

        println!("success");
        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("over");
    }
}
