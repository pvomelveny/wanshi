// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::{io::Write, sync::mpsc::RecvTimeoutError, time::Instant};

use camino::{Utf8Path, Utf8PathBuf};
use notify::{Config, RecommendedWatcher, Watcher};

use super::strategy::{
    default_watch_strategy, display_watch_path, watch_mode_for_path, MissingPathLevel,
    WatchBatcher, WatchChangeFoldState, WatchStrategy,
};

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
pub(in crate::cli::serve) fn watch_paths<P: AsRef<Utf8Path>, F>(
    watched_paths: &[P],
    assets_dir: &Utf8Path,
    quiet: bool,
    mut action: F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    watch_paths_with_strategy(
        watched_paths,
        assets_dir,
        default_watch_strategy(),
        quiet,
        &mut action,
    )
}

fn watch_paths_with_strategy<P: AsRef<Utf8Path>, F>(
    watched_paths: &[P],
    assets_dir: &Utf8Path,
    strategy: WatchStrategy,
    quiet: bool,
    action: &mut F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let mut batcher = WatchBatcher::new(strategy.debounce);
    let mut fold_state = WatchChangeFoldState::default();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    if !quiet {
        print!("[watch] ");
    }
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        if !watched_path.exists() {
            let watched_path_display = display_watch_path(watched_path);
            match (strategy.missing_path_level)(watched_path, assets_dir) {
                MissingPathLevel::Hint => {
                    color_print::ceprintln!(
                        "<dim>[watch] Hint: Optional path \"{}\" does not exist, skipping.</>",
                        watched_path_display
                    );
                }
                MissingPathLevel::Warning => {
                    color_print::ceprintln!(
                        "<y>[watch] Warning: Path \"{}\" does not exist, skipping.</>",
                        watched_path_display
                    );
                }
            }
            continue;
        }

        let mode = watch_mode_for_path(watched_path);
        watcher.watch(watched_path.as_std_path(), mode)?;
        if !quiet {
            print!("\"{}\"  ", display_watch_path(watched_path));
        }
    }
    if !quiet {
        println!("\n\nPress Ctrl+C to stop watching.\n");
    }

    loop {
        let now = Instant::now();
        let res = match batcher.time_until_ready(now) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        match res {
            Ok(message) => {
                absorb_watch_message(message, &mut batcher, strategy);
                // Drain whatever else is already queued. One loop over the
                // payload, not one loop per variant: `while let Ok(Ok(..))`
                // stopped at the first error *after consuming it*, and its
                // twin for errors symmetrically consumed and dropped a real
                // event — a missed rebuild whenever the two interleaved.
                while let Ok(message) = rx.try_recv() {
                    absorb_watch_message(message, &mut batcher, strategy);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                let Some(changed_paths) = batcher.take_ready(Instant::now()) else {
                    continue;
                };
                process_batch(&changed_paths, &mut fold_state, strategy, quiet, action)?;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn process_batch<F>(
    changed_paths: &[Utf8PathBuf],
    fold_state: &mut WatchChangeFoldState,
    strategy: WatchStrategy,
    quiet: bool,
    action: &mut F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    if !quiet {
        for line in (strategy.format_change_lines)(fold_state, changed_paths) {
            println!("{line}");
        }
        std::io::stdout().flush()?;
    }
    if let Err(err) = action(changed_paths) {
        // A warning color should be used here, as rebuild failures during user editing are acceptable.
        color_print::ceprintln!("<y>[watch] Rebuild failed: {}</>", err);
    }
    Ok(())
}

/// Feed one watcher message into the batcher, or report it.
///
/// Generally only content changes (`ModifyKind::Data(_)`) would matter, but
/// notify-rs reports everything as `Modify(Any)` on Windows, so the strategy's
/// event filter is deliberately broad.
fn absorb_watch_message(
    message: notify::Result<notify::Event>,
    batcher: &mut WatchBatcher,
    strategy: WatchStrategy,
) {
    match message {
        Ok(event) => {
            if let Some(paths) = collect_event_paths(event, strategy) {
                batcher.push_paths(paths, Instant::now());
            }
        }
        Err(error) => {
            color_print::ceprintln!("<r>[watch] Error: {error:?}</>");
        }
    }
}

fn collect_event_paths(event: notify::Event, strategy: WatchStrategy) -> Option<Vec<Utf8PathBuf>> {
    if !(strategy.should_handle_event)(&event.kind) {
        return None;
    }
    Some(
        event
            .paths
            .iter()
            .filter_map(|path| Utf8PathBuf::from_path_buf(path.clone()).ok())
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::Duration};

    use notify::{event::ModifyKind, EventKind};

    use super::*;

    /// An error arriving between two events must cost neither of them. The old
    /// drain ran one `while let Ok(Ok(..))` loop and then one for errors; each
    /// stopped at the other's variant *after consuming it*, so an interleaved
    /// batch silently dropped a change and the rebuild never happened.
    #[test]
    fn test_events_survive_an_interleaved_error() {
        let strategy = default_watch_strategy();
        let mut batcher = WatchBatcher::new(Duration::ZERO);
        let event = |path: &str| {
            notify::Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from(path))
        };

        absorb_watch_message(Ok(event("trees/a.typ")), &mut batcher, strategy);
        absorb_watch_message(
            Err(notify::Error::generic("transient watcher error")),
            &mut batcher,
            strategy,
        );
        absorb_watch_message(Ok(event("trees/b.typ")), &mut batcher, strategy);

        let ready = batcher
            .take_ready(std::time::Instant::now())
            .expect("both events should be batched");
        assert_eq!(
            ready,
            vec![
                Utf8PathBuf::from("trees/a.typ"),
                Utf8PathBuf::from("trees/b.typ")
            ]
        );
    }
}
