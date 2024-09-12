use csv::ReaderBuilder;
use serde::Serialize;
use std::collections::BTreeMap;

use anyhow::{Result};
use indexmap::IndexMap;

pub type DuplicateMap = IndexMap<RecordIdentifier, Vec<usize>>;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct RecordIdentifier {
    pub bc: String,
    pub umi: String,
}

#[derive(Serialize, Debug)]
pub struct DuplicateStatistics {
    pub total_reads: usize,
    pub duplicate_reads: usize,
    pub duplicate_ids: usize,
    pub proportion_duplicate: f64,
    pub distribution: BTreeMap<usize, usize>,
}

pub fn get_duplicates(index: &str) -> Result<(DuplicateMap, DuplicateStatistics)> {
    let mut map = IndexMap::<RecordIdentifier, Vec<usize>>::new();
    let mut stats = DuplicateStatistics {
        total_reads: 0,
        duplicate_reads: 0,
        duplicate_ids: 0,
        proportion_duplicate: 0.0,
        distribution: BTreeMap::new(),
    };

    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(index)?;

    for read in reader.records() {
        let record = read?;
        stats.total_reads += 1;

        let id = RecordIdentifier {
            bc: record[1].to_string(),
            umi: record[4].to_string(),
        };

        let index = record[5].parse()?;
        if let Some(v) = map.get_mut(&id) {
            v.push(index);
        } else {
            map.insert(id, vec![index]);
        }
    }

    map.shrink_to_fit(); // optimise memory usage

    stats.duplicate_ids = 0;
    stats.duplicate_reads = map.values().map(|v| {
        let length = v.len();
        if length > 1 {
            stats.duplicate_ids += 1;

            if let Some(x) = stats.distribution.get_mut(&length) {
                *x += 1
            } else {
                stats.distribution.insert(length, 1);
            }
            length
        } else {
            0
        }
    })
        .sum();

    stats
        .distribution
        .insert(1, stats.total_reads - stats.duplicate_reads);

    stats.proportion_duplicate = stats.duplicate_reads as f64 / stats.total_reads as f64;

    Ok((map, stats))
}
