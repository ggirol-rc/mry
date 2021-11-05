mod logs;
mod mock_result;
use std::iter::repeat;
use std::{fmt::Debug, ops::DerefMut};

pub use logs::*;
pub use mock_result::*;

use parking_lot::{Mutex, RwLock};

use crate::{Behavior, Matcher, Output, Rule};

pub type BoxMockObject<I, O> = Box<dyn MockObject<I, O> + Send + Sync>;

#[doc(hidden)]
pub trait MockObject<I, O> {
    fn record_call_and_find_mock_output(&mut self, input: I) -> Option<O>;
    fn returns_with(&mut self, matcher: Matcher<I>, behavior: Behavior<I, O>);
    fn calls_real_impl(&mut self, matcher: Matcher<I>);
    fn assert_called(&self, matcher: Matcher<I>) -> MockResult<I>;
}

// Separated because implementention needs Clone for O.
#[doc(hidden)]
pub trait MockObjectReturns<I, O> {
    fn returns(&mut self, matcher: Matcher<I>, ret: O);
}

pub struct Mock<I, O> {
    pub name: &'static str,
    logs: Mutex<Logs<I>>,
    rules: Vec<Rule<I, O>>,
}

impl<I, O> Mock<I, O> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            logs: Default::default(),
            rules: Default::default(),
        }
    }
}

impl<I: PartialEq + Clone, O> Mock<I, O> {
    fn handle_assert_called(&self, matcher: &Matcher<I>, f: impl FnOnce()) -> Logs<I> {
        let logs = self.logs.lock().filter_matches(matcher);
        if logs.is_empty() {
            f();
        }
        logs
    }
}

impl<I: Clone + PartialEq + Debug, O: Debug> MockObject<I, O> for Mock<I, O> {
    fn returns_with(&mut self, matcher: Matcher<I>, behavior: Behavior<I, O>) {
        self.rules.push(Rule {
            matcher,
            behavior: behavior,
        });
    }

    fn calls_real_impl(&mut self, matcher: Matcher<I>) {
        self.rules.push(Rule {
            matcher,
            behavior: Behavior::CallsRealImpl,
        })
    }

    fn assert_called(&self, matcher: Matcher<I>) -> MockResult<I> {
        let logs = self.handle_assert_called(&matcher, || {
            panic!("{} was not called\n{:?}", self.name, *self.logs.lock())
        });
        MockResult {
            name: self.name,
            logs,
        }
    }

    fn record_call_and_find_mock_output(&mut self, input: I) -> Option<O> {
        self.logs.lock().push(input.clone());
        for rule in &mut self.rules {
            match rule.called(&input) {
                Output::Found(output) => return Some(output),
                Output::NotMatches => {}
                Output::CallsRealImpl => return None,
            };
        }
        panic!("mock not found for {}\n{:?}", self.name, self.rules)
    }
}
impl<I, O, T> MockObjectReturns<I, O> for T
where
    O: Clone + Send + Sync + 'static,
    T: DerefMut<Target = BoxMockObject<I, O>>,
{
    fn returns(&mut self, matcher: Matcher<I>, ret: O) {
        self.returns_with(matcher, Behavior::Const(RwLock::new(Box::new(repeat(ret)))))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Behavior1;

    #[test]
    fn returns_with() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        assert_eq!(
            mock.record_call_and_find_mock_output(3),
            "aaa".to_string().into()
        );
    }

    #[test]
    fn returns() {
        let mut mock: Box<BoxMockObject<_, _>> =
            Box::new(Box::new(Mock::<usize, String>::new("a")));
        mock.returns(Matcher::Any, "a".repeat(3));

        assert_eq!(
            mock.record_call_and_find_mock_output(3),
            "aaa".to_string().into()
        );

        // allows called multiple times
        assert_eq!(
            mock.record_call_and_find_mock_output(3),
            "aaa".to_string().into()
        );
    }

    #[test]
    #[should_panic(expected = "mock not found for a")]
    fn returns_with_never() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Never, Behavior1::from(|a| "a".repeat(a)).into());

        mock.record_call_and_find_mock_output(3);
    }

    #[test]
    fn returns_with_always() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        assert_eq!(
            mock.record_call_and_find_mock_output(3),
            "aaa".to_string().into()
        );
    }

    #[test]
    #[should_panic(expected = "mock not found for a")]
    fn returns_never() {
        let mut mock: Box<BoxMockObject<_, _>> =
            Box::new(Box::new(Mock::<usize, String>::new("a")));
        mock.returns(Matcher::Never, "a".repeat(3));

        mock.record_call_and_find_mock_output(3);
    }

    #[test]
    fn returns_always() {
        let mut mock: Box<BoxMockObject<_, _>> =
            Box::new(Box::new(Mock::<usize, String>::new("a")));
        mock.returns(Matcher::Any, "a".repeat(3));

        assert_eq!(
            mock.record_call_and_find_mock_output(3),
            "aaa".to_string().into()
        );
    }

    #[test]
    fn calls_real_impl() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.calls_real_impl(Matcher::Eq(3));

        assert_eq!(mock.record_call_and_find_mock_output(3), None);
    }

    #[test]
    #[should_panic(expected = "mock not found for a")]
    fn calls_real_impl_never() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.calls_real_impl(Matcher::Eq(3));

        mock.record_call_and_find_mock_output(2);
    }

    #[test]
    fn assert_called_with() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        mock.record_call_and_find_mock_output(3);

        mock.assert_called(Matcher::Eq(3));
    }

    #[test]
    #[should_panic(expected = "a was not called")]
    fn assert_called_with_not_eq() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        mock.record_call_and_find_mock_output(3);

        mock.assert_called(Matcher::Eq(2));
    }

    #[test]
    #[should_panic(expected = "a was not called")]
    fn assert_called_with_panics() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        mock.assert_called(Matcher::Eq(3));
    }

    #[test]
    #[should_panic(expected = "[1, 2, 2]")]
    fn assert_called_with_log() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());
        mock.record_call_and_find_mock_output(1);
        mock.record_call_and_find_mock_output(2);
        mock.record_call_and_find_mock_output(2);

        mock.assert_called(Matcher::Eq(3));
    }

    #[test]
    fn assert_called_returns_logs() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        mock.record_call_and_find_mock_output(3);
        mock.record_call_and_find_mock_output(3);
        mock.record_call_and_find_mock_output(2);

        assert_eq!(
            mock.assert_called(Matcher::Any),
            MockResult {
                name: "a",
                logs: Logs(vec![3, 3, 2]),
            }
        );
    }

    #[test]
    fn assert_called_returns_logs_matching() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(Matcher::Any, Behavior1::from(|a| "a".repeat(a)).into());

        mock.record_call_and_find_mock_output(2);
        mock.record_call_and_find_mock_output(3);
        mock.record_call_and_find_mock_output(3);
        mock.record_call_and_find_mock_output(2);

        assert_eq!(
            mock.assert_called(Matcher::Eq(2)),
            MockResult {
                name: "a",
                logs: Logs(vec![2, 2]),
            }
        );
    }

    #[test]
    #[should_panic(
        expected = "[Rule { matcher: Eq(3), behavior: Function(_) }, Rule { matcher: Eq(3), behavior: CallsRealImpl }]"
    )]
    fn mock_not_found_with_rules() {
        let mut mock = Mock::<usize, String>::new("a");
        mock.returns_with(
            Matcher::Eq(3),
            Behavior::Function(Box::new(|_| "42".to_string())),
        );
        mock.calls_real_impl(Matcher::Eq(3));

        mock.record_call_and_find_mock_output(2);
    }
}
