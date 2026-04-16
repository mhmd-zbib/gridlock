#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    MovingToLastKnown,
    Scanning,
    HoldingGap,
    ReturningToAnchor,
    Done,
}
