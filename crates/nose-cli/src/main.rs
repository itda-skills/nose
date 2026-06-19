//! `nose` — multi-language code clone detector CLI.

use anyhow::Result;

fn main() -> Result<()> {
    nose_cli::install_broken_pipe_guard();
    // rayon executes tasks both on its pool workers AND inline on the calling thread,
    // so enlarge the workers' stacks here and run the command body on a big-stack
    // thread below — otherwise a deep file lowered inline on a normal-stack thread
    // still overflows.
    let _ = rayon::ThreadPoolBuilder::new()
        .stack_size(nose_cli::STACK_SIZE)
        .build_global();
    std::thread::Builder::new()
        .stack_size(nose_cli::STACK_SIZE)
        .spawn(nose_cli::run_command)
        .expect("spawn worker thread")
        .join()
        .expect("worker thread panicked")
}
