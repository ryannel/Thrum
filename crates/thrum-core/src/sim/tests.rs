/// A fixture refusal with an owned error identity, independent of message classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TestError(pub(super) &'static str);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

mod rule_boundaries;
