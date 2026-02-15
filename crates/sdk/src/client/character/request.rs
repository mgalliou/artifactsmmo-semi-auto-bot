use std::marker::PhantomData;

use crate::char::request_handler::ResponseSchema;

pub struct Request<R, E> {
    request: PhantomData<R>,
    error: PhantomData<E>,
}

impl<R, E> Request<R, E> {
    fn new() -> Self {
        Self {
            request: PhantomData,
            error: PhantomData,
        }
    }

    fn send<F: Fn() -> Result<R, E>>(request_fn: F) -> Result<R, E>
    where
        R: ResponseSchema,
    {
        request_fn()
    }
}
