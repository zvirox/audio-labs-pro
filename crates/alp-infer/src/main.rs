//! `alp-infer` — the out-of-process inference worker.
//!
//! Phase 1 establishes the process boundary. The worker will load models and
//! speak the versioned Unix-socket protocol owned by `alp-voice` in Phase 6.

fn main() {
    eprintln!("alp-infer: model backends arrive in Phase 6");
}
