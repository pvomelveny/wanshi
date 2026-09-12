// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::{
    collections::HashSet, ffi::OsString, io::Write, sync::mpsc::RecvTimeoutError, time::Instant,
};

use camino::{Utf8Path, Utf8PathBuf};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use super::strategy::{
    default_watch_strategy, display_watch_path, partition_watch_targets, MissingPathLevel,
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

    if !quiet {
        print!("[watch] ");
    }
    let mut existing_paths: Vec<&Utf8Path> = Vec::new();
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

        existing_paths.push(watched_path);
        if !quiet {
            print!("\"{}\"  ", display_watch_path(watched_path));
        }
    }
    if !quiet {
        println!("\n\nPress Ctrl+C to stop watching.\n");
    }

    let targets = partition_watch_targets(existing_paths);

    // Single files are watched through their parent directory — see
    // `WatchTargets` for why watching the file's own path goes dead on Linux —
    // and the parent watcher filters to the registered names so an unrelated
    // sibling edit does not trigger a rebuild.
    let file_names = targets.file_names.clone();
    let file_tx = tx.clone();
    let mut file_watcher = RecommendedWatcher::new(
        move |message: notify::Result<notify::Event>| {
            let keep = match &message {
                Ok(event) => event_names_a_registered_file(event, &file_names),
                Err(_) => true,
            };
            if keep {
                let _ = file_tx.send(message);
            }
        },
        Config::default(),
    )?;
    for parent in &targets.file_parents {
        file_watcher.watch(parent.as_std_path(), RecursiveMode::NonRecursive)?;
    }

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut dir_watcher = RecommendedWatcher::new(tx, Config::default())?;
    for dir in &targets.directories {
        dir_watcher.watch(dir.as_std_path(), RecursiveMode::Recursive)?;
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

/// Whether the event touches one of the single files registered with the
/// parent-directory watcher.
fn event_names_a_registered_file(event: &notify::Event, file_names: &HashSet<OsString>) -> bool {
    event.paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| file_names.contains(name))
    })
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

    /// The parent-directory watcher stands in for the single files, so its
    /// filter must pass exactly the registered names: the config saved by
    /// rename must still arrive, an unrelated sibling file must not.
    #[test]
    fn test_parent_watch_filter_passes_only_registered_file_names() {
        let file_names: HashSet<OsString> = [OsString::from("Wanshi.toml")].into();
        let event = |path: &str| {
            notify::Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from(path))
        };

        assert!(event_names_a_registered_file(
            &event("site/Wanshi.toml"),
            &file_names
        ));
        assert!(!event_names_a_registered_file(
            &event("site/README.md"),
            &file_names
        ));
    }
}
