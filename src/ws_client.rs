
use crate::error::{ Error, Result };
use futures::prelude::*;
use futures::channel::mpsc::{ UnboundedSender, UnboundedReceiver, unbounded };
use std::time::Duration;
use std::pin::Pin;
use std::thread;

use jsonrpsee::{
  transport::{
    TransportClient,
  },
  common::{
    Request,
    Response,
  },
  raw::{
    RawClient
  },
  Client,
};

use tungstenite::{
  connect, 
  Message,
  error::Error as WsError,
};
use url::Url;
use std::sync::Arc;

pub struct WsTransportClient {
  req_tx: UnboundedSender<Request>,
  res_rx: UnboundedReceiver<Message>,
}

impl WsTransportClient {

  pub fn new(url: &str) -> Result<Self> {
    let url = Url::parse(&url).map_err(|err| format!("{:?}", err) )?;
    let (socket, _respose) = connect(url.clone())?;
    let (req_tx, mut req_rx) = unbounded::<Request>();
    let (res_tx, res_rx) = unbounded::<Message>();

    let socket = Arc::new(socket);
    let mut writer = socket.clone();
    let mut reader = socket.clone();

    std::thread::spawn(move || {
      let reader = unsafe { Arc::get_mut_unchecked(&mut reader) };
      loop {
        match reader.read_message() {
          Ok(response) => {
            let _ = res_tx.unbounded_send(response);
          },
          Err(err) => match err { 
            WsError::ConnectionClosed | WsError::AlreadyClosed => break,
            _ => continue,
          },
        }
      }
    });

    async_std::task::spawn(async move {
      let writer = unsafe { Arc::get_mut_unchecked(&mut writer) };
      loop {
        if let Ok(Some(req)) = req_rx.try_next() {
          let body = serde_json::to_string(&req).unwrap();
          let _ = writer.write_message(Message::Text(body.clone()));
        } else {
          thread::sleep(Duration::from_millis(5));
        }
      }
    });

    let client = Self {
      req_tx,
      res_rx,
    };
    Ok(client)
  }
}

impl TransportClient for WsTransportClient {
  type Error = Error;

  fn send_request<'a>(
    &'a mut self,
    request: Request,
  ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
  {
    Box::pin(async move {
      let _ = self.req_tx.unbounded_send(request);
      Ok(())
    })
  }

  fn next_response<'a>(
    &'a mut self,
  ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>
  {
    
    Box::pin(async move {
      match self.res_rx.try_next() {
        Ok(Some(v)) => {
          let msg = v.into_text()?;
          Response::from_json(&msg).map_err(Into::into)
        },
        _ => {
          thread::sleep(Duration::from_millis(5));
          Err("retry".into())
        },
      }
    })
  }
}


pub fn create(url: &str) -> Client {
  let transport = WsTransportClient::new(url).unwrap();
  let client = Client::from(RawClient::new(transport));
  client
}


#[cfg(test)]
mod tests {
  use super::*;
  use runtime::{ SignedBlock };
  use crate::primitives::{ Hash};
  use jsonrpsee::common::{
    to_value as to_json_value,
    Params,
  };

  #[tokio::test]
  async fn test_transport() {
    let client = create("wss://rpc.polkadot.io");

    let params = Params::Array(vec![to_json_value(1).unwrap()]);
    let hash: Result<Option<Hash>> = client.request("chain_getBlockHash", params.clone()).await.map_err(Into::into);
    assert!(hash.is_ok());

    let params = Params::Array(vec![to_json_value(hash.unwrap()).unwrap()]);
    let block: Result<Option<SignedBlock>> = client.request("chain_getBlock", params).await.map_err(Into::into);
    assert!(block.is_ok());
  }
}