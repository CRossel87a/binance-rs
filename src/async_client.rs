use hex::encode as hex_encode;
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT, CONTENT_TYPE};
use sha2::Sha256;
use serde::de::DeserializeOwned;
use crate::api::API;

#[derive(Clone)]
pub struct AsyncClient {
    api_key: String,
    secret_key: String,
    host: String,
    inner_client: reqwest::Client,
}

impl AsyncClient {
    pub fn new(api_key: Option<String>, secret_key: Option<String>, host: String, proxy_op: Option<String>) -> anyhow::Result<Self> {

        let inner_client = match proxy_op {
            Some(url) => {
                let proxy = reqwest::Proxy::all(url)?;
                reqwest::Client::builder().proxy(proxy).build()?
            },
            None => reqwest::Client::new()
        };
        
        Ok(AsyncClient {
            api_key: api_key.unwrap_or_default(),
            secret_key: secret_key.unwrap_or_default(),
            host,
            inner_client,
        })
    }

    pub async fn get_signed<T: DeserializeOwned>(
        &self, endpoint: API, request: Option<String>,
    ) -> anyhow::Result<T> {
        let url = self.sign_request(endpoint, request);
        let client = &self.inner_client;
        let response = client
            .get(url.as_str())
            .headers(self.build_headers(true)?)
            .send().await?;

        self.handler(response).await
    }

    pub async fn post_signed<T: DeserializeOwned>(&self, endpoint: API, request: String) -> anyhow::Result<T> {
        let url = self.sign_request(endpoint, Some(request));
        let client = &self.inner_client;
        let response = client
            .post(url.as_str())
            .headers(self.build_headers(true)?)
            .send().await?;

        self.handler(response).await
    }

    pub async fn delete_signed<T: DeserializeOwned>(
        &self, endpoint: API, request: Option<String>,
    ) -> anyhow::Result<T> {
        let url = self.sign_request(endpoint, request);
        let client = &self.inner_client;
        let response = client
            .delete(url.as_str())
            .headers(self.build_headers(true)?)
            .send().await?;

        self.handler(response).await
    }

    pub async fn get<T: DeserializeOwned>(&self, endpoint: API, request: Option<String>) -> anyhow::Result<T> {
        let mut url: String = format!("{}{}", self.host, String::from(endpoint));
        if let Some(request) = request {
            if !request.is_empty() {
                url.push_str(format!("?{}", request).as_str());
            }
        }

        let client = &self.inner_client;
        let response = client.get(url.as_str()).send().await?;

        self.handler(response).await
    }

    pub async fn post<T: DeserializeOwned>(&self, endpoint: API) -> anyhow::Result<T> {
        let url: String = format!("{}{}", self.host, String::from(endpoint));

        let client = &self.inner_client;
        let response = client
            .post(url.as_str())
            .headers(self.build_headers(false)?)
            .send().await?;

        self.handler(response).await
    }

    pub async fn put<T: DeserializeOwned>(&self, endpoint: API, listen_key: &str) -> anyhow::Result<T> {
        let url: String = format!("{}{}", self.host, String::from(endpoint));
        let data: String = format!("listenKey={}", listen_key);

        let client = &self.inner_client;
        let response = client
            .put(url.as_str())
            .headers(self.build_headers(false)?)
            .body(data)
            .send().await?;

        self.handler(response).await
    }

    pub async fn delete<T: DeserializeOwned>(&self, endpoint: API, listen_key: &str) -> anyhow::Result<T> {
        let url: String = format!("{}{}", self.host, String::from(endpoint));
        let data: String = format!("listenKey={}", listen_key);

        let client = &self.inner_client;
        let response = client
            .delete(url.as_str())
            .headers(self.build_headers(false)?)
            .body(data)
            .send().await?;

        self.handler(response).await
    }

    // Request must be signed
    fn sign_request(&self, endpoint: API, request: Option<String>) -> String {
        if let Some(request) = request {
            let mut signed_key =
                Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
            signed_key.update(request.as_bytes());
            let signature = hex_encode(signed_key.finalize().into_bytes());
            let request_body: String = format!("{}&signature={}", request, signature);
            format!("{}{}?{}", self.host, String::from(endpoint), request_body)
        } else {
            let signed_key = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
            let signature = hex_encode(signed_key.finalize().into_bytes());
            let request_body: String = format!("&signature={}", signature);
            format!("{}{}?{}", self.host, String::from(endpoint), request_body)
        }
    }

    fn build_headers(&self, content_type: bool) -> anyhow::Result<HeaderMap> {
        let mut custom_headers = HeaderMap::new();

        custom_headers.insert(USER_AGENT, HeaderValue::from_static("binance-rs"));
        if content_type {
            custom_headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }
        custom_headers.insert(
            HeaderName::from_static("x-mbx-apikey"),
            HeaderValue::from_str(self.api_key.as_str())?,
        );

        Ok(custom_headers)
    }

    async fn handler<T: DeserializeOwned>(&self, response: reqwest::Response) -> anyhow::Result<T> {
        match response.status() {
            StatusCode::OK => Ok(response.json::<T>().await?),
            StatusCode::INTERNAL_SERVER_ERROR => {
                anyhow::bail!("Internal Server Error");
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                anyhow::bail!("Service Unavailable");
            }
            StatusCode::UNAUTHORIZED => {
                anyhow::bail!("Unauthorized");
            }
            StatusCode::BAD_REQUEST => {
                //let error: BinanceContentError = response.json().await?;

                let err = response.json().await?;

                anyhow::bail!("binance error: {:?}",err);
                //Err(anyhow::anyhow!(error))

                //Err(ErrorKind::BinanceError(error).into())
            }
            s => {
                anyhow::bail!(format!("Received response: {:?}", s));
            }
        }
    }
}
