use std::ops::ControlFlow;
use std::sync::Arc;

use lambda_http::{run, service_fn, Body, Request, Response};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;

use isracard_payment::bot::{self, State};
use isracard_payment::{load_config, load_isracard_client};

type BotError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    pretty_env_logger::init();
    log::info!("Initializing Isracard payment bot (Lambda)...");

    let bot = Bot::from_env();
    let me = bot.get_me().await.expect("Failed to get bot info");

    let handler = Arc::new(bot::schema());

    let mut deps = DependencyMap::new();
    deps.insert(bot);
    deps.insert(InMemStorage::<State>::new());
    deps.insert(load_isracard_client());
    deps.insert(load_config());
    deps.insert(me);

    let deps = Arc::new(deps);

    run(service_fn(move |event: Request| {
        let deps = deps.clone();
        let handler = handler.clone();
        async move { handle_webhook(&deps, &handler, event).await }
    }))
    .await
}

async fn handle_webhook(
    deps: &Arc<DependencyMap>,
    handler: &UpdateHandler<BotError>,
    event: Request,
) -> Result<Response<Body>, lambda_http::Error> {
    let body = match event.body() {
        Body::Text(text) => text.clone(),
        Body::Binary(bin) => String::from_utf8_lossy(bin).to_string(),
        Body::Empty => {
            return Ok(Response::builder()
                .status(200)
                .body(Body::Text("OK".to_string()))?);
        }
        _ => {
            return Ok(Response::builder()
                .status(200)
                .body(Body::Text("OK".to_string()))?);
        }
    };

    let update: Update = match serde_json::from_str(&body) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Failed to parse update: {e}");
            return Ok(Response::builder()
                .status(200)
                .body(Body::Text("OK".to_string()))?);
        }
    };

    log::info!("Update received: {:?}", update.kind);

    let mut dep_map = (*deps).as_ref().clone();
    dep_map.insert(update);

    match handler.dispatch(dep_map).await {
        ControlFlow::Break(Ok(())) => {}
        ControlFlow::Break(Err(err)) => {
            log::error!("Handler error: {:?}", err);
        }
        ControlFlow::Continue(_) => {
            log::warn!("Update was not handled");
        }
    }

    Ok(Response::builder()
        .status(200)
        .body(Body::Text("OK".to_string()))?)
}
