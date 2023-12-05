use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    process,
};

use clap::Parser;
use ipa_core::cli::Verbosity;
use rand::{rngs::StdRng, SeedableRng};
use tracing::{debug, error, info};

use crate::{gen_events::generate_events, sample::Sample};

const DEFAULT_EVENT_GEN_COUNT: u32 = 100_000;

#[derive(Debug, Parser)]
pub struct CommonArgs {
    #[clap(flatten)]
    pub logging: Verbosity,

    #[arg(short, long, global = true, help = "Write the result to the file.")]
    output_file: Option<PathBuf>,

    #[arg(long, global = true, help = "Overwrite the specified output file.")]
    overwrite: bool,
}

impl CommonArgs {
    fn get_output(&self) -> Result<Box<dyn io::Write>, io::Error> {
        match self.output_file {
            Some(ref path) => {
                let mut file = File::options();

                if self.overwrite {
                    file.truncate(true).create(true);
                } else {
                    file.create_new(true);
                }

                file.write(true)
                    .open(path)
                    .map(|f| Box::new(f) as Box<dyn io::Write>)
            }
            None => Ok(Box::new(io::stdout())),
        }
    }
}

#[derive(Debug, Parser)]
#[clap(name = "ipa_bench", about = "Synthetic data test harness for IPA")]
pub struct Args {
    #[clap(flatten)]
    pub common: CommonArgs,

    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Parser)]
#[clap(name = "command")]
pub enum Command {
    #[command(about = "Generate synthetic events.")]
    GenEvents {
        #[arg(
            short,
            long,
            default_value = "1",
            help = "Multiply the number of events generated by the scale factor. For example, --scale-factor=100 generates 10,000,000 synthetic events."
        )]
        scale_factor: u32,

        #[arg(
            short,
            long,
            help = "Random generator seed. Setting the seed allows reproduction of the synthetic data exactly."
        )]
        random_seed: Option<u64>,

        #[arg(
            short,
            long,
            default_value = "0",
            help = "Simulate ads created in this epoch. Impressions and conversions for a given ad may happen in the next epoch."
        )]
        epoch: u8,

        #[arg(
            short,
            long,
            help = "Configuration file containing distributions data."
        )]
        config_file: PathBuf,
    },
}

impl Command {
    pub fn dispatch(&self, common: &CommonArgs) {
        info!("Command {:?}", self);

        match self {
            Self::GenEvents {
                scale_factor,
                random_seed,
                epoch,
                config_file,
            } => {
                Command::gen_events(common, *scale_factor, random_seed, *epoch, config_file);
            }
        }
    }

    fn gen_events(
        common: &CommonArgs,
        scale_factor: u32,
        random_seed: &Option<u64>,
        epoch: u8,
        config_file: &Path,
    ) {
        let mut input = Command::get_input(&Some(config_file.to_path_buf())).unwrap_or_else(|e| {
            error!("Failed to open the input file. {}", e);
            process::exit(1);
        });

        let mut out = common.get_output().unwrap_or_else(|e| {
            error!("Failed to open the output file. {}", e);
            process::exit(1);
        });

        info!(
            "scale: {}, seed: {:?}, epoch: {}",
            scale_factor, random_seed, epoch
        );
        debug!(
            "Total number of events to generate: {}",
            DEFAULT_EVENT_GEN_COUNT * scale_factor
        );

        let config = serde_json::from_reader(&mut input).unwrap();
        let sample = Sample::new(&config);

        let mut rng = random_seed.map_or(StdRng::from_entropy(), StdRng::seed_from_u64);

        let (s_count, t_count) = generate_events(
            &sample,
            DEFAULT_EVENT_GEN_COUNT * scale_factor,
            epoch,
            &mut rng,
            &mut out,
        );

        info!("{} source events generated", s_count);
        info!("{} trigger events generated", t_count);
        info!(
            "trigger/source ratio: {}",
            f64::from(t_count) / f64::from(s_count)
        );
    }

    fn get_input(path: &Option<PathBuf>) -> Result<Box<dyn io::Read>, io::Error> {
        match path {
            Some(ref path) => File::open(path).map(|f| Box::new(f) as Box<dyn io::Read>),
            None => Ok(Box::new(io::stdin())),
        }
    }
}