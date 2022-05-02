use crate::backend::BackendOutput;

// TODO: PartialEq?
#[derive(Debug)]
pub struct Output(Box<dyn BackendOutput>);

impl Output {}

impl<O: BackendOutput + 'static> From<O> for Output {
    fn from(output: O) -> Self {
        Output(Box::new(output))
    }
}
