

use crate::Matcher;

pub enum Behavior<I, O> {
    Function(Box<dyn for<'a> Fn(I) -> O + Send + Sync + 'static>),
}

impl<I: Clone, O> Behavior<I, O> {
    pub fn called(&self, input: I) -> Option<O> {
        match self {
            Behavior::Function(function) => Some(function(input)),
            _ => {
                todo!()
            }
        }
    }
}

impl<F, I, O> From<F> for Behavior<I, O>
where
    F: for<'a> Fn(I) -> O + Send + Sync + 'static,
{
    fn from(function: F) -> Self {
        Behavior::Function(Box::new(function))
    }
}
