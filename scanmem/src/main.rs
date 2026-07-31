// Placeholder entry point — real CLI lands per docs/scanmem-plan.md.
fn main() {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "world".to_owned());
    println!("{}", libscanmem::greet(&name));
}

#[cfg(test)]
mod tests;
