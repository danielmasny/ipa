use rand::Rng;
use raw_ipa::{
    error::Error,
    ff::Fp32BitPrime,
    ipa_test_input,
    protocol::{ipa::ipa, BreakdownKey, MatchKey},
    test_fixture::{input::GenericReportTestInput, Runner, TestWorld, TestWorldConfig},
};
use std::{num::NonZeroUsize, time::Instant};
use raw_ipa::helpers::GatewayConfig;

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> Result<(), Error> {
    const BATCHSIZE: usize = 5000;
    const MAX_TRIGGER_VALUE: u128 = 5;
    const PER_USER_CAP: u32 = 3;
    const MAX_BREAKDOWN_KEY: u128 = 4;
    const NUM_MULTI_BITS: u32 = 3;

    let mut config = TestWorldConfig::default();
    let buffer_capacity = NonZeroUsize::new(BATCHSIZE.clamp(16, 1024)).unwrap();
    config.gateway_config = GatewayConfig::symmetric_buffers(buffer_capacity);
    let world = TestWorld::new_with(config).await;
    let mut rng = rand::thread_rng();

    let max_match_key = u128::try_from(BATCHSIZE / 4).unwrap();

    let mut records: Vec<GenericReportTestInput<Fp32BitPrime, MatchKey, BreakdownKey>> =
        Vec::with_capacity(BATCHSIZE);

    for _ in 0..BATCHSIZE {
        records.push(ipa_test_input!(
            {
                match_key: rng.gen_range(0..max_match_key),
                is_trigger_report: rng.gen::<u32>(),
                breakdown_key: rng.gen_range(0..MAX_BREAKDOWN_KEY),
                trigger_value: rng.gen_range(0..MAX_TRIGGER_VALUE),
            };
            (Fp32BitPrime, MatchKey, BreakdownKey)
        ));
    }

    let start = Instant::now();
    let result = world
        .semi_honest(records, |ctx, input_rows| async move {
            ipa::<Fp32BitPrime, MatchKey, BreakdownKey>(
                ctx,
                &input_rows,
                PER_USER_CAP,
                MAX_BREAKDOWN_KEY,
                NUM_MULTI_BITS,
            )
            .await
            .unwrap()
        })
        .await;

    let duration = start.elapsed();
    println!("rows {BATCHSIZE} benchmark complete after {duration:?}");

    assert_eq!(MAX_BREAKDOWN_KEY, result[0].len().try_into().unwrap());
    Ok(())
}
