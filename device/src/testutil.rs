use crate::actors::button::{ButtonEvent, FromButtonEvent};
use crate::kernel::{
    actor::{Actor, ActorContext, ActorSpawner},
    device::DeviceContext,
    signal::SignalSlot,
    util::ImmediateFuture,
};
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};
use embassy::executor::{raw, SpawnError, Spawner};
use embassy::time::driver::{AlarmHandle, Driver};
use embassy::time::TICKS_PER_SECOND;
use embassy::traits::gpio::WaitForAnyEdge;
use embassy::util::Signal;
use embedded_hal::digital::v2::InputPin;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::time::Instant as StdInstant;
use std::vec::Vec;

#[derive(Clone, Copy)]
pub struct TestSpawner;

impl TestSpawner {
    pub fn new() -> Self {
        Self {}
    }
}

impl ActorSpawner for TestSpawner {
    fn start<A: Actor, const QUEUE_SIZE: usize>(
        &self,
        _actor: &'static ActorContext<'static, A, QUEUE_SIZE>,
    ) -> Result<(), SpawnError>
    where
        [SignalSlot<<A as Actor>::Response>; QUEUE_SIZE]: Default,
    {
        Ok(())
    }
}

/// A test context that can execute test for a given device
pub struct TestContext<D: 'static> {
    runner: &'static TestRunner,
    device: &'static DeviceContext<D>,
}

impl<D> TestContext<D> {
    pub fn new(runner: &'static TestRunner, device: &'static DeviceContext<D>) -> Self {
        Self { runner, device }
    }

    /// Configure context with a device
    pub fn configure(&mut self, device: D) {
        self.device.configure(device);
    }

    /// Create a test pin that can be used in tests
    pub fn pin(&mut self, initial: bool) -> TestPin {
        self.runner.pin(initial)
    }

    /// Create a signal that can be used in tests
    pub fn signal(&mut self) -> &'static TestSignal {
        self.runner.signal()
    }

    /// Mount the device, running the provided callback function.
    pub async fn mount<FUT: Future<Output = R>, F: FnOnce(&'static D) -> FUT, R>(
        &mut self,
        f: F,
    ) -> R {
        self.device.mount(f).await
    }
}

impl<D> Drop for TestContext<D> {
    fn drop(&mut self) {
        self.runner.done()
    }
}

/// A test message with an id that can be passed around to verify the system
#[derive(Copy, Clone)]
pub struct TestMessage(pub u32);

impl FromButtonEvent<TestMessage> for TestHandler {
    fn from(event: ButtonEvent) -> Option<TestMessage> {
        match event {
            ButtonEvent::Pressed => Some(TestMessage(0)),
            ButtonEvent::Released => Some(TestMessage(1)),
        }
    }
}

/// A dummy actor that does nothing
#[derive(Default)]
pub struct DummyActor {}

impl DummyActor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Actor for DummyActor {
    type Message<'m> = TestMessage;
    type OnStartFuture<'m> = ImmediateFuture;
    type OnMessageFuture<'m> = ImmediateFuture;

    fn on_start(self: Pin<&'_ mut Self>) -> Self::OnStartFuture<'_> {
        ImmediateFuture::new()
    }

    fn on_message<'m>(self: Pin<&'m mut Self>, _: Self::Message<'m>) -> Self::OnMessageFuture<'m> {
        ImmediateFuture::new()
    }
}

/// A test handler that carries a signal that is set on `on_message`
pub struct TestHandler {
    on_message: &'static TestSignal,
}

impl TestHandler {
    pub fn new(signal: &'static TestSignal) -> Self {
        Self { on_message: signal }
    }
}

impl Actor for TestHandler {
    type Configuration = ();
    type Message<'m> = TestMessage;
    type OnStartFuture<'m> = ImmediateFuture;
    type OnMessageFuture<'m> = ImmediateFuture;

    fn on_start(self: Pin<&'_ mut Self>) -> Self::OnStartFuture<'_> {
        ImmediateFuture::new()
    }

    fn on_message<'m>(
        self: Pin<&'m mut Self>,
        message: Self::Message<'m>,
    ) -> Self::OnMessageFuture<'m> {
        self.on_message.signal(message);
        ImmediateFuture::new()
    }
}

/// A Pin that implements some embassy and embedded_hal traits that can be used to drive device changes.
pub struct TestPin {
    inner: &'static InnerPin,
}

struct InnerPin {
    value: AtomicBool,
    signal: Signal<()>,
}

impl Copy for TestPin {}
impl Clone for TestPin {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}

impl TestPin {
    pub fn set_high(&self) {
        self.inner.set_value(true)
    }

    pub fn set_low(&self) {
        self.inner.set_value(false)
    }
}

impl InnerPin {
    pub fn new(initial: bool) -> Self {
        Self {
            value: AtomicBool::new(initial),
            signal: Signal::new(),
        }
    }

    fn set_value(&self, value: bool) {
        self.signal.reset();
        self.value.store(value, Ordering::SeqCst);
        self.signal.signal(());
    }

    fn get_value(&self) -> bool {
        self.value.load(Ordering::SeqCst)
    }

    fn wait_changed<'m>(&'m self) -> SignalFuture<'m> {
        SignalFuture {
            signal: &self.signal,
        }
    }
}

/// A future that awaits a signal
pub struct SignalFuture<'m> {
    signal: &'m Signal<()>,
}

impl<'m> Future for SignalFuture<'m> {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self.signal.poll_wait(cx);
        result
    }
}

impl WaitForAnyEdge for TestPin {
    type Future<'m> = SignalFuture<'m>;
    fn wait_for_any_edge<'m>(&'m mut self) -> Self::Future<'m> {
        self.inner.wait_changed()
    }
}

impl InputPin for TestPin {
    type Error = ();
    fn is_high(&self) -> Result<bool, ()> {
        Ok(self.inner.get_value())
    }
    fn is_low(&self) -> Result<bool, ()> {
        Ok(!self.inner.get_value())
    }
}

/// A generic signal construct that can be used across actor and test states.
pub struct TestSignal {
    signal: Signal<()>,
    value: RefCell<Option<TestMessage>>,
}

impl TestSignal {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
            value: RefCell::new(None),
        }
    }

    pub fn signal(&self, value: TestMessage) {
        self.value.borrow_mut().replace(value);
        self.signal.signal(())
    }

    pub fn message(&self) -> Option<TestMessage> {
        *self.value.borrow()
    }

    pub fn wait_signaled<'m>(&'m self) -> SignalFuture<'m> {
        SignalFuture {
            signal: &self.signal,
        }
    }
}

/// A test context that can execute test for a given device
pub struct TestRunner {
    inner: UnsafeCell<raw::Executor>,
    not_send: PhantomData<*mut ()>,
    signaler: Signaler,
    pins: UnsafeCell<Vec<InnerPin>>,
    signals: UnsafeCell<Vec<TestSignal>>,
    done: AtomicBool,
}

impl TestRunner {
    pub fn new() -> Self {
        unsafe {
            CLOCK_ZERO.as_mut_ptr().write(StdInstant::now());
        }

        Self {
            inner: UnsafeCell::new(raw::Executor::new(Signaler::signal, ptr::null_mut())),
            not_send: PhantomData,
            signaler: Signaler::new(),
            pins: UnsafeCell::new(Vec::new()),
            signals: UnsafeCell::new(Vec::new()),
            done: AtomicBool::new(false),
        }
    }

    pub fn initialize(&'static self, init: impl FnOnce(Spawner)) {
        unsafe { (&mut *self.inner.get()).set_signal_ctx(&self.signaler as *const _ as _) };
        init(unsafe { (&*self.inner.get()).spawner() });
    }

    pub fn run_until_idle(&'static self) {
        self.signaler.prepare();
        while self.signaler.should_run() {
            unsafe { (&*self.inner.get()).run_queued() };
        }
    }

    /// Create a test pin that can be used in tests
    pub fn pin(&'static self, initial: bool) -> TestPin {
        let pins = unsafe { &mut *self.pins.get() };
        pins.push(InnerPin::new(initial));
        TestPin {
            inner: &pins[pins.len() - 1],
        }
    }

    /// Create a signal that can be used in tests
    pub fn signal(&'static self) -> &'static TestSignal {
        let signals = unsafe { &mut *self.signals.get() };
        signals.push(TestSignal::new());
        &signals[signals.len() - 1]
    }

    pub fn done(&'static self) {
        self.done.store(true, Ordering::SeqCst);
    }

    pub fn is_done(&'static self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}

static mut CLOCK_ZERO: MaybeUninit<StdInstant> = MaybeUninit::uninit();

struct Signaler {
    run: AtomicBool,
}

impl Signaler {
    fn new() -> Self {
        Self {
            run: AtomicBool::new(false),
        }
    }

    fn prepare(&self) {
        self.run.store(true, Ordering::SeqCst);
    }

    fn should_run(&self) -> bool {
        self.run.swap(false, Ordering::SeqCst)
    }

    fn signal(ctx: *mut ()) {
        let this = unsafe { &*(ctx as *mut Self) };
        this.run.store(true, Ordering::SeqCst);
    }
}

static mut ALARM_AT: u64 = u64::MAX;
static mut NEXT_ALARM_ID: u8 = 0;

struct TimeDriver;
embassy::time_driver_impl!(TimeDriver);

impl Driver for TimeDriver {
    fn now() -> u64 {
        let zero = unsafe { CLOCK_ZERO.as_ptr().read() };
        let dur = StdInstant::now().duration_since(zero);
        dur.as_secs() * (TICKS_PER_SECOND as u64)
            + (dur.subsec_nanos() as u64) * (TICKS_PER_SECOND as u64) / 1_000_000_000
    }

    unsafe fn allocate_alarm() -> Option<AlarmHandle> {
        let r = NEXT_ALARM_ID;
        NEXT_ALARM_ID += 1;
        Some(AlarmHandle::new(r))
    }

    fn set_alarm_callback(_alarm: AlarmHandle, _callback: fn(*mut ()), _ctx: *mut ()) {}

    fn set_alarm(_alarm: AlarmHandle, timestamp: u64) {
        unsafe { ALARM_AT = ALARM_AT.min(timestamp) }
    }
}

// Perform a process step for an Actor, processing a single message
pub fn step_actor<A: Actor + Unpin, const QUEUE_SIZE: usize>(
    actor: &'static ActorContext<'static, A, QUEUE_SIZE>,
) where
    [SignalSlot<<A as Actor>::Response>; QUEUE_SIZE]: Default,
{
    let waker = futures::task::noop_waker_ref();
    let mut cx = std::task::Context::from_waker(waker);
    let mut actor_fut = actor.process();
    while unsafe {
        Pin::new_unchecked(&mut actor_fut)
            .poll(&mut cx)
            .is_pending()
    } {}
}
