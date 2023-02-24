use super::Tree;
use std::any::Any;
use crate::newsletter::archive::Sink;

pub struct Context {
    pub tree: Tree,
    pub newsletter_sink: Sink,
}

// TODO: use enum dispatch instead of dynamic dispatch if the performance impact of the latter
// proves significant.

pub type ErasedCommandReturnType = Box<dyn Any + Send + 'static>;
pub type ErasedCommandFn = Box<dyn FnOnce(&mut Context) -> ErasedCommandReturnType + Send>;

pub struct ErasedCommand {
    operation: ErasedCommandFn,
}

impl ErasedCommand {
    pub fn run(self, context: &mut Context) -> ErasedCommandReturnType {
        (self.operation)(context)
    }
}

pub struct Command<R, F>
where
    F: FnOnce(&mut Context) -> R,
{
    operation: F,
}

impl<R, F> From<Command<R, F>> for ErasedCommand
where
    R: Any + Send + 'static,
    F: FnOnce(&mut Context) -> R + Send + 'static,
{
    fn from(cmd: Command<R, F>) -> Self {
        ErasedCommand {
            operation: Box::new(|context| Box::new((cmd.operation)(context))),
        }
    }
}

impl<R, F> Command<R, F>
where
    F: FnOnce(&mut Context) -> R,
{
    pub fn new(operation: F) -> Self {
        Self { operation }
    }
}
