fn main() {
    if let Err(error) = waif::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
