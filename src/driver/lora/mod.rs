use crate::actor::Actor;
use crate::address::Address;
use crate::handler::RequestHandler;
pub use drogue_lora::*;

#[derive(Debug)]
pub struct Initialize;
#[derive(Debug)]
pub struct Configure<'a>(&'a LoraConfig);
#[derive(Debug)]
pub struct Join(ConnectMode);
#[derive(Debug)]
pub struct Reset(ResetMode);
#[derive(Debug)]
pub struct Send<'a>(QoS, Port, &'a [u8]);
#[derive(Debug)]
pub struct Recv<'a>(&'a mut [u8]);

#[derive(Debug)]
pub enum LoraError {
    SendError,
    RecvError,
    RecvTimeout,
    NotInitialized,
    OtherError,
}

impl<A> Address<A>
where
    A: Actor,
{
    pub async fn initialize(&self) -> Result<(), LoraError>
    where
        A: RequestHandler<Initialize, Response = Result<(), LoraError>>,
    {
        self.request(Initialize).await
    }

    pub async fn configure<'a>(&self, config: &'a LoraConfig) -> Result<(), LoraError>
    where
        A: RequestHandler<Configure<'a>, Response = Result<(), LoraError>>,
    {
        self.request_panicking(Configure(config)).await
    }

    pub async fn reset(&self, mode: ResetMode) -> Result<(), LoraError>
    where
        A: RequestHandler<Reset, Response = Result<(), LoraError>>,
    {
        self.request(Reset(mode)).await
    }

    pub async fn join(&self, mode: ConnectMode) -> Result<(), LoraError>
    where
        A: RequestHandler<Join, Response = Result<(), LoraError>>,
    {
        self.request(Join(mode)).await
    }

    pub async fn send<'a>(&self, qos: QoS, port: Port, data: &'a [u8]) -> Result<(), LoraError>
    where
        A: RequestHandler<Send<'a>, Response = Result<(), LoraError>>,
    {
        self.request_panicking(Send(qos, port, data)).await
    }

    pub async fn recv<'a>(&self, port: Port, rx_buf: &mut [u8]) -> Result<(), LoraError>
    where
        A: RequestHandler<Recv<'a>>,
    {
        Ok(())
    }
}

#[cfg(feature = "driver-rak811")]
pub mod rak811;
