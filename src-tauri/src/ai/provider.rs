#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    TextGeneration,
    ToolCalling,
    Vision,
}
