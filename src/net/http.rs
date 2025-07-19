#![allow(dead_code)]

use crate::Ctx;
use reqwest::header::HeaderMap;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

pub use reqwest::*;

#[async_trait::async_trait]
pub trait ResponseHook: Send + Sync {
    async fn hook(&self, ctx: Ctx, resp: Response) -> anyhow::Result<Box<dyn Any>>;
}

struct ResponseHookImpl<T, F> {
    inner: T,
    b: PhantomData<F>,
}

#[async_trait::async_trait]
impl<T, F> ResponseHook for ResponseHookImpl<T, F>
where
    T: Fn(Ctx, Response) -> F + Send + Sync,
    F: Future<Output = anyhow::Result<Box<dyn Any>>> + Send + Sync,
{
    async fn hook(&self, ctx: Ctx, resp: Response) -> anyhow::Result<Box<dyn Any>> {
        (self.inner)(ctx, resp).await
    }
}

pub struct Http {
    pub method: Method,
    pub url: Url,
    pub header: Option<HashMap<String, String>>,
    pub body: Option<Body>,
    pub hook_ctx: Ctx,
    client_build_hook:
        Option<Arc<dyn Fn(Ctx, ClientBuilder) -> anyhow::Result<Client> + Send + Sync>>,
    request_build_hook:
        Option<Arc<dyn Fn(Ctx, RequestBuilder) -> anyhow::Result<RequestBuilder> + Send + Sync>>,
    response_hook: Option<Arc<dyn ResponseHook>>,
}

impl Clone for Http {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            header: self.header.clone(),
            body: None,
            hook_ctx: self.hook_ctx.clone(),
            client_build_hook: self.client_build_hook.clone(),
            request_build_hook: self.request_build_hook.clone(),
            response_hook: self.response_hook.clone(),
        }
    }
}

impl Http {
    async fn default_response_hook(_: Ctx, resp: Response) -> anyhow::Result<Box<dyn Any>> {
        Ok(Box::new(resp))
    }
    pub fn new<M: Into<Method>, U: IntoUrl>(method: M, url: U) -> anyhow::Result<Self> {
        let method = method.into();
        let url = url.into_url().unwrap();
        let header = None;
        let body = None;
        let hook_ctx = Ctx::default();
        let client_build_hook = None;
        let request_build_hook = None;
        let response_hook: Option<Arc<dyn ResponseHook>> = Some(Arc::new(ResponseHookImpl {
            inner: Http::default_response_hook,
            b: PhantomData::default(),
        }));
        Ok(Http {
            method,
            url,
            header,
            body,
            hook_ctx,
            client_build_hook,
            request_build_hook,
            response_hook,
        })
    }
    pub fn header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        if self.header.is_none() {
            self.header = Some(HashMap::new());
        }
        if let Some(ref mut header) = self.header {
            header.insert(key.into(), value.into());
        }
        self
    }
    pub fn body<T: Into<Body>>(mut self, body: T) -> Self {
        self.body = Some(body.into());
        self
    }
    pub fn hook_client_build(
        mut self,
        func: impl Fn(Ctx, ClientBuilder) -> anyhow::Result<Client> + Send + Sync + 'static,
    ) -> Self {
        self.client_build_hook = Some(Arc::new(func));
        self
    }
    pub fn hook_request_build(
        mut self,
        func: impl Fn(Ctx, RequestBuilder) -> anyhow::Result<RequestBuilder> + Send + Sync + 'static,
    ) -> Self {
        self.request_build_hook = Some(Arc::new(func));
        self
    }
    pub fn hook_response<
        F: Future<Output = anyhow::Result<Box<dyn Any>>> + Sync + Send + 'static,
    >(
        mut self,
        func: impl Fn(Ctx, Response) -> F + Send + Sync + 'static,
    ) -> Self {
        self.response_hook = Some(Arc::new(ResponseHookImpl {
            inner: func,
            b: PhantomData::default(),
        }));
        self
    }
    pub async fn send<B: Into<Body>, T: Any>(&self, body: B) -> anyhow::Result<T> {
        self.clone().body(body).into_send().await
    }
    pub async fn send_no_body<T: Any>(&self) -> anyhow::Result<T> {
        self.clone().into_send().await
    }
    pub async fn into_send<T: Any>(self) -> anyhow::Result<T> {
        let builder = Client::builder();
        let client = match self.client_build_hook {
            None => builder.build()?,
            Some(hook) => hook(self.hook_ctx.clone(), builder)?,
        };

        let mut builder = client.request(self.method, self.url);
        if let Some(headers) = self.header {
            builder = builder.headers(HeaderMap::try_from(&headers).unwrap());
        }
        if let Some(body) = self.body {
            builder = builder.body(body);
        }
        let builder = match self.request_build_hook {
            None => builder,
            Some(hook) => hook(self.hook_ctx.clone(), builder)?,
        };

        let resp = builder.send().await?;
        if let Some(hook) = self.response_hook {
            let x = hook.hook(self.hook_ctx.clone(), resp).await?;
            if (*x).type_id() != TypeId::of::<T>() {
                return Err(anyhow::anyhow!(
                    "decoding error: expect type[{:?}], but find type[{:?}]",
                    TypeId::of::<T>(),
                    x.type_id()
                ));
            }
            return Ok(*crate::ptr::unsafe_downcast::<_, T>(x));
        }
        Err(anyhow::anyhow!("decoding error,unknown expect type"))
    }
}

#[cfg(test)]
mod test {
    use super::{Method, Response};
    use crate::http::{Http, StatusCode};
    use std::any::Any;
    use std::time::Duration;

    #[tokio::test]
    async fn test_http_into_send() {
        let http: Response = Http::new(Method::GET, "https://www.baidu.com")
            .unwrap()
            .header("hello", "world")
            .into_send()
            .await
            .unwrap();
        println!("--->\n {:?}", http.text().await.unwrap());
    }

    #[tokio::test]
    async fn test_hook() {
        let (code,body):(StatusCode,String) = Http::new(Method::GET, "https://crates.io/api/v1/crates/wd_tools").unwrap()
            .hook_client_build(|_, builder| {
                let client = builder.timeout(Duration::from_secs(10)).build()?;
                Ok(client)
            })
            .hook_request_build(|_, builder| {
                Ok(builder.header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"))
            })
            .hook_response(|_, resp| async move {
                let status = resp.status();
                let body = resp.text().await?;
                let result:anyhow::Result<Box<dyn Any>>= Ok(Box::new((status, body)));
                result
            })
            .send_no_body().await.unwrap();
        assert_eq!(code.as_u16(), 200u16);
        println!("{}-->{}", code, body)
    }
}
