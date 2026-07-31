//! Process attach, memory scanning/matching engine, and memory read/write.

pub mod error;
pub mod maps;
pub mod scanroutines;
pub mod session;
pub mod sets;
pub mod swath;
pub mod value;

/// Returns a greeting for the given name.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests;
