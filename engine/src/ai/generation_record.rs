use crate::ai::generation_stats::GenerationStatistics;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GenerationRecord {
    file: File,
    path: PathBuf,
}

impl GenerationRecord {
    pub fn new() -> io::Result<Self> {
        let system_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let filename = format!("generation-record-{}.csv", system_time.as_secs());
        let path = PathBuf::from(filename);
        let file = File::create(&path)?;
        let mut record = Self { file, path };

        // Write CSV header
        writeln!(record.file, "Generation,Phase,Fitness,Score,Cleared,Bonus,Pieces,Fitness P95,Score P95,Cleared P95,Bonus P95,Fitness P50,Score P50,Cleared P50,Bonus P50,Seed,Genome")?;
        Ok(record)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn add<const N: usize>(&mut self, stats: &GenerationStatistics<N>) -> io::Result<()> {
        writeln!(
            self.file,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",\"{}\"",
            stats.id(),
            stats.objective(),
            stats.objective().fitness(&stats.max().result()),
            stats.max().result().score(),
            stats.max().result().cleared(),
            stats.max().result().bonus(),
            stats.max().result().pieces(),
            stats.objective().fitness(&stats.p95().result()),
            stats.p95().result().score(),
            stats.p95().result().cleared(),
            stats.p95().result().bonus(),
            stats.objective().fitness(&stats.median().result()),
            stats.median().result().score(),
            stats.median().result().cleared(),
            stats.median().result().bonus(),
            stats.seed(),
            stats.max().genome()
        )
    }
}
