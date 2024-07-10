use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::async_client::AsyncClient;
use crate::model::Empty;
use crate::futures::model::{ExchangeInformation, ServerTime, Symbol};
use crate::api::API;
use crate::api::Futures;

#[derive(Clone)]
pub struct FuturesGeneral {
    pub client: AsyncClient,
}

impl FuturesGeneral {
    // Test connectivity
    pub async fn ping(&self) -> Result<Duration> {
        let t0 = Instant::now();
        self.client.get::<Empty>(API::Futures(Futures::Ping), None).await?;
        Ok(t0.elapsed())
    }

    // Check server time
    pub async fn get_server_time(&self) -> Result<ServerTime> {
        self.client.get(API::Futures(Futures::Time), None).await
    }

    // Obtain exchange information
    // - Current exchange trading rules and symbol information
    pub async fn exchange_info(&self) -> Result<ExchangeInformation> {
        self.client.get(API::Futures(Futures::ExchangeInfo), None).await
    }

    // Get Symbol information
    pub async fn get_symbol_info<S>(&self, symbol: S) -> Result<Symbol>
    where
        S: Into<String>,
    {
        let upper_symbol = symbol.into().to_uppercase();
        match self.exchange_info().await {
            Ok(info) => {
                for item in info.symbols {
                    if item.symbol == upper_symbol {
                        return Ok(item);
                    }
                }
                bail!("Symbol not found")
            }
            Err(e) => Err(e),
        }
    }
}
