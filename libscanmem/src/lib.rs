//! Process attach, memory scanning/matching engine, and memory read/write.
//!
//! Placeholder — real modules land per [`docs/libscanmem-plan.md`](../../docs/libscanmem-plan.md).

/// Returns a greeting for the given name.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests;
