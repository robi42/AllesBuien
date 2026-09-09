//! Bounded materialisation and retention for one metadata-defined radar run.
//!
//! Four frame requests may be in flight: the bound is a courteous policy for a
//! free public CDN, while still letting three requests advance if one stalls.
//! Completed frames are indexed and sorted back into metadata order. Each body
//! reaches its final cache path only through a same-directory atomic rename, so
//! an interrupted temporary write never masquerades as a cached frame.

use std::path::Path;

use futures::{StreamExt, TryStreamExt, stream};

use crate::radar::{MAX_CACHED_RUNS, MetadataTime, RadarApi, RadarError, RadarFrame};

const MAX_CONCURRENT_FRAMES: usize = 4;

pub(crate) struct RadarCache<'a> {
    state_dir: &'a Path,
}

impl<'a> RadarCache<'a> {
    pub(crate) const fn new(state_dir: &'a Path) -> Self {
        Self { state_dir }
    }

    pub(crate) async fn materialize(
        &self,
        api: &dyn RadarApi,
        run_id: &str,
        times: Vec<MetadataTime>,
    ) -> Result<Vec<RadarFrame>, RadarError> {
        let run_dir = self.state_dir.join("runs").join(run_id);
        std::fs::create_dir_all(&run_dir)?;
        let downloads = times.into_iter().enumerate().map(|(index, time)| {
            let run_dir = &run_dir;
            async move {
                let path = run_dir.join(format!("frame-{index}.png"));
                if !path.exists() {
                    let temporary = path.with_extension("png.tmp");
                    let bytes = api.frame(&time.url).await?;
                    std::fs::write(&temporary, bytes)?;
                    std::fs::rename(temporary, &path)?;
                }
                Ok::<(usize, RadarFrame), RadarError>((
                    index,
                    RadarFrame {
                        timestamp: time.timestamp,
                        url: time.url,
                        path,
                    },
                ))
            }
        });
        let mut indexed_frames: Vec<(usize, RadarFrame)> = stream::iter(downloads)
            .buffer_unordered(MAX_CONCURRENT_FRAMES)
            .try_collect()
            .await?;
        indexed_frames.sort_unstable_by_key(|(index, _frame)| *index);
        prune_runs(self.state_dir, &run_dir)?;
        Ok(indexed_frames
            .into_iter()
            .map(|(_index, frame)| frame)
            .collect())
    }
}

pub(crate) fn prune_runs(state_dir: &Path, current_run: &Path) -> Result<(), RadarError> {
    let runs = state_dir.join("runs");
    let mut run_dirs = Vec::new();
    for entry in std::fs::read_dir(runs)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            run_dirs.push(entry.path());
        }
    }
    run_dirs.sort();
    let remove_count = run_dirs.len().saturating_sub(MAX_CACHED_RUNS);
    for old_run in run_dirs
        .into_iter()
        .filter(|run| run != current_run)
        .take(remove_count)
    {
        std::fs::remove_dir_all(old_run)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
