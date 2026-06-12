fn main() {
    uniffi::generate_scaffolding("src/text.udl").expect("failed to generate UniFFI scaffolding");
}
