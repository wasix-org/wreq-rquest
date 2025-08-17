use std::{
    pin::Pin,
    task::{Poll, ready},
};

pub use futures_util::io::{AsyncRead as Read, AsyncWrite as Write};

pub(crate) async fn read<T>(io: &mut T, buf: &mut [u8]) -> Result<usize, std::io::Error>
where
    T: Read + Unpin,
{
    std::future::poll_fn(move |cx| {
        ready!(Pin::new(&mut *io).poll_read(cx, buf))?;
        Poll::Ready(Ok(buf.len()))
    })
    .await
}

pub(crate) async fn write_all<T>(io: &mut T, buf: &[u8]) -> Result<(), std::io::Error>
where
    T: Write + Unpin,
{
    let mut n = 0;
    std::future::poll_fn(move |cx| {
        while n < buf.len() {
            n += ready!(Pin::new(&mut *io).poll_write(cx, &buf[n..])?);
        }
        Poll::Ready(Ok(()))
    })
    .await
}
