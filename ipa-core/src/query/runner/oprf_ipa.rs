use std::marker::PhantomData;

use futures::TryStreamExt;

use crate::{
    error::Error,
    ff::{
        boolean::Boolean,
        boolean_array::{BA20, BA3, BA4, BA5, BA6, BA7, BA8},
        Field, PrimeField, Serializable,
    },
    helpers::{
        query::{IpaQueryConfig, QuerySize},
        BodyStream, RecordsStream,
    },
    protocol::{
        basics::ShareKnownValue,
        context::{UpgradableContext, UpgradedContext},
        ipa_prf::prf_sharding::{
            attribute_cap_aggregate, compute_histogram_of_users_with_row_count,
            PrfShardedIpaInputRow,
        },
    },
    report::{EventType, OprfReport},
    secret_sharing::{
        replicated::{malicious::ExtendableField, semi_honest::AdditiveShare as Replicated},
        SharedValue,
    },
};

pub struct OprfIpaQuery<C, F> {
    config: IpaQueryConfig,
    phantom_data: PhantomData<(C, F)>,
}

impl<C, F> OprfIpaQuery<C, F> {
    pub fn new(config: IpaQueryConfig) -> Self {
        Self {
            config,
            phantom_data: PhantomData,
        }
    }
}

#[allow(clippy::too_many_lines)]
impl<C, F> OprfIpaQuery<C, F>
where
    C: UpgradableContext,
    C::UpgradedContext<F>: UpgradedContext<F, Share = Replicated<F>>,
    C::UpgradedContext<Boolean>: UpgradedContext<Boolean, Share = Replicated<Boolean>>,
    F: PrimeField + ExtendableField,
    Replicated<F>: Serializable + ShareKnownValue<C, F>,
    Replicated<Boolean>: Serializable + ShareKnownValue<C, Boolean>,
{
    #[tracing::instrument("oprf_ipa_query", skip_all, fields(sz=%query_size))]
    pub async fn execute<'a>(
        self,
        ctx: C,
        query_size: QuerySize,
        input_stream: BodyStream,
    ) -> Result<Vec<Replicated<F>>, Error> {
        let Self {
            config,
            phantom_data: _,
        } = self;
        tracing::info!("New query: {config:?}");
        let sz = usize::from(query_size);

        let input = if config.plaintext_match_keys {
            let mut v = RecordsStream::<OprfReport<BA20, BA8, BA3>, _>::new(input_stream)
                .try_concat()
                .await?;
            v.truncate(sz);
            v
        } else {
            panic!("Encrypted match key handling is not handled for OPRF flow as yet");
        };

        let histogram = compute_histogram_of_users_with_row_count(&input);
        let ref_to_histogram = &histogram;

        // TODO: Compute OPRFs and shuffle and add dummies and stuff (Daniel's code will be called here)
        let sharded_input = input
            .into_iter()
            .map(|single_row| {
                let is_trigger_bit_share = if single_row.event_type == EventType::Trigger {
                    Replicated::share_known_value(&ctx, Boolean::ONE)
                } else {
                    Replicated::share_known_value(&ctx, Boolean::ZERO)
                };
                PrfShardedIpaInputRow {
                    prf_of_match_key: single_row.mk_oprf,
                    is_trigger_bit: is_trigger_bit_share,
                    breakdown_key: single_row.breakdown_key,
                    trigger_value: single_row.trigger_value,
                    timestamp: single_row.timestamp,
                }
            })
            .collect::<Vec<_>>();
        // Until then, we convert the output to something next function is happy about.
        match config.per_user_credit_cap {
            8 => attribute_cap_aggregate::<
                C,
                BA8,  // BreakdownKey,
                BA3,  // TriggerValue,
                BA20, // Timestamp,
                BA3,  // Saturating Sum
                Replicated<F>,
                F,
            >(
                ctx,
                sharded_input,
                config.attribution_window_seconds,
                ref_to_histogram,
            )
            .await,
            16 => attribute_cap_aggregate::<
                C,
                BA8,  // BreakdownKey,
                BA3,  // TriggerValue,
                BA20, // Timestamp,
                BA4,  // Saturating Sum
                Replicated<F>,
                F,
            >(
                ctx,
                sharded_input,
                config.attribution_window_seconds,
                ref_to_histogram,
            )
            .await,
            32 => attribute_cap_aggregate::<
                C,
                BA8,  // BreakdownKey,
                BA3,  // TriggerValue,
                BA20, // Timestamp,
                BA5,  // Saturating Sum
                Replicated<F>,
                F,
            >(
                ctx,
                sharded_input,
                config.attribution_window_seconds,
                ref_to_histogram,
            )
            .await,
            64 => attribute_cap_aggregate::<
                C,
                BA8,  // BreakdownKey,
                BA3,  // TriggerValue,
                BA20, // Timestamp,
                BA6,  // Saturating Sum
                Replicated<F>,
                F,
            >(
                ctx,
                sharded_input,
                config.attribution_window_seconds,
                ref_to_histogram,
            )
            .await,
            128 => attribute_cap_aggregate::<
                C,
                BA8,  // BreakdownKey,
                BA3,  // TriggerValue,
                BA20, // Timestamp,
                BA7,  // Saturating Sum
                Replicated<F>,
                F,
            >(
                ctx,
                sharded_input,
                config.attribution_window_seconds,
                ref_to_histogram,
            )
            .await,
            _ => panic!(
                "Invalid value specified for per-user cap: {:?}. Must be one of 8, 16, 32, 64, or 128.",
                config.per_user_credit_cap
            ),
        }
    }
}
