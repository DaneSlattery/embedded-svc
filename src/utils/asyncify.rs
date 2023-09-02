#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod event_bus;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod mqtt;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod timer;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod ws;

pub use async_wrapper::*;

#[cfg(feature = "alloc")]
pub use blocking_unblocker::*;

mod async_wrapper {
    pub trait AsyncWrapper<S> {
        fn new(sync: S) -> Self;
    }

    pub trait Asyncify {
        type AsyncWrapper<S>: AsyncWrapper<S>;

        fn into_async(self) -> Self::AsyncWrapper<Self>
        where
            Self: Sized,
        {
            Self::AsyncWrapper::new(self)
        }

        fn as_async(&mut self) -> Self::AsyncWrapper<&mut Self> {
            Self::AsyncWrapper::new(self)
        }
    }

    pub trait UnblockingAsyncWrapper<U, S> {
        fn new(unblocker: U, sync: S) -> Self;
    }

    pub trait UnblockingAsyncify {
        type AsyncWrapper<U, S>: UnblockingAsyncWrapper<U, S>;

        fn unblock_into_async<U>(self, unblocker: U) -> Self::AsyncWrapper<U, Self>
        where
            Self: Sized,
        {
            Self::AsyncWrapper::new(unblocker, self)
        }

        fn unblock_as_async<U>(&mut self, unblocker: U) -> Self::AsyncWrapper<U, &mut Self> {
            Self::AsyncWrapper::new(unblocker, self)
        }
    }
}

#[cfg(feature = "alloc")]
mod blocking_unblocker {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::task::Poll;

    extern crate alloc;

    use alloc::boxed::Box;

    #[derive(Clone)]
    pub struct BlockingUnblocker(());

    impl BlockingUnblocker {
        pub fn unblock<F, T>(&self, f: F) -> BlockingFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            BlockingFuture::new(f)
        }
    }

    impl crate::executor::asynch::Unblocker for BlockingUnblocker {
        type UnblockFuture<T> = BlockingFuture<T> where T: Send;

        fn unblock<F, T>(&self, f: F) -> Self::UnblockFuture<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            BlockingUnblocker::unblock(self, f)
        }
    }

    // #[cfg(feature = "nightly")]
    // impl crate::executor::asynch::Unblocker for BlockingUnblocker {
    //     async fn unblock<F, T>(&self, f: F) -> T
    //     where
    //         F: FnOnce() -> T + Send + 'static,
    //         T: Send + 'static,
    //     {
    //         BlockingUnblocker::unblock(self, f).await
    //     }
    // }

    pub fn blocking_unblocker() -> BlockingUnblocker {
        BlockingUnblocker(())
    }

    pub struct BlockingFuture<T> {
        // TODO: Need to box or else we get rustc error:
        // "type parameter `F` is part of concrete type but not used in parameter list for the `impl Trait` type alias"
        computation: Option<Box<dyn FnOnce() -> T + Send + 'static>>,
        _result: PhantomData<fn() -> T>,
    }

    impl<T> BlockingFuture<T> {
        fn new<F>(computation: F) -> Self
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            Self {
                computation: Some(Box::new(computation)),
                _result: PhantomData,
            }
        }
    }

    impl<T> Future for BlockingFuture<T>
    where
        T: Send,
    {
        type Output = T;

        fn poll(
            mut self: core::pin::Pin<&mut Self>,
            _cx: &mut core::task::Context<'_>,
        ) -> Poll<Self::Output> {
            let computation = self.computation.take();

            if let Some(computation) = computation {
                Poll::Ready((computation)())
            } else {
                unreachable!()
            }
        }
    }
}
