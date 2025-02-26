#![macro_use]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "std")]
mod tests {
    use drogue_device::*;
    use embassy::executor::Spawner;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::{sync::mpsc, thread, time::Duration};

    static INITIALIZED: AtomicU32 = AtomicU32::new(0);

    #[test]
    fn test_device_setup() {
        pub struct MyActor {
            value: &'static AtomicU32,
        }

        pub struct Add(u32);
        impl Actor for MyActor {
            type Configuration = ();
            type Message<'a> = Add;
            type OnStartFuture<'a> = ImmediateFuture;
            type OnMessageFuture<'a> = ImmediateFuture;

            fn on_start(self: core::pin::Pin<&'_ mut Self>) -> Self::OnStartFuture<'_> {
                ImmediateFuture::new()
            }

            fn on_message<'m>(
                self: core::pin::Pin<&'m mut Self>,
                message: Self::Message<'m>,
            ) -> Self::OnMessageFuture<'m> {
                self.value.fetch_add(message.0, Ordering::SeqCst);
                ImmediateFuture::new()
            }
        }

        struct MyDevice {
            a: ActorContext<'static, MyActor>,
        }

        #[embassy::main]
        async fn main(spawner: Spawner) {
            static DEVICE: DeviceContext<MyDevice> = DeviceContext::new();

            DEVICE.configure(MyDevice {
                a: ActorContext::new(MyActor {
                    value: &INITIALIZED,
                }),
            });

            let a_addr = DEVICE
                .mount(|device| async move { device.a.mount((), spawner) })
                .await;

            a_addr.request(Add(10)).unwrap().await;
        }

        std::thread::spawn(move || {
            main();
        });

        panic_after(Duration::from_secs(10), move || {
            while INITIALIZED.load(Ordering::SeqCst) != 10 {
                std::thread::sleep(Duration::from_secs(1))
            }
        })
    }

    fn panic_after<T, F>(d: Duration, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T,
        F: Send + 'static,
    {
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let val = f();
            done_tx.send(()).expect("Unable to send completion signal");
            val
        });

        match done_rx.recv_timeout(d) {
            Ok(_) => handle.join().expect("Thread panicked"),
            Err(_) => panic!("Thread took too long"),
        }
    }
}
