mod database;
mod routes;
mod tracing_helper;

mod local {
    use std::env::args;

    use lambda_http::{
        http::{request::Builder, Method},
        Body, IntoResponse, Request,
    };
    use tracing::trace;

    use crate::{database, routes::routes, tracing_helper};

    async fn func(req: Request) -> Result<impl IntoResponse, anyhow::Error> {
        let db = database::File;
        routes(&req, &db).await
    }

    #[allow(unused)]
    pub async fn main() -> anyhow::Result<()> {
        tracing_helper::init_local_tracing();

        let mut args = args();
        let method = args.nth(1).unwrap();
        let uri = args.next().unwrap();
        let body = args.next().unwrap();

        let req: Request = Builder::new()
            .method(Method::from_bytes(method.as_bytes()).unwrap())
            .uri(uri)
            .body(Body::Text(body))
            .unwrap();
        let res = func(req).await?;
        trace!("{:?}", res.into_response().await);
        Ok(())
    }
}

mod lambda {
    use lambda_http::{service_fn, IntoResponse, Request};

    use crate::{database, routes::routes, tracing_helper};

    async fn func(req: Request) -> Result<impl IntoResponse, anyhow::Error> {
        let db = database::DynamoDB::new().await;
        routes(&req, &db).await
    }

    #[allow(unused)]
    pub async fn main() -> Result<(), lambda_http::Error> {
        tracing_helper::init_server_tracing();

        lambda_http::run(service_fn(func)).await
    }
}

#[cfg(not(target_os = "linux"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    local::main().await
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::main().await
}
