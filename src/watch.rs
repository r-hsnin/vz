//! File watching mode: re-renders chart when input file changes.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};

/// Run the oneshot renderer in a watch loop, re-rendering when the file changes.
/// Watches `file` for modifications and calls `render_fn` on each change.
pub fn run_watch<F>(file: &Path, mut render_fn: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    // Validate: watch mode requires a real file, not stdin
    if file == Path::new("-") {
        anyhow::bail!("--watch cannot be used with stdin input");
    }
    if !file.exists() {
        anyhow::bail!("Cannot watch '{}': file not found", file.display());
    }

    // Initial render
    eprintln!(
        "Watching {} for changes (Ctrl-C to stop)...",
        file.display()
    );
    render_fn()?;

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                // Only trigger on data modifications (not metadata-only changes)
                let dominated = matches!(
                    event.kind,
                    notify::EventKind::Modify(ModifyKind::Data(_))
                        | notify::EventKind::Modify(ModifyKind::Any)
                        | notify::EventKind::Create(_)
                );
                if dominated {
                    let _ = tx.send(());
                }
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    // Watch the parent directory (more reliable for atomic writes that replace files)
    let watch_path = file.parent().unwrap_or(Path::new("."));
    watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

    // Debounce: wait at least 200ms between re-renders to avoid flicker
    while let Ok(()) = rx.recv() {
        // Drain any queued events (debounce)
        while rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(200));
        while rx.try_recv().is_ok() {}

        // Clear screen and re-render
        eprint!("\x1b[2J\x1b[H"); // ANSI clear screen + move cursor to top
        eprintln!("Re-rendering... ({})", file.display());
        if let Err(e) = render_fn() {
            eprintln!("Error: {:#}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_watch_rejects_stdin() {
        let result = run_watch(Path::new("-"), || Ok(()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stdin"));
    }

    #[test]
    fn test_watch_rejects_nonexistent_file() {
        let result = run_watch(Path::new("/tmp/this_file_does_not_exist_12345.csv"), || {
            Ok(())
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("file not found"));
    }

    #[test]
    fn test_watch_calls_render_fn_initially() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "x,y\na,1").unwrap();
        tmpfile.flush().unwrap();

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        // Run in a thread and kill after initial render
        let path = tmpfile.path().to_path_buf();
        let handle = std::thread::spawn(move || {
            let _ = run_watch(&path, || {
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                // Return error to break out of the watch loop after initial render
                // (this is the simplest way to test without complex signal handling)
                anyhow::bail!("test: stop after first render");
            });
        });

        // Give it time to run
        std::thread::sleep(Duration::from_millis(300));
        // The thread should have completed (error breaks the loop)
        let _ = handle.join();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
