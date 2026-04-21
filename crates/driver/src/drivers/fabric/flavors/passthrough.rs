use super::common::FlavorBehavior;

pub(super) static BEHAVIOR: PassthroughBehavior = PassthroughBehavior;

pub(super) struct PassthroughBehavior;

impl FlavorBehavior for PassthroughBehavior {}
