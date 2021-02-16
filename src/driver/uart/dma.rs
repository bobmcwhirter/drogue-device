use crate::prelude::*;

pub use crate::api::uart::Error;
use crate::api::{
    scheduler::*,
    uart::{UartRead, UartReadWithTimeout, UartReader, UartWrite, UartWriter},
};
use crate::domain::time::duration::{Duration, Milliseconds};
use crate::hal::uart::dma::DmaUartHal;
use crate::interrupt::{Interrupt, InterruptContext};
use crate::package::Package;
use crate::synchronization::Signal;

use core::cell::{Cell, RefCell, UnsafeCell};
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use core::task::{Context, Poll};
use cortex_m::interrupt::Nr;

use crate::util::dma::async_bbqueue::*;

pub struct UartActor<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    me: Option<Address<Self>>,
    shared: Option<&'static Shared<U, T>>,
    rx_consumer: Option<AsyncBBConsumer<RXN>>,
    controller: Option<Address<UartController<U, T>>>,
}

pub struct UartController<U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    shared: Option<&'static Shared<U, T>>,
}

pub struct UartInterrupt<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    shared: Option<&'static Shared<U, T>>,
    me: Option<Address<Self>>,
    controller: Option<Address<UartController<U, T>>>,
    rx_producer: Option<AsyncBBProducer<RXN>>,
    rx_producer_grant: Option<RefCell<AsyncBBProducerGrant<'static, RXN>>>,
}

const READY_STATE: bool = false;
const BUSY_STATE: bool = true;

pub struct Shared<U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    uart: U,
    timer: RefCell<Option<Address<T>>>,
    tx_state: Cell<State>,
    tx_done: Signal<Result<(), Error>>,

    rx_state: AtomicBool,
    rx_timeout: Signal<()>,
}

pub struct DmaUart<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    actor: ActorContext<UartActor<U, T, RXN>>,
    controller: ActorContext<UartController<U, T>>,
    interrupt: InterruptContext<UartInterrupt<U, T, RXN>>,
    shared: Shared<U, T>,
    rx_buffer: UnsafeCell<AsyncBBBuffer<'static, RXN>>,
    rx_cons: RefCell<Option<UnsafeCell<AsyncBBConsumer<RXN>>>>,
    rx_prod: RefCell<Option<UnsafeCell<AsyncBBProducer<RXN>>>>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum State {
    Ready,
    InProgress,
    Timeout,
}

impl<U, T> Shared<U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    fn new(uart: U) -> Self {
        Self {
            tx_done: Signal::new(),
            uart,
            timer: RefCell::new(None),
            tx_state: Cell::new(State::Ready),
            rx_timeout: Signal::new(),
            rx_state: AtomicBool::new(READY_STATE),
        }
    }
}

impl<U, T, RXN> DmaUart<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    pub fn new<IRQ>(uart: U, irq: IRQ) -> Self
    where
        IRQ: Nr,
    {
        Self {
            actor: ActorContext::new(UartActor::new()).with_name("uart_actor"),
            controller: ActorContext::new(UartController::new()).with_name("uart_controller"),
            interrupt: InterruptContext::new(UartInterrupt::new(), irq).with_name("uart_interrupt"),
            shared: Shared::new(uart),
            rx_buffer: UnsafeCell::new(AsyncBBBuffer::new()),
            rx_prod: RefCell::new(None),
            rx_cons: RefCell::new(None),
        }
    }
}

impl<U, T, RXN> Package for DmaUart<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    type Primary = UartActor<U, T, RXN>;
    type Configuration = Address<T>;
    fn mount(
        &'static self,
        timer: Self::Configuration,
        supervisor: &mut Supervisor,
    ) -> Address<UartActor<U, T, RXN>> {
        let (rx_prod, rx_cons) = unsafe { (&mut *self.rx_buffer.get()).split() };

        self.shared.timer.borrow_mut().replace(timer);
        let controller = self.controller.mount(&self.shared, supervisor);
        let addr = self
            .actor
            .mount((&self.shared, controller, rx_cons), supervisor);
        self.interrupt
            .mount((&self.shared, controller, rx_prod), supervisor);

        addr
    }

    fn primary(&'static self) -> Address<Self::Primary> {
        self.actor.address()
    }
}

impl<U, T, RXN> UartActor<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    pub fn new() -> Self {
        Self {
            shared: None,
            me: None,
            rx_consumer: None,
            controller: None,
        }
    }
}

impl<U, T> Actor for UartController<U, T>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
{
    type Configuration = &'static Shared<U, T>;

    fn on_mount(&mut self, me: Address<Self>, config: Self::Configuration) {
        self.shared.replace(config);
    }
}

impl<U, T> UartController<U, T>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
{
    pub fn new() -> Self {
        Self { shared: None }
    }
}

// DMA implementation of the trait
impl<U, T, RXN> UartReader for UartActor<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    /// Read bytes into the provided rx_buffer. The memory pointed to by the buffer must be available until the return future is await'ed
    fn read<'a>(self, message: UartRead<'a>) -> Response<Self, Result<usize, Error>> {
        let shared = self.shared.as_ref().unwrap();
        if READY_STATE == shared.rx_state.swap(BUSY_STATE, Ordering::SeqCst) {
            let rx_consumer = self.rx_consumer.as_ref().unwrap();
            let future = unsafe { rx_consumer.read(message.0) };
            let future = RxFuture::new(future, shared);
            Response::immediate_future(self, future)
        } else {
            Response::immediate(self, Err(Error::RxInProgress))
        }
    }

    /// Receive bytes into the provided rx_buffer. The memory pointed to by the buffer must be available until the return future is await'ed
    fn read_with_timeout<'a, DUR>(
        self,
        message: UartReadWithTimeout<'a, DUR>,
    ) -> Response<Self, Result<usize, Error>>
    where
        DUR: Duration + Into<Milliseconds> + 'static,
    {
        let shared = self.shared.as_ref().unwrap();
        if READY_STATE == shared.rx_state.swap(BUSY_STATE, Ordering::SeqCst) {
            let rx_consumer = self.rx_consumer.as_ref().unwrap();
            let future = unsafe { rx_consumer.read(message.0) };
            let future = RxFuture::new(future, shared);
            shared.rx_timeout.reset();
            shared.timer.borrow().as_ref().unwrap().schedule(
                message.1,
                ReadTimeout,
                *self.controller.as_ref().unwrap(),
            );
            Response::immediate_future(self, future)
        } else {
            Response::immediate(self, Err(Error::RxInProgress))
        }
    }
}

impl<U, T> NotifyHandler<ReadTimeout> for UartController<U, T>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
{
    fn on_notify(self, message: ReadTimeout) -> Completion<Self> {
        let shared = self.shared.as_ref().unwrap();
        shared.rx_timeout.signal(());
        Completion::immediate(self)
    }
}

impl<U, T, RXN> UartWriter for UartActor<U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    /// Transmit bytes from provided tx_buffer over UART. The memory pointed to by the buffer must be available until the return future is await'ed
    fn write<'a>(self, message: UartWrite<'a>) -> Response<Self, Result<(), Error>> {
        let shared = self.shared.as_ref().unwrap();
        match shared.tx_state.get() {
            State::Ready => {
                shared.tx_done.reset();
                shared.tx_state.set(State::InProgress);
                match shared.uart.start_write(message.0) {
                    Ok(_) => {
                        let future = TxFuture::new(shared);
                        Response::immediate_future(self, future)
                    }
                    Err(e) => Response::immediate(self, Err(e)),
                }
            }
            _ => Response::immediate(self, Err(Error::TxInProgress)),
        }
    }
}

impl<U, T> NotifyHandler<RxTimeout> for UartController<U, T>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
{
    fn on_notify(self, message: RxTimeout) -> Completion<Self> {
        let shared = self.shared.as_ref().unwrap();
        shared.uart.cancel_read();
        Completion::immediate(self)
    }
}

impl<U, T, RXN> Actor for UartActor<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    type Configuration = (
        &'static Shared<U, T>,
        Address<UartController<U, T>>,
        AsyncBBConsumer<RXN>,
    );

    fn on_mount(&mut self, me: Address<Self>, config: Self::Configuration) {
        self.me.replace(me);
        self.shared.replace(config.0);
        self.controller.replace(config.1);
        self.rx_consumer.replace(config.2);
    }
}

impl<U, T, RXN> UartInterrupt<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    pub fn new() -> Self {
        Self {
            shared: None,
            rx_producer: None,
            rx_producer_grant: None,
            me: None,
            controller: None,
        }
    }

    fn start_read(&mut self, read_size: usize, timeout: Milliseconds) {
        let shared = self.shared.as_ref().unwrap();
        let rx_producer = self.rx_producer.as_ref().unwrap();
        // TODO: Handle error?
        match rx_producer.prepare_write(read_size) {
            Ok(mut grant) => match shared.uart.prepare_read(grant.buf()) {
                Ok(_) => {
                    self.rx_producer_grant.replace(RefCell::new(grant));
                    shared.uart.start_read();
                    shared.timer.borrow().as_ref().unwrap().schedule(
                        timeout,
                        RxTimeout,
                        *self.controller.as_ref().unwrap(),
                    );
                }
                Err(e) => {
                    // TODO: Notify self of starting read again?
                    log::error!("Error initiating DMA transfer: {:?}", e);
                    shared.timer.borrow().as_ref().unwrap().schedule(
                        timeout,
                        RxStart,
                        *self.me.as_ref().unwrap(),
                    );
                }
            },
            Err(e) => {
                log::error!("Producer not ready, backing off: {:?}", e);
                shared.timer.borrow().as_ref().unwrap().schedule(
                    Milliseconds(1000),
                    RxStart,
                    *self.me.as_ref().unwrap(),
                );
            }
        }
    }
}

const READ_TIMEOUT: u32 = 100;
const READ_SIZE: usize = 128;

impl<U, T, RXN> Actor for UartInterrupt<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    type Configuration = (
        &'static Shared<U, T>,
        Address<UartController<U, T>>,
        AsyncBBProducer<RXN>,
    );

    fn on_mount(&mut self, me: Address<Self>, config: Self::Configuration) {
        self.shared.replace(config.0);
        self.controller.replace(config.1);
        self.rx_producer.replace(config.2);
        self.me.replace(me);
    }

    fn on_start(mut self) -> Completion<Self> {
        self.start_read(READ_SIZE, Milliseconds(READ_TIMEOUT));
        Completion::immediate(self)
    }
}

impl<U, T, RXN> NotifyHandler<RxStart> for UartInterrupt<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    fn on_notify(mut self, message: RxStart) -> Completion<Self> {
        // log::info!("RX START");
        self.start_read(READ_SIZE, Milliseconds(READ_TIMEOUT));
        Completion::immediate(self)
    }
}

impl<U, T, RXN> Interrupt for UartInterrupt<U, T, RXN>
where
    U: DmaUartHal,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8>,
{
    fn on_interrupt(&mut self) {
        let shared = self.shared.as_ref().unwrap();
        let (tx_done, rx_done) = shared.uart.process_interrupts();
        log::trace!(
            "[UART ISR] TX SIGNALED: {}. TX DONE: {}. RX DONE: {}",
            shared.tx_done.signaled(),
            tx_done,
            rx_done,
        );

        if tx_done {
            shared.tx_done.signal(shared.uart.finish_write());
        }

        if rx_done {
            if let Ok(len) = shared.uart.finish_read() {
                if let Some(grant) = self.rx_producer_grant.take() {
                    if len > 0 {
                        log::trace!("COMMITTING {} bytes", len);
                        grant.into_inner().commit(len);
                    }
                }
            } else {
                log::error!("FINISH READ ERROR");
            }
            self.start_read(READ_SIZE, Milliseconds(READ_TIMEOUT));
        }
    }
}

pub struct TxFuture<'a, U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    shared: &'a Shared<U, T>,
}

impl<'a, U, T> TxFuture<'a, U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    fn new(shared: &'a Shared<U, T>) -> Self {
        Self { shared }
    }
}

impl<'a, U, T> Future for TxFuture<'a, U, T>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
{
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        log::trace!("Polling TX: {:?}", self.shared.tx_state.get());
        if State::InProgress == self.shared.tx_state.get() {
            if let Poll::Ready(result) = self.shared.tx_done.poll_wait(cx) {
                self.shared.tx_state.set(State::Ready);
                log::trace!("Marked TX future complete. Set ready");
                return Poll::Ready(result);
            }
        }
        return Poll::Pending;
    }
}

struct RxFuture<'a, U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    future: AsyncRead<RXN>,
    shared: &'a Shared<U, T>,
}

impl<'a, U, T, RXN> RxFuture<'a, U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    fn new(future: AsyncRead<RXN>, shared: &'a Shared<U, T>) -> Self {
        Self { future, shared }
    }
}

impl<'a, U, T, RXN> Future for RxFuture<'a, U, T, RXN>
where
    U: DmaUartHal + 'static,
    T: Scheduler + 'static,
    RXN: ArrayLength<u8> + 'static,
{
    type Output = Result<usize, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(_) = self.shared.rx_timeout.poll_wait(cx) {
            self.future.cancel();
        }

        match Future::poll(Pin::new(&mut self.future), cx) {
            Poll::Ready(result) => {
                self.shared.rx_state.store(READY_STATE, Ordering::SeqCst);
                return Poll::Ready(result.map_err(|_| Error::Receive));
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone)]
struct ReadTimeout;

#[derive(Clone)]
struct RxTimeout;

#[derive(Clone)]
struct RxStart;

#[cfg(test)]
mod tests {
    /*
    extern crate std;
    use super::*;
    use crate::driver::timer::TimerActor;
    use core::sync::atomic::*;
    use futures::executor::block_on;
    use std::boxed::Box;

    struct TestTimer {}

    impl crate::hal::timer::Timer for TestTimer {
        fn start(&mut self, duration: Milliseconds) {}

        fn clear_update_interrupt_flag(&mut self) {}
    }

    struct TestHal {
        internal_buf: RefCell<[u8; 255]>,
        interrupt: Option<RefCell<UartInterrupt<Self, TimerActor<TestTimer>>>>,
        did_tx: AtomicBool,
        did_rx: AtomicBool,
    }

    impl TestHal {
        fn new() -> Self {
            Self {
                internal_buf: RefCell::new([0; 255]),
                interrupt: None,
                did_tx: AtomicBool::new(false),
                did_rx: AtomicBool::new(false),
            }
        }

        fn fire_interrupt(&self) {
            self.interrupt.as_ref().unwrap().borrow_mut().on_interrupt();
        }

        fn set_interrupt(&mut self, i: UartInterrupt<Self, TimerActor<TestTimer>>) {
            self.interrupt.replace(RefCell::new(i));
        }
    }

    impl DmaUartHal for TestHal {
        fn start_write(&self, tx_buffer: &[u8]) -> Result<(), Error> {
            {
                self.internal_buf.borrow_mut().copy_from_slice(tx_buffer);
                self.did_tx.store(true, Ordering::SeqCst);
            }
            self.fire_interrupt();
            Ok(())
        }

        fn finish_write(&self) -> Result<(), Error> {
            Ok(())
        }

        fn cancel_write(&self) {}

        fn prepare_read(&self, rx_buffer: &mut [u8]) -> Result<(), Error> {
            rx_buffer.copy_from_slice(&self.internal_buf.borrow()[..]);
            Ok(())
        }

        fn start_read(&self) {
            self.did_rx.store(true, Ordering::SeqCst);
            self.fire_interrupt();
        }

        fn finish_read(&self) -> Result<usize, Error> {
            if self.did_rx.load(Ordering::SeqCst) {
                Ok(self.internal_buf.borrow().len())
            } else {
                Ok(0)
            }
        }

        fn cancel_read(&self) {}

        fn process_interrupts(&self) -> (bool, bool) {
            (
                self.did_tx.swap(false, Ordering::SeqCst),
                self.did_rx.swap(false, Ordering::SeqCst),
            )
        }
    }

    struct TestIrq {}

    unsafe impl cortex_m::interrupt::Nr for TestIrq {
        fn nr(&self) -> u8 {
            0
        }
    }
    */

    /*
    #[test]
    fn test_read() {
        let testuart = TestHal::new();
        let uart: DmaUart<TestHal, TimerActor<TestTimer>> = DmaUart::new(testuart, TestIrq {});
    }
    */
}
