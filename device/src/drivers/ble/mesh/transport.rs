use core::future::Future;
use embassy::executor::Spawner;

pub trait Handler: Sized {
    fn handle(&self, message: &[u8]);
}

pub trait Transport {
    fn new() -> Self;

    type StartFuture<'m>: Future<Output = ()>
    where
        Self: 'm;

    fn start<'m>(&'m self) -> Self::StartFuture<'m>;

    type ReceiveFuture<'m, H>: Future<Output = ()>
    where
        Self: 'm,
        H: 'm;

    fn start_receive<'m, H: Handler + 'm>(&'m self, handler: &'m H) -> Self::ReceiveFuture<'m, H>;

    type TransmitFuture<'m>: Future<Output = ()>
    where
        Self: 'm;

    fn transmit<'m>(&'m self, message: &'m [u8]) -> Self::TransmitFuture<'m>;
}
