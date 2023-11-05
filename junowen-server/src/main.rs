mod tracing_helper;

async fn app() -> anyhow::Result<()> {
    tracing::info!("hello world");

    Ok(())
}

mod local {
    use anyhow::Result;

    use crate::tracing_helper;

    #[allow(unused)]
    pub async fn main() -> Result<()> {
        tracing_helper::init_local_tracing();

        // app(database::File).await
        crate::app().await
    }
}

mod lambda {
    use lambda_runtime::{service_fn, LambdaEvent};
    use serde_json::Value;

    use crate::tracing_helper;

    pub async fn func(_event: LambdaEvent<Value>) -> Result<(), lambda_runtime::Error> {
        // if let Err(err) = app(database::DynamoDB::new().await).await {
        //     tracing::error!("{:?}", err);
        //     return Err(err.into());
        // }
        crate::app().await.map_err(|err| err.into())
    }

    #[allow(unused)]
    pub async fn main() -> Result<(), lambda_runtime::Error> {
        tracing_helper::init_server_tracing();

        let func = service_fn(func);
        lambda_runtime::run(func).await
    }
}

#[cfg(not(target_os = "linux"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    local::main().await
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda::main().await
}
