use super::AsyncConnWithInfo;

/// Controls whether to enable verbose tracing for connections.
///
/// When enabled (with the `tracing` feature), connections are wrapped to log I/O operations for
/// debugging.
#[derive(Clone, Copy)]
pub struct Verbose(pub(super) bool);

impl Verbose {
    pub const OFF: Verbose = Verbose(false);

    #[cfg_attr(not(feature = "tracing"), inline(always))]
    pub(super) fn wrap<T>(&self, conn: T) -> Box<dyn AsyncConnWithInfo>
    where
        T: AsyncConnWithInfo + 'static,
    {
        #[cfg(feature = "tracing")]
        if self.0 {
            return Box::new(sealed::Wrapper {
                id: crate::util::fast_random(),
                inner: conn,
            });
        }

        Box::new(conn)
    }
}

#[cfg(feature = "tracing")]
mod sealed {
    use std::{
        fmt,
        io::{self, IoSlice},
        pin::Pin,
        task::{Context, Poll},
    };

    use super::super::TlsInfoFactory;
    use crate::{
        core::{
            client::connect::{Connected, Connection},
            rt::{Read, Write},
        },
        tls::TlsInfo,
        util::Escape,
    };

    pub(super) struct Wrapper<T> {
        pub(super) id: u64,
        pub(super) inner: T,
    }

    impl<T: Connection + Read + Write + Unpin> Connection for Wrapper<T> {
        fn connected(&self) -> Connected {
            self.inner.connected()
        }
    }

    impl<T: Read + Write + Unpin> Read for Wrapper<T> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            match Pin::new(&mut self.inner).poll_read(cx, buf) {
                Poll::Ready(Ok(n)) => {
                    trace!("{:08x} read: {:?}", self.id, Escape::new(&buf[..n]));
                    Poll::Ready(Ok(n))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl<T: Read + Write + Unpin> Write for Wrapper<T> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            match Pin::new(&mut self.inner).poll_write(cx, buf) {
                Poll::Ready(Ok(n)) => {
                    trace!("{:08x} write: {:?}", self.id, Escape::new(&buf[..n]));
                    Poll::Ready(Ok(n))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn poll_write_vectored(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize, io::Error>> {
            match Pin::new(&mut self.inner).poll_write_vectored(cx, bufs) {
                Poll::Ready(Ok(nwritten)) => {
                    trace!(
                        "{:08x} write (vectored): {:?}",
                        self.id,
                        Vectored { bufs, nwritten }
                    );
                    Poll::Ready(Ok(nwritten))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), std::io::Error>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), std::io::Error>> {
            Pin::new(&mut self.inner).poll_close(cx)
        }
    }

    impl<T: TlsInfoFactory> TlsInfoFactory for Wrapper<T> {
        fn tls_info(&self) -> Option<TlsInfo> {
            self.inner.tls_info()
        }
    }

    struct Vectored<'a, 'b> {
        bufs: &'a [IoSlice<'b>],
        nwritten: usize,
    }

    impl fmt::Debug for Vectored<'_, '_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut left = self.nwritten;
            for buf in self.bufs.iter() {
                if left == 0 {
                    break;
                }
                let n = std::cmp::min(left, buf.len());
                Escape::new(&buf[..n]).fmt(f)?;
                left -= n;
            }
            Ok(())
        }
    }
}
