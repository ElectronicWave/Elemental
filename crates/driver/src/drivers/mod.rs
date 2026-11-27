#[cfg(feature = "cleanroom")]
pub mod cleanroom;
#[cfg(feature = "fabric")]
pub mod fabric;
#[cfg(feature = "forge")]
pub mod forge;
#[cfg(feature = "neoforge")]
pub mod neoforge;
#[cfg(feature = "outlands")]
pub mod outlands;
#[cfg(feature = "quilt")]
pub mod quilt;
#[cfg(any(feature = "cleanroom", feature = "outlands"))]
pub mod retrofutura;
#[cfg(feature = "rift")]
pub mod rift;
pub mod vanilla;
