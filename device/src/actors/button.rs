use crate::kernel::{
    actor::{Actor, Address},
    util::ImmediateFuture,
};
use core::future::Future;
use core::pin::Pin;
use embassy::traits::gpio::WaitForAnyEdge;
use embedded_hal::digital::v2::InputPin;

pub trait FromButtonEvent<M> {
    fn from(event: ButtonEvent) -> Option<M>
    where
        Self: Sized;
}

pub enum ButtonEvent {
    Pressed,
    Released,
}

pub struct Button<
    'a,
    P: WaitForAnyEdge + InputPin + 'a,
    A: Actor + FromButtonEvent<A::Message<'a>> + 'static,
> {
    pin: P,
    handler: Option<Address<'a, A>>,
}

impl<'a, P: WaitForAnyEdge + InputPin + 'a, A: Actor + FromButtonEvent<A::Message<'a>> + 'a>
    Button<'a, P, A>
{
    pub fn new(pin: P) -> Self {
        Self { pin, handler: None }
    }
}

impl<'a, P: WaitForAnyEdge + InputPin + 'a, A: Actor + FromButtonEvent<A::Message<'a>> + 'a> Unpin
    for Button<'a, P, A>
{
}

impl<'a, P: WaitForAnyEdge + InputPin + 'a, A: Actor + FromButtonEvent<A::Message<'a>> + 'a> Actor
    for Button<'a, P, A>
{
    type Configuration = Address<'a, A>;
    #[rustfmt::skip]
    type OnStartFuture<'m> where 'a: 'm = impl Future<Output = ()> + 'm;
    #[rustfmt::skip]
    type OnMessageFuture<'m> where 'a: 'm = ImmediateFuture;

    fn on_mount(&mut self, _: Address<'static, Self>, config: Self::Configuration) {
        self.handler.replace(config);
    }

    fn on_start(mut self: Pin<&mut Self>) -> Self::OnStartFuture<'_> {
        async move {
            loop {
                trace!("Button wait for edge");
                self.pin.wait_for_any_edge().await;
                let event = if self.pin.is_high().ok().unwrap() {
                    trace!("Button released");
                    ButtonEvent::Released
                } else {
                    trace!("Button pressed");
                    ButtonEvent::Pressed
                };

                if let Some(handler) = self.handler {
                    let mut message = A::from(event);
                    if let Some(m) = message.take() {
                        let _ = handler.notify(m);
                    }
                }
            }
        }
    }

    fn on_message<'m>(self: Pin<&'m mut Self>, _: Self::Message<'m>) -> Self::OnMessageFuture<'m> {
        ImmediateFuture::new()
    }
}
