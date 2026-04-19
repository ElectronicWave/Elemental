#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AnyHost;
impl PartialEq<&str> for AnyHost {
    fn eq(&self, _: &&str) -> bool {
        true
    }
}

pub(crate) const ANY_HOST: AnyHost = AnyHost {};
