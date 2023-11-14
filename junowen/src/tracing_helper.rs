use std::{num::NonZeroU8, panic};

use time::format_description::well_known::{iso8601, Iso8601};
use tracing::{error, Level};
use tracing_subscriber::{
    fmt::{time::LocalTime, writer::MakeWriterExt},
    prelude::__tracing_subscriber_SubscriberExt,
    EnvFilter, Layer,
};

const MY_CONFIG: iso8601::EncodedConfig = iso8601::Config::DEFAULT
    .set_time_precision(iso8601::TimePrecision::Second {
        decimal_digits: NonZeroU8::new(3),
    })
    .encode();

pub fn init_tracing(dir: &str, file_name: &str, ansi: bool) {
    let default_layer = || {
        const WITH_FILE_PATH: bool = cfg!(debug_assertions);
        tracing_subscriber::fmt::layer()
            .compact()
            .with_file(WITH_FILE_PATH)
            .with_line_number(WITH_FILE_PATH)
            .with_target(!WITH_FILE_PATH)
            .with_thread_ids(true)
            .with_timer(LocalTime::new(Iso8601::<MY_CONFIG>))
    };
    let writer = tracing_appender::rolling::never(dir, file_name);
    let writer = writer.with_max_level(Level::WARN);

    let layer = default_layer().with_ansi(false).with_writer(writer);

    if cfg!(debug_assertions) {
        let make_filter = || EnvFilter::new(concat!(env!("CARGO_CRATE_NAME"), "=trace"));
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(
                layer
                    .with_filter(make_filter())
                    .and_then(default_layer().with_ansi(ansi).with_filter(make_filter())),
            ),
        )
        .unwrap();
    } else {
        let make_filter = || EnvFilter::new(concat!(env!("CARGO_CRATE_NAME"), "=info"));
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(layer.with_filter(make_filter())),
        )
        .unwrap();
    }

    panic::set_hook(Box::new(|panic| error!("{}", panic)));
}
