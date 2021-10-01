use crate::traits::ip::{IpProtocol, SocketAddress};
use crate::traits::tcp::{TcpError, TcpStack};
use core::future::Future;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use embassy_net::TcpSocket;

#[derive(Copy, Clone, Eq, PartialEq)]
enum State {
    Closed,
    Open,
    Connected,
}

impl Default for State {
    fn default() -> Self {
        State::Closed
    }
}

pub struct EthernetTcpStack<
    'a,
    const NUM_SOCKETS: usize,
    const RX_SIZE: usize,
    const TX_SIZE: usize,
> {
    rx_buffers: [[u8; RX_SIZE]; NUM_SOCKETS],
    tx_buffers: [[u8; TX_SIZE]; NUM_SOCKETS],
    states: [State; NUM_SOCKETS],
    _marker: PhantomData<&'a ()>,
}

impl<'a, const NUM_SOCKETS: usize, const RX_SIZE: usize, const TX_SIZE: usize>
    EthernetTcpStack<'a, NUM_SOCKETS, RX_SIZE, TX_SIZE>
{
    pub fn new() -> Self {
        Self {
            rx_buffers: [[0; RX_SIZE]; NUM_SOCKETS],
            tx_buffers: [[0; TX_SIZE]; NUM_SOCKETS],
            states: [State::Closed; NUM_SOCKETS],
            _marker: Default::default(),
        }
    }

    fn take(&mut self) -> Option<usize> {
        let index = self
            .states
            .iter()
            .enumerate()
            .find(|(_index, state)| **state == State::Closed)
            .map(|(index, _state)| index)?;
        self.states[index] = State::Open;
        Some(index)
    }
}

impl<'a, const NUM_SOCKETS: usize, const RX_SIZE: usize, const TX_SIZE: usize> TcpStack
    for EthernetTcpStack<'a, NUM_SOCKETS, RX_SIZE, TX_SIZE>
{
    type SocketHandle = (usize, TcpSocket<'a>);

    #[rustfmt::skip]
    type OpenFuture<'m> where 'a: 'm = impl Future<Output=Self::SocketHandle> + 'm;

    fn open<'m>(&'m mut self) -> Self::OpenFuture<'m> {
        async move {
            let index = self.take().ok_or(TcpError::ConnectError)?;
            let socket = TcpSocket::new(&mut self.rx_buffers[index], &mut self.tx_buffers[index]);
            Ok((index, socket))
        }
    }

    #[rustfmt::skip]
    type ConnectFuture<'m> where 'a: 'm = impl Future<Output=Result<(), TcpError>> + 'm;

    fn connect<'m>(
        &'m mut self,
        handle: Self::SocketHandle,
        proto: IpProtocol,
        dst: SocketAddress,
    ) -> Self::ConnectFuture<'m> {
        async move { todo!() }
    }

    #[rustfmt::skip]
    type WriteFuture<'m> where 'a: 'm = impl Future<Output=Result<usize, TcpError>> + 'm;

    fn write<'m>(&'m mut self, handle: Self::SocketHandle, buf: &'m [u8]) -> Self::WriteFuture<'m> {
        async move { todo!() }
    }

    #[rustfmt::skip]
    type ReadFuture<'m> where 'a: 'm = impl Future<Output=Result<usize, TcpError>> + 'm;

    fn read<'m>(
        &'m mut self,
        handle: Self::SocketHandle,
        buf: &'m mut [u8],
    ) -> Self::ReadFuture<'m> {
        async move { todo!() }
    }

    #[rustfmt::skip]
    type CloseFuture<'m> where 'a: 'm = impl Future<Output=()> + 'm;

    fn close<'m>(&'m mut self, handle: Self::SocketHandle) -> Self::CloseFuture<'m> {
        async move { todo!() }
    }
}
