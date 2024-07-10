use crate::duplicates::DuplicateMap;
use crate::duplicates::RecordIdentifier;
use bio::io::fastq;
use bio::io::fastq::FastqRead;

use std;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};

use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rayon::{self};

use spoa::{self};

struct DuplicateRecord {
    id: RecordIdentifier,
    records: Vec<fastq::Record>,
}

pub fn consensus<R: Write + Send>(
    input: &str,
    writer: &Arc<Mutex<R>>,
    duplicates: DuplicateMap,
    threads: u8,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<(), Box<dyn Error>> {
    // set number of threads that Rayon uses
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .unwrap();

    let mut file = File::open(input)?;
    //let file = Arc::new(Mutex::new(file));

    duplicates
        .into_iter()
        // first, read from the file (sequential)
        .filter_map(|(id, positions)| {
            // if we choose not to only output duplicates, we can skip over this
            if (positions.len() == 1) && (duplicates_only) {
                return None;
            }

            let mut rec = DuplicateRecord {
                id,
                records: Vec::new(),
            };

            for pos in positions.iter() {
                let mut record = fastq::Record::new();
                file.seek(SeekFrom::Start(*pos as u64))
                    .expect("Reading from file should not fail!");

                let mut reader = fastq::Reader::new(&mut file);
                reader.read(&mut record).unwrap();
                rec.records.push(record);
            }
            Some(rec)
        })
        // convert this sequential iterator into a parallel one for consensus calling
        .par_bridge()
        .for_each(|rec| {
            let single = rec.records.len() == 1;

            let mut poa_graph;
            if single {
                let consensus = std::str::from_utf8(rec.records[0].seq()).unwrap();

                let writer = Arc::clone(writer);
                let mut writer = writer.lock().unwrap();

                writeln!(
                    writer,
                    ">{0}_{1}_SIN\n{2}",
                    rec.id.bc, rec.id.umi, consensus
                )
                .unwrap();
            } else {
                let mut alignment_engine =
                    spoa::AlignmentEngine::new(spoa::AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
                poa_graph = spoa::Graph::new();

                let record_count = rec.records.len();

                let mut existing_reads: Option<String> = if output_originals {
                    Some(String::new())
                } else {
                    None
                };

                for (index, record) in rec.records.iter().enumerate() {
                    let seq = record.seq();
                    let qual = record.qual();

                    if output_originals {
                        let s = format!(
                            ">{0}_{1}_DUP_{2}_of_{3}\n{4}\n",
                            rec.id.bc,
                            rec.id.umi,
                            index + 1,
                            record_count,
                            std::str::from_utf8(seq).unwrap()
                        );
                        match existing_reads.as_mut() {
                            Some(x) => x.push_str(&s),
                            None => {}
                        }
                    }

                    let align = alignment_engine.align_from_bytes(seq, &poa_graph);
                    poa_graph.add_alignment_from_bytes(&align, seq, &qual);
                }

                let consensus = poa_graph.consensus();
                let consensus = consensus
                    .to_str()
                    .expect("spoa module should produce valid utf-8");

                let writer = Arc::clone(writer);
                let mut writer = writer.lock().unwrap();

                if output_originals {
                    // unwrap is fine here, as this is only Some() if output_originals is set
                    match existing_reads {
                        Some(s) => {
                            writer.write_all(s.as_bytes()).unwrap();
                        }
                        None => {}
                    };
                }

                // this is repeated, but I'm not sure how to pass out
                // a string slice from the if scope without creating
                // a borrow checker error.
                writeln!(
                    writer,
                    ">{0}_{1}_CON_{2}\n{3}",
                    rec.id.bc, rec.id.umi, record_count, consensus
                )
                .unwrap();
            };
        });

    return Ok(());
}
