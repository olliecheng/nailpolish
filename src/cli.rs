use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{Parser, Subcommand};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const INFO_STRING: &str = "
💅 nailpolish version ";
const AFTER_STRING: &str = "
   ──────────────────────────────────
   tools for consensus calling barcode and UMI duplicates
   https://github.com/olliecheng/nailpolish";

// colouring of the help
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::BrightMagenta.on_default().bold())
    .literal(AnsiColor::BrightMagenta.on_default())
    .placeholder(AnsiColor::White.on_default());

#[derive(Parser)]
#[command(
    version = VERSION,
    about = format!("{}{}{}", INFO_STRING, VERSION, AFTER_STRING),
    arg_required_else_help = true,
    flatten_help = true,
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create an index file from a demultiplexed .fastq, if one doesn't already exist
    #[command(arg_required_else_help = true)]
    GenerateIndex {
        /// the input .fastq file
        #[arg(long)]
        file: String,

        /// the output index file
        #[arg(long, default_value = "index.tsv")]
        index: String,
    },

    /// Generate a summary of duplicate statistics from an index file
    #[command(arg_required_else_help = true)]
    Summary {
        /// the index file
        #[arg(long)]
        index: String,
    },

    /// Generate a consensus-called 'cleaned up' file
    #[command(arg_required_else_help = true)]
    Call {
        /// the index file
        #[arg(long)]
        index: String,

        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the output .fasta; note that quality values are not preserved
        #[arg(long)]
        output: Option<String>,

        /// the number of threads to use
        #[arg(short, long, default_value_t = 4)]
        threads: usize,

        /// only show the duplicated reads, not the single ones
        #[arg(short, long, action)]
        duplicates_only: bool,

        /// for each duplicate group of reads, report the original reads along with the consensus
        #[arg(short, long, action)]
        report_original_reads: bool,
    },

    /// 'Group' duplicate reads, and pass to downstream applications.
    #[command(arg_required_else_help = true)]
    Group {
        /// the index file
        #[arg(long)]
        index: String,

        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the output location, or default to stdout
        #[arg(long)]
        output: Option<String>,

        /// the shell used to run the given command
        #[arg(long, default_value = "bash")]
        shell: String,

        /// the number of threads to use. this will not guard against race conditions in any
        /// downstream applications used. this will effectively set the number of individual
        /// processes to launch
        #[arg(short, long, default_value_t = 1)]
        threads: usize,

        /// the command to run. any groups will be passed as .fastq standard input.
        #[arg(trailing_var_arg = true, default_value = "cat")]
        command: Vec<String>,
    },
}
